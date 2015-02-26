#![feature(plugin_registrar, quote, plugin, box_syntax, rustc_private)]

#![allow(missing_copy_implementations, unused)]

#![plugin(syntax)]
#![plugin(rustc)]

#[macro_use]
extern crate syntax;
#[macro_use]
extern crate rustc;

use rustc::lint::{Context, LintPassObject, LintArray, LintPass};
use rustc::plugin::Registry;

use syntax::ast::*;
use rustc::middle::ty::{self, ctxt};
use rustc::util::nodemap::{FnvHashMap, NodeMap};
use rustc::middle::def::DefLocal;
use syntax::visit;
use syntax::codemap::Span;

use std::collections::HashMap;

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
        visit::walk_block(&mut visitor, block);
    }
}

struct DropVisitor<'a : 'b, 'tcx : 'a, 'b> {
    // Type context, with all the goodies
    cx: &'b Context<'a, 'tcx>,
    in_allowed: bool,
    in_dropper: bool,
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
            in_allowed: false,
            in_dropper: false,
            map: FnvHashMap(),
            current_block: block,
        }
    }
    // Given a declaration-y pattern, look for types that are
    // annotated accordingly and only store the pattern in that case
    // (for efficiency)
    fn walk_pat_and_add(&mut self, pat: &Pat, block: NodeId) {
        // This doesn't actually do what the comment above says...yet
        // Steps:
        // use ty::walk_pat
        // use ty::has_attr

        // Stub: just indiscriminately adds it
        self.map.insert(pat.id, (block, pat.span));
    }
}

impl<'a, 'b, 'tcx, 'v> visit::Visitor<'v> for DropVisitor<'a, 'tcx, 'b> {
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
            ExprPath(_) => {
                let def = self.cx.tcx.def_map.borrow().get(&ex.id).map(|&v| v);
                match def {
                    Some(DefLocal(id)) => {
                        if is_protected(self.cx.tcx, ex) {
                            let decl = self.map[id];
                            self.cx.tcx.sess.span_warn(ex.span, "found usage of variable of protected type");
                            self.cx.tcx.sess.span_note(decl.1, "declaration here");
                        }
                    }
                    // Not a local binding, so it's
                    // not of any interest
                    _ => {}
                }
            }
            _ => {
                // We'll need to handle special casing
                // for match/for, and later any other type
                // of branch (if)
                visit::walk_expr(self, ex);
            }
        }
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