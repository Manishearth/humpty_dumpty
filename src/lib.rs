#![feature(plugin_registrar, quote, plugin, box_syntax, rustc_private)]

#![allow(missing_copy_implementations, unused)]

#![plugin(syntax)]
#![plugin(rustc)]

#[macro_use]
extern crate syntax;
#[macro_use]
extern crate rustc;

use rustc::lint::{Context, LintPassObject, LintArray, LintPass, Level};
use rustc::plugin::Registry;
use rustc::metadata::csearch;

use syntax::ast::*;
use syntax::ast_map;
use syntax::ast_util::is_local;
use syntax::attr::{AttrMetaMethods};
use rustc::middle::ty::{self, ctxt};
use rustc::util::nodemap::{FnvHashMap, NodeMap};
use rustc::middle::def::*;
use syntax::visit::{self, Visitor};
use syntax::codemap::Span;

use std::collections::HashMap;
use std::borrow::Cow;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_lint_pass(box HumptyPass as LintPassObject);
}


declare_lint!(DROP_VIOLATION, Deny,
              "Violations of the drop contract");

/// Prefer str.to_owned() over str.to_string()
///
/// The latter creates a `Formatter` and is 5x slower than the former
pub struct HumptyPass;

impl LintPass for HumptyPass {
    fn get_lints(&self) -> LintArray {
        lint_array!(DROP_VIOLATION)
    }

    fn check_fn(&mut self, cx: &Context, _: visit::FnKind, _: &FnDecl, block: &Block, _: Span, _: NodeId) {
        // TODO error if FnDecl contains protected types, unless there's
        // the right attribute on this
        let mut visitor = DropVisitor::new(cx, block.id);
        visitor.visit_block(block);
    }
}

struct DropVisitor<'a : 'b, 'tcx : 'a, 'b> {
    // Type context, with all the goodies
    cx: &'b Context<'a, 'tcx>,
    // No need to store the span, we can
    // do a lookup on the Map if we wish
    map: NodeMap<(NodeId, Span)>,
    // The current block id
    current_block: NodeId,
}


impl<'a, 'tcx, 'b> DropVisitor<'a, 'tcx, 'b> {
    fn new(cx: &'b Context<'a, 'tcx>, block: NodeId) -> DropVisitor<'a, 'tcx, 'b> {
        DropVisitor {
            cx: cx,
            map: FnvHashMap(),
            current_block: block,
        }
    }

    fn walk_pat_and_add(&mut self, pat: &Pat, block: NodeId) {
        let ty = ty::pat_ty(self.cx.tcx, pat);
        let mut protected = 0u8;
        ty::walk_ty(ty, |t| {
            match t.sty {
                ty::ty_enum(did, _) | ty::ty_struct(did, _) => {
                    if ty::has_attr(self.cx.tcx, did, "drop_protection") {
                        protected += 1;
                        return;
                    }
                }
                _ => ()
            }
        });
        if protected == 1 {
            self.map.insert(pat.id, (block, pat.span));
        } else if protected == 2 {
            self.cx.span_lint(DROP_VIOLATION, pat.span, "This pattern contains multiple \
                                                         drop-protected types, please split it up \
                                                         somehow. We only support destructuring of one \
                                                         drop-protected type at a time")
        }
    }
}

impl<'a, 'b, 'tcx, 'v> Visitor<'v> for DropVisitor<'a, 'tcx, 'b> {
    fn visit_decl(&mut self, d: &'v Decl) {
        if let DeclLocal(ref l) = d.node {
            if l.source == LocalFor {
                // We don't handle for bindings yet
                // since they are tied to a different block
                // Delegate to the usual walker
                visit::walk_decl(self, d);
                return;
            }
            // First walk down the initializer
            if let Some(ref ex) = l.init {
                self.visit_expr(&*ex);
            }
            // grumble grumble Copy grumble
            let block = self.current_block;
            self.walk_pat_and_add(&*l.pat, block);
        } else {
            // Walk normally
            visit::walk_decl(self, d);
        }
    }

    fn visit_expr(&mut self, ex: &'v Expr) {
        match ex.node {
            // Might need to be ExprPath(None, _)
            // to work on current nightly

            ExprPath(_, _) => {
                let def = self.cx.tcx.def_map.borrow().get(&ex.id).map(|&v| v);
                if let Some(PathResolution { base_def: DefLocal(id),  .. }) = def {
                    if is_protected(self.cx.tcx, ex) {
                        let decl = self.map[id];
                        // TODO use proper lint erroring
                        self.cx.tcx.sess.span_warn(ex.span, "found usage of variable of protected type");
                        self.cx.tcx.sess.span_note(decl.1, "declaration here");
                    }
                }
            }

            ExprCall(ref name, ref params) => {
                match name.node {
                    ExprPath(_, _) => {
                        let def = self.cx.tcx.def_map.borrow().get(&name.id).map(|&x| x);
                        if let Some(PathResolution { base_def: DefFn(id, _), .. }) = def {
                            if ty::has_attr(self.cx.tcx, id, "allowed_on_protected") {
                                for param in params {
                                    if let ExprPath(_, _) = param.node {
                                        // It's an ident within an allowed method call,
                                        // it's fine!
                                        self.cx.tcx.sess.span_note(ex.span, "Allowed usage of type. Carry on!")
                                    } else {
                                        // It could be a situation like
                                        // allowed_fn(foo, bar, {foo(); bar(); unsafe_drop_fn(protected); baz()})
                                        // this ensures that no trickery happens
                                        self.visit_expr(&*param)
                                    }
                                }
                            } else if ty::has_attr(self.cx.tcx, id, "allowed_drop") {
                                for param in params {
                                    if let ExprPath(_, _) = param.node {
                                        // It's an ident within an allowed drop call,
                                        // we should remove it from the map if it was there
                                        let def = self.cx.tcx.def_map.borrow().get(&param.id).map(|&x| x);
                                        if let Some(PathResolution { base_def: DefLocal(id), .. }) = def {
                                            self.cx.tcx.sess.span_note(ex.span, "Properly dropped!");
                                            self.map.remove(&id);
                                        }
                                    } else {
                                        self.visit_expr(&*param)
                                    }
                                }
                            } else {
                                visit::walk_expr(self, ex)
                            }
                        } else {
                            visit::walk_expr(self, ex)
                        }
                    }
                    _ => {
                        // Technically we could have a function producing function
                        // or something here, that may produce safe functions
                        // We can probably write sophisticated checks for that,
                        // but we don't need to really.
                        visit::walk_expr(self, ex)
                    }
                }
            }
            ExprMethodCall(ref spanid, ref tys, ref params) => {
                let method_call = ty::MethodCall::expr(ex.id);
                let method_map = self.cx.tcx.method_map.borrow();
                let method = method_map.get(&method_call);
                if let Some(method_callee) = method {
                    if let ty::MethodStatic(id) = method_callee.origin {
                        let attrs = get_attrs_opt(self.cx.tcx, id);
                        if let Some(v) = attrs {
                            if v.iter().any(|item| item.check_name("allowed_on_protected")) {
                                for param in params {
                                    if let ExprPath(_, _) = param.node {
                                        // It's an ident within an allowed method call,
                                        // it's fine!
                                        self.cx.tcx.sess.span_note(ex.span, "Allowed usage of type. Carry on!");
                                        return;
                                    } else {
                                        // It could be a situation like
                                        // allowed_fn(foo, bar, {foo(); bar(); unsafe_drop_fn(protected); baz()})
                                        // this ensures that no trickery happens
                                        self.visit_expr(&*param);
                                    }
                                }
                            } else if v.iter().any(|item| item.check_name("allowed_drop")) {
                                for param in params {
                                    if let ExprPath(_, _) = param.node {
                                        // It's an ident within an allowed drop call,
                                        // we should remove it from the map if it was there
                                        let def = self.cx.tcx.def_map.borrow().get(&param.id).map(|&x| x);
                                        if let Some(PathResolution { base_def: DefLocal(id), .. }) = def {
                                            self.cx.tcx.sess.span_note(ex.span, "Properly dropped!");
                                            self.map.remove(&id);
                                            return;
                                        }
                                    } else {
                                        self.visit_expr(&*param)
                                    }
                                }
                            }
                        }
                    }
                }
                // Otherwise walk the expression
                visit::walk_expr(self, ex);
            },
            _ => {
                // We'll need to handle special casing
                // for match/for, and later any other type
                // of branch (if)
                visit::walk_expr(self, ex);
            }
        }
    }

    fn visit_block(&mut self, b: &'v Block) {
        let old_id = self.current_block;
        self.current_block = b.id;
        visit::walk_block(self, b);
        for (_, &(block, sp)) in self.map.iter() {
            if self.current_block == block {
                self.cx.span_lint(DROP_VIOLATION, b.span, "Drop-protected variable was implicitly dropped at the end of this block");
                if self.cx.current_level(DROP_VIOLATION) != Level::Allow {
                    self.cx.tcx.sess.span_note(sp, "Variable declared here")
                }
            }
        }
        self.current_block = old_id;
    }

}

fn is_protected<'tcx>(cx: &ctxt<'tcx>, expr: &Expr) -> bool {
    let ty = ty::expr_ty(cx, expr);
    let mut protected = false;
    ty::walk_ty(ty, |t| {
        match t.sty {
            ty::ty_enum(did, _) | ty::ty_struct(did, _) => {
                if ty::has_attr(cx, did, "drop_protection") {
                    protected = true;
                    return;
                }
            }
            _ => ()
        }
    });
    protected
}


// Copied from
// https://github.com/rust-lang/rust/pull/22348/files#diff-d6c1a78a970f7c6174f36c924862b879R813
// Thanks to @huon
// Get the attributes of a definition, returning `None` if `did`
// refers to nothing or something that cannot have attributes.
pub fn get_attrs_opt<'tcx>(tcx: &'tcx ctxt, did: DefId)
                           -> Option<Cow<'tcx, [Attribute]>>
{
    let attrs: Cow<'tcx, [Attribute]> = if is_local(did) {
        match tcx.map.find(did.node) {
            Some(ast_map::NodeItem(item)) => Cow::Borrowed(&item.attrs),
            Some(ast_map::NodeForeignItem(item)) => Cow::Borrowed(&item.attrs),
            Some(ast_map::NodeTraitItem(item)) => match *item {
                RequiredMethod(ref ty_meth) => Cow::Borrowed(&ty_meth.attrs),
                ProvidedMethod(ref meth) => Cow::Borrowed(&meth.attrs),
                TypeTraitItem(ref ty) => Cow::Borrowed(&ty.attrs),
            },
            Some(ast_map::NodeImplItem(item)) => match *item {
                MethodImplItem(ref meth) => Cow::Borrowed(&meth.attrs),
                TypeImplItem(ref ty) => Cow::Borrowed(&ty.attrs),
            },
            Some(ast_map::NodeVariant(variant)) => Cow::Borrowed(&variant.node.attrs),
            _ => return None
        }
    } else {
        Cow::Owned(csearch::get_item_attrs(&tcx.sess.cstore, did))
    };
    Some(attrs)
}
