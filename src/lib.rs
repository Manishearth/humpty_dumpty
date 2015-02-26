#![feature(plugin_registrar, quote, plugin, box_syntax, rustc_private, core)]

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
use rustc::middle::ty::ctxt;
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
        let mut visitor = DropVisitor::new(&*cx.tcx, block.id);
        visit::walk_block(&mut visitor, block);
    }
}

struct DropVisitor<'a, 'tcx : 'a> {
    tcx: &'a ctxt<'tcx>,
    in_allowed: bool,
    in_dropper: bool,
    map: NodeMap<(NodeId, Span)>,
    current_block: NodeId,
    in_pat_block: Option<CurrentPat>,
}

#[derive(Copy)]
struct CurrentPat {
    id: NodeId,
    block: NodeId,
}

impl CurrentPat {
    fn new(id: NodeId, block: NodeId) -> CurrentPat {
        CurrentPat {
            id: id,
            block: block,
        }
    }
}
impl<'a, 'tcx> DropVisitor<'a, 'tcx> {
    fn new(tcx: &'a ctxt<'tcx>, block: NodeId) -> DropVisitor<'a, 'tcx> {
        DropVisitor {
            tcx: tcx,
            in_allowed: false,
            in_dropper: false,
            map: FnvHashMap(),
            current_block: block,
            in_pat_block: None
        }
    }
}

impl<'a, 'tcx, 'v> visit::Visitor<'v> for DropVisitor<'a, 'tcx> {
    fn visit_ident(&mut self, sp: Span, ident: Ident) {
        if let Some(curr) = self.in_pat_block {
            self.map.insert(curr.id, (curr.block, sp));
            self.tcx.sess.span_note(sp, format!("Found local being declared, \
                                                 with id {} and block {}",
                                                 curr.id, curr.block).as_slice());
        }
    }

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
            self.in_pat_block = Some(CurrentPat::new(l.pat.id, self.current_block));
            visit::walk_pat(self, &*l.pat);
            self.in_pat_block = None;
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
                let def = self.tcx.def_map.borrow().get(&ex.id).map(|&v| v);
                match def {
                    Some(DefLocal(id)) => {
                        let decl = self.map[id];
                        self.tcx.sess.span_warn(ex.span,
                                                format!("Found usage of local variable, \
                                                         declared at id {}, and block {}",
                                                         id, decl.0).as_slice());
                        self.tcx.sess.span_note(decl.1, "Declaration here");
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