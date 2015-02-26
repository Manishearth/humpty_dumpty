#![feature(plugin_registrar, quote, plugin, box_syntax, rustc_private)]

#![allow(missing_copy_implementations)]

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
    map: NodeMap<NodeId>,
    current_block: NodeId, 
}

impl<'a, 'tcx> DropVisitor<'a, 'tcx> {
    fn new(tcx: &'a ctxt<'tcx>, block: NodeId) -> DropVisitor<'a, 'tcx> {
        DropVisitor {
            tcx: tcx,
            in_allowed: false,
            in_dropper: false,
            map: FnvHashMap(),
            current_block: block
        }
    }
}

impl<'a, 'tcx, 'v> visit::Visitor<'v> for DropVisitor<'a, 'tcx> {
    fn visit_ident(&mut self, _: Span, ident: Ident) {
        println!("Encountered ident {}", ident.as_str())
    }
}