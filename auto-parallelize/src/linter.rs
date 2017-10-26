use rustc::lint::{LintArray, LintPass, LateContext, LateLintPass};
use rustc::hir;
use rustc::hir::intravisit::FnKind;
use syntax::ast;
use syntax_pos::Span;

use AutoParallelise;

impl LintPass for AutoParallelise {
    fn get_lints(&self) -> LintArray {
        lint_array!()
    }
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for AutoParallelise {
    fn check_body(&mut self, _: &LateContext, _: &'tcx hir::Body) { self.save(); }
    fn check_body_post(&mut self, _: &LateContext, _: &'tcx hir::Body) { }
    fn check_name(&mut self, _: &LateContext, _: Span, _: ast::Name) { }
    fn check_crate(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Crate) { }
    fn check_crate_post(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Crate) { }
    fn check_mod(&mut self,
                 _: &LateContext<'a, 'tcx>,
                 _: &'tcx hir::Mod,
                 _: Span,
                 _: ast::NodeId) { }
    fn check_mod_post(&mut self,
                      _: &LateContext<'a, 'tcx>,
                      _: &'tcx hir::Mod,
                      _: Span,
                      _: ast::NodeId) { }
    fn check_foreign_item(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::ForeignItem) { }
    fn check_foreign_item_post(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::ForeignItem) { }
    fn check_item(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Item) { }
    fn check_item_post(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Item) { }
    fn check_local(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Local) { }
    fn check_block(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Block) { }
    fn check_block_post(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Block) { }
    fn check_stmt(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Stmt) { }
    fn check_arm(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Arm) { }
    fn check_pat(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Pat) { }
    fn check_decl(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Decl) { }
    fn check_expr(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Expr) { }
    fn check_expr_post(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Expr) { }
    fn check_ty(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Ty) { }
    fn check_generics(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Generics) { }
    fn check_fn(&mut self,
                _: &LateContext<'a, 'tcx>,
                _: FnKind<'tcx>,
                _: &'tcx hir::FnDecl,
                _: &'tcx hir::Body,
                _: Span,
                _: ast::NodeId) { }
    fn check_fn_post(&mut self,
                     _: &LateContext<'a, 'tcx>,
                     _: FnKind<'tcx>,
                     _: &'tcx hir::FnDecl,
                     _: &'tcx hir::Body,
                     _: Span,
                     _: ast::NodeId) { }
    fn check_trait_item(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::TraitItem) { }
    fn check_trait_item_post(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::TraitItem) { }
    fn check_impl_item(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::ImplItem) { }
    fn check_impl_item_post(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::ImplItem) { }
    fn check_struct_def(&mut self,
                        _: &LateContext<'a, 'tcx>,
                        _: &'tcx hir::VariantData,
                        _: ast::Name,
                        _: &'tcx hir::Generics,
                        _: ast::NodeId) { }
    fn check_struct_def_post(&mut self,
                             _: &LateContext<'a, 'tcx>,
                             _: &'tcx hir::VariantData,
                             _: ast::Name,
                             _: &'tcx hir::Generics,
                             _: ast::NodeId) { }
    fn check_struct_field(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::StructField) { }
    fn check_variant(&mut self,
                     _: &LateContext<'a, 'tcx>,
                     _: &'tcx hir::Variant,
                     _: &'tcx hir::Generics) { }
    fn check_variant_post(&mut self,
                          _: &LateContext<'a, 'tcx>,
                          _: &'tcx hir::Variant,
                          _: &'tcx hir::Generics) { }
    fn check_lifetime(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Lifetime) { }
    fn check_lifetime_def(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::LifetimeDef) { }
    fn check_path(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx hir::Path, _: ast::NodeId) { }
    fn check_attribute(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx ast::Attribute) { }

    /// Called when entering a syntax node that can have lint attributes such
    /// as `#[allow(...)]`. Called with *all* the attributes of that node.
    fn enter_lint_attrs(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx [ast::Attribute]) { }

    /// Counterpart to `enter_lint_attrs`.
    fn exit_lint_attrs(&mut self, _: &LateContext<'a, 'tcx>, _: &'tcx [ast::Attribute]) { }
}
