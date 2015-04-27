#![feature(plugin_registrar, quote, plugin, box_syntax, rustc_private,
           slice_patterns)]

#![plugin(syntax)]
#![plugin(rustc)]

#![crate_type="dylib"]

#[macro_use]
extern crate syntax;
#[macro_use]
extern crate rustc;
#[macro_use]
extern crate log;

use rustc::lint::{Context, LintArray, LintPass};
use rustc::plugin::Registry;

use syntax::ast::*;
use syntax::ast_util;
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

    fn check_fn(&mut self, cx: &Context, _: visit::FnKind, decl: &FnDecl, block: &Block, _: Span, id: NodeId) {
        // Walk the arguments and add them to the map
        let attrs = cx.tcx.map.attrs(id);
        let mut visitor = LinearVisitor::new(cx, block.id, attrs);
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
struct LinearVisitor<'a : 'b, 'tcx : 'a, 'b> {
    // Type context, with all the goodies
    map: NodeMap<Span>, // (blockid and span for declaration)
    cx: &'b Context<'a, 'tcx>,
    diverging: bool,
    attrs: &'tcx [Attribute],
    breaking: bool,
    loopin: Option<NodeMap<Span>>,
    loopout: Option<NodeMap<Span>>,
}

impl <'a, 'tcx, 'b> LinearVisitor<'a, 'tcx, 'b> {
    fn new(cx: &'b Context<'a, 'tcx>, _: NodeId, attrs: &'tcx [Attribute]) -> Self {
        let map = FnvHashMap();
        let visitor = LinearVisitor { cx: cx,
                                  map: map,
                                  diverging: false,
                                  attrs: attrs,
                                  breaking: false,
                                  loopin: None,
                                  loopout: None
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

    fn update_loopout(&mut self, e: &Expr, loopout: &Option<NodeMap<Span>>) {
        if self.loopout.is_some() {
            if &self.loopout != loopout {
                self.cx.tcx.sess.span_err(e.span, "Diverging loopout");
            }
        } else {
            self.loopout = loopout.clone();
        }
    }
}

impl<'a, 'b, 'tcx, 'v> Visitor<'v> for LinearVisitor<'a, 'tcx, 'b> {
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
            if let StmtSemi(ref e, _) = s.node {
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
        if self.diverging || self.breaking {
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

            ExprIf(_, ref if_block, ref else_expr) => {
                // Walk each of the arms, and check that outcoming hms are
                // identical
                let mut old: Option<LinearVisitor> = None;

                let mut v = self.clone();
                v.visit_block(&if_block);
                if !v.diverging {
                    self.update_loopout(e, &v.loopout);

                    if else_expr.is_none() {
                        if v.map != self.map && !v.breaking {
                            self.cx.tcx.sess.span_err(e.span, "If branch is not linear");
                        }
                        v.breaking = false;
                    }
                    old = Some(v);
                }

                if let &Some(ref else_expr) = else_expr {
                    let mut v = self.clone();
                    v.visit_expr(&else_expr);
                    if !v.diverging {
                        self.update_loopout(e, &v.loopout);
                        if let &Some(ref tmp) = &old {
                            if !tmp.breaking {
                                if !v.breaking && tmp.map != v.map {
                                    // neither branch is breaking, but their maps are unequal
                                    self.cx.tcx.sess.span_err(e.span, "If branches are not linear");
                                } else if v.breaking && tmp.map != self.map {
                                    // `else` is breaking and `if` map is not neutral
                                    self.cx.tcx.sess.span_err(e.span, "If branches are not linear");
                                }
                                // The resulting state is non-breaking
                                v.breaking = false;
                            }
                            // If tmp (the `if` branch) is breaking the whole if
                            // expr is breaking iff the `else` branch is
                            // breaking.
                        }
                        old = Some(v);
                    }
                }

                if let Some(old) = old {
                    self.map = old.map;
                    self.breaking = old.breaking;
                } else {
                    // Everything is diverging?
                    self.diverging = true;
                }
            }
            ExprMatch(ref e1, ref arms, _) => {
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
                                let mut tmp = self.clone();
                                tmp.visit_block(loop_block);
                                if !tmp.diverging {
                                    if let Some(hm) = tmp.loopout {
                                        if tmp.map != hm {
                                            self.cx.tcx.sess.span_err(e.span, "Non-linear for loop");
                                        } else {
                                            self.map = tmp.map;
                                        }
                                    }
                                } else {
                                    self.diverging = true;
                                }
                            }
                        }
                    }
                }

                if !is_for_loop {
                    // Walk each of the arms, and check that outcoming hms are
                    // identical
                    //
                    // TODO: Replace with Option<Self> once rust-lang/rust/issues/24227
                    // is fixed
                    let mut old: Option<LinearVisitor<'a, 'tcx, 'b>> = None;
                    for arm in arms {
                        let mut v = self.clone();
                        v.visit_arm(&arm);
                        if !v.diverging {
                            self.update_loopout(e, &v.loopout);
                            if let Some(tmp) = old {
                                if !tmp.breaking {
                                    v.breaking = false;
                                }
                                if tmp.map != v.map {
                                    self.cx.tcx.sess.span_err(e.span, "Match arms are not linear");
                                }
                            }
                            old = Some(v);
                        }
                    }

                    if let Some(old) = old {
                        self.map = old.map;
                        self.breaking = old.breaking;
                    } else {
                        // Everything is diverging
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
            ExprLoop(ref body, _) => {
                let mut tmp = self.clone();
                tmp.loopin = Some(self.map.clone());
                tmp.visit_block(body);
                if !tmp.diverging {
                    if let Some(outgoing) = tmp.loopout {
                        self.map = outgoing;
                    }
                } else {
                    self.diverging = true;
                }
            }
            ExprWhile(_, _, _) => {
                unimplemented!();
            }
            ExprBreak(label) => {
                if label.is_some() {
                    unimplemented!();
                }
                self.breaking = true;
                if let Some(ref outgoing) = self.loopout {
                    if &self.map == outgoing {
                        // All good
                    } else {
                        self.cx.tcx.sess.span_err(e.span, "Diverging break");
                    }
                } else {
                    self.loopout = Some(self.map.clone());
                }
            }
            ExprAgain(ref label) => {
                if label.is_some() {
                    unimplemented!();
                }
                self.breaking = true;
                if let Some(ref incoming) = self.loopin {
                    if &self.map == incoming {
                        // All good
                    } else {
                        self.cx.tcx.sess.span_err(e.span, "Diverging continue");
                    }
                } else {
                    unreachable!();
                }
            }
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
