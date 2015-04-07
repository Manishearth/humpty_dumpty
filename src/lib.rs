#![feature(plugin_registrar, quote, plugin, box_syntax, rustc_private, slice_patterns)]

#![allow(missing_copy_implementations, unused)]

#![plugin(syntax)]
#![plugin(rustc)]

#![crate_type="dylib"]

#[macro_use]
extern crate syntax;
#[macro_use]
extern crate rustc;
#[macro_use]
extern crate log;

use rustc::lint::{Context, LintPassObject, LintArray, LintPass, Level};
use rustc::plugin::Registry;
use rustc::metadata::csearch;

use syntax::ast::*;
use syntax::ast_map;
use syntax::ast_util;
use syntax::ast_util::is_local;
use syntax::attr::{AttrMetaMethods};
use rustc::middle::ty::{self, ctxt};
use rustc::util::ppaux::Repr;
use rustc::util::nodemap::{FnvHashMap, NodeMap};
use rustc::middle::def::*;
use syntax::visit::{self, Visitor};
use syntax::codemap::Span;

declare_lint!(TEST_LINT, Warn, "Warn about items named 'lintme'");

struct Pass;

impl LintPass for Pass {
    fn get_lints(&self) -> LintArray {
        lint_array!(TEST_LINT)
    }

    fn check_fn(&mut self, cx: &Context, _: visit::FnKind, decl: &FnDecl, block: &Block, span: Span, id: NodeId) {
        // Walk the arguments and add them to the map
        let attrs = cx.tcx.map.attrs(id);
        let mut visitor = MyVisitor::new(cx, block.id, attrs);
        for arg in decl.inputs.iter() {
            visitor.walk_pat_and_add(&arg.pat);
        }

        visit::walk_block(&mut visitor, block);

        if !visitor.diverging {
            for var in visitor.map.iter() {
                // TODO: prettify
                if !visitor.can_drop(var.0) {
                    cx.tcx.sess.span_err(*var.1, "dropped var");
                }
            }
        }
    }
}

#[derive(Clone)]
struct MyVisitor<'a : 'b, 'tcx : 'a, 'b> {
    // Type context, with all the goodies
    map: NodeMap<Span>, // (blockid and span for declaration)
    cx: &'b Context<'a, 'tcx>,
    diverging: bool,
    attrs: &'tcx [Attribute],
}

impl <'a, 'tcx, 'b> MyVisitor<'a, 'tcx, 'b> {
    fn new(cx: &'b Context<'a, 'tcx>, id: NodeId, attrs: &'tcx [Attribute]) -> Self {
        let map = FnvHashMap();
        let visitor = MyVisitor { cx: cx,
                                  map: map,
                                  diverging: false,
                                  attrs: attrs,
        };
        visitor
    }

    fn is_protected(&self, ty: ty::Ty<'tcx>) -> bool {
        match ty.sty {
            ty::sty::ty_enum(did, _) | ty::sty::ty_struct(did, _)
                if ty::has_attr(self.cx.tcx, did, "drop_protect") => true,
            _ => false,
        }
    }

    fn can_drop(&self, id: &NodeId) -> bool {
        let tcx = self.cx.tcx;
        let node_ty = ty::node_id_to_type(tcx, *id);
        for attr in self.attrs {
            if let MetaNameValue(ref intstr, ref lit) = attr.node.value.node {
                if *intstr == "allow_drop" {
                    if let LitStr(ref litstr, _) = lit.node {
                        if *litstr == &node_ty.repr(tcx)[..] {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn walk_pat_and_add(&mut self, pat: &Pat) {
        ast_util::walk_pat(pat, |p| {
            if let PatIdent(_, _, _) = p.node {
                let ty = ty::pat_ty(self.cx.tcx, p);
                let mut protected = false;
                ty::walk_ty(ty, |t| {
                    if self.is_protected(t) {
                        protected = true;
                    }
                });
                if protected {
                    self.cx.tcx.sess.span_note(p.span, &format!("Adding drop protected type to map. Id: {:?}", p.id));
                    self.map.insert(p.id, p.span);
                }
            } else if let PatWild(_) = p.node {
                let ty = ty::pat_ty(self.cx.tcx, p);
                let mut protected = false;
                ty::walk_ty(ty, |t| {
                    if self.is_protected(t) {
                        protected = true;
                    }
                });
                if protected && !self.can_drop(&pat.id) {
                    self.cx.tcx.sess.span_err(p.span, "Protected type is dropped");
                }
            }
            true
        });
    }
}

impl<'a, 'b, 'tcx, 'v> Visitor<'v> for MyVisitor<'a, 'tcx, 'b> {
    fn visit_decl(&mut self, d: &'v Decl) {
        // If d is local and if d.ty is protected:
        //  - First handle the initializer: We need to remove any used variables that are moved
        //  - Also, if the initializer is a reference, then what?
        //  - then add pat.id to self.map so we can track it going forward
        // We also need to handle if l.source is a LocalFor

        // self.cx.tcx.sess.span_note(d.span, &format!("decl: {:?}\n", d));
        if let DeclLocal(ref l) = d.node {
            debug!("decllocal: {:?}\n", ty::pat_ty(self.cx.tcx, &l.pat));
            if l.source == LocalFor {
                unimplemented!();
            }

            // Remove moved variables from map
            // Maybe it's a reference? Use maybe_walk_expr
            if let Some(ref ex) = l.init {
                self.visit_expr(&ex);
            }

            // Add the ids in the pat
            self.walk_pat_and_add(&l.pat);
            return;
        }
        visit::walk_decl(self, d);
    }

    fn visit_stmt(&mut self, s: &'v Stmt) {
        if !self.diverging {
            if let StmtSemi(ref e, id) = s.node {
                let ty = ty::expr_ty(self.cx.tcx, e);
                if self.is_protected(ty) {
                    self.cx.tcx.sess.span_err(s.span, "Return type is protected but unused");
                }
            }
            visit::walk_stmt(self, s);
        }
    }

    fn visit_expr(&mut self, e: &'v Expr) {
        // Visit and remove all consumed values
        // Which Exprs do we need to handle?
        // At least ExprCall and ExprMethodCall
        debug!("visit_expr: {:?}\n", e);
        if self.diverging {
            return              // Don't proceed
        }
        match e.node {
            ExprAssign(ref lhs, ref rhs) => {
                // Remove all protected vars in rhs
                self.visit_expr(&rhs);

                // Get the defid
                let defid = if let ExprPath(_, _) = lhs.node {
                    expr_to_deflocal(self.cx.tcx, lhs).unwrap()
                } else {
                    unimplemented!()
                };

                // Check that we're not overwriting something
                if self.map.contains_key(&defid) {
                    self.cx.tcx.sess.span_err(lhs.span, "cannot overwrite linear type");
                } else {
                    self.map.insert(defid, e.span);
                }
            }
            ExprPath(_, _) => {
                // If the path is a local id that's in our map and it is getting
                // moved, remove it from self.map. If we got this far, it is a
                // move
                if let Some(id) = expr_to_deflocal(self.cx.tcx, e) {
                    debug!("Trying to find id: {:?}\n", id);
                    if self.map.contains_key(&id) {
                        self.cx.tcx.sess.span_note(e.span, "Consuming protected var");
                        self.map.remove(&id).unwrap();
                    }
                }
                visit::walk_expr(self, e);
            }
            ExprCall(_, _) | ExprMethodCall(_, _, _) => {
                visit::walk_expr(self, e);
            }
            ExprAddrOf(_, ref e1) => {
                if let ExprPath(_, _) = e1.node {
                    // ignore
                } else {
                    // recurse on e1
                    self.visit_expr(&e1);
                }
            }
            ExprIf(ref e1, ref b, ref else_block) => {
                // Consume stuff in expr
                self.visit_expr(&e1);

                // For convenience, we first check the else block, and set the
                // outhm to test against:
                let outhm = if let &Some(ref e2) = else_block {
                    let mut v = self.clone();
                    v.visit_expr(e2);
                    if !v.diverging {
                        // The block was not diverging, so outhm should be the resulting map
                        Some(v.map)
                    } else {
                        // The result block was diverging, so the if block can return anything
                        None
                    }
                } else {
                    // There's no else-block, but an empty else block is the
                    // same, and thus the hash-map has to be the same as the one
                    // from the start
                    Some(self.map.clone())
                };

                let mut v = self.clone();
                v.visit_block(&b);

                // Update the outgoing hm.
                if v.diverging {
                    if let Some(map) = outhm {
                        // Hvis den første ikke divergede skal map sættes til outhm
                        self.map = map;
                    } else {
                        // Ellers diverger begge branches
                        self.diverging = true;
                    }
                } else {
                    match outhm {
                        Some(map) => {
                            if v.map == map {
                                // Hvis outhm er noget, og den er ens med v.map er alt godt
                                self.map = map;
                            } else {
                                // Ellers er der fejl
                                self.cx.tcx.sess.span_err(e.span, "Branch arms are not linear");
                            }
                        },
                        None => {
                            // Hvis outhm er none, er alt også godt
                            self.map = v.map;
                        },
                    }
                }
            }
            ExprMatch(ref e1, ref arms, ref source) => {
                // Consume stuff in e
                self.visit_expr(&e1);

                // If the match looks like this, we're in an expanded for loop:
                // match ::std::iter::IntoIterator::into_iter(&[1, 2, 3]) {
                //     mut iter =>
                //         loop  {
                //             match ::std::iter::Iterator::next(&mut iter) {   <- ForLoopDesugar
                //                 ::std::option::Option::Some(x) => { }
                //                 ::std::option::Option::None => break ,
                //             }
                //         },
                // }
                let mut is_for_loop = false;
                if let [Arm { ref body, .. }] = &arms[..] {
                    if let ExprLoop(ref loop_block, _) = body.node {
                        if let &Block { expr: Some(ref loop_expr), .. } = &**loop_block {
                            if let ExprMatch(_, _, MatchSource::ForLoopDesugar) = loop_expr.node {
                                self.cx.tcx.sess.span_note(e.span, "Desugar");
                                is_for_loop = true;
                                // Skip pattern in outermost arm, just visit the body
                                // TODO: Guards
                                self.visit_expr(body);
                            }
                        }
                    }
                }

                if !is_for_loop {
                    // Walk each of the arms, and check that outcoming hms are
                    // identical
                    let mut old: Option<Self> = None;
                    for arm in arms {
                        let mut v = self.clone();
                        v.visit_arm(&arm);
                        if !v.diverging {
                            if let Some(tmp) = old {
                                if tmp.map != v.map {
                                    self.cx.tcx.sess.span_err(e.span, "Match arms are not linear");
                                }
                            }
                            old = Some(v);
                        }
                    }
                    if let Some(new) = old {
                        self.map = new.map
                    } else {
                        // Everything is diverging?
                        self.diverging = true;
                    }
                }
            }
            ExprRet(ref e1) => {
                // If there is a return value, consume it
                if let &Some(ref ret) = e1 {
                    self.visit_expr(ret);
                }

                // Check that the hm is empty
                for var in self.map.iter() {
                    // TODO: prettify
                    if !self.can_drop(var.0) {
                        self.cx.tcx.sess.span_err(*var.1, "dropped var");
                    }
                }

                // Set the flag, indicating that we've returned
                self.diverging = true;
            }
            // todo: We need to do something about while loops, breaks, returns.
            _ => visit::walk_expr(self, e),
        }
    }

    fn visit_arm(&mut self, a: &'v Arm) {
        // Add patterns
        for pat in a.pats.iter() {
            self.walk_pat_and_add(&pat);
        }

        // TODO: What about guards
        if let Some(_) = a.guard {
            unimplemented!();
        }

        // Consume stuff in body
        visit::walk_expr(self, &a.body);
    }

    fn visit_block(&mut self, b: &'v Block) {
        debug!("visit_block: stmts: {:?}\n", b.stmts);
        visit::walk_block(self, b);

        if !self.diverging {
            if let Some(ref e) = b.expr {
                debug!("visit_block: expr is {:?}\n", e);
                let ty = ty::expr_ty(self.cx.tcx, e);
                if self.is_protected(ty) {
                    // This value is returned, and thus we can consume it
                    visit::walk_expr(self, e);
                }
            }
        }
    }
}

fn expr_to_deflocal<'tcx>(tcx: &'tcx ctxt, expr: &Expr) -> Option<NodeId> {
    let def = tcx.def_map.borrow().get(&expr.id).map(|&v| v);
    if let Some(PathResolution { base_def: DefLocal(id), .. }) = def {
        Some(id)
    } else {
        None
    }
}


#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_lint_pass(box Pass);
}
