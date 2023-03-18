use super::item::*;
use super::project::*;
use cranelift_isle::ast::*;

impl Project {
    pub(crate) fn visit(&self, provider: impl AstProvider, handler: &mut dyn ItemOrAccessHandler) {
        provider.with_pragma(|_| {
            // Nothing here,
        });

        // handle type first.
        provider.with_type(|x| {
            let item = ItemOrAccess::Item(Item::Type { ty: x.clone() });
            handler.handle_item_or_access(self, &item);
            self.globals.enter_item(x.name.0.clone(), item);
            if handler.finished() {
                return;
            }
        });

        // handle const
        provider.with_extern(|x| match x {
            Extern::Const { name, ty, pos: _ } => {
                self.visit_type_apply(ty, handler);
                if handler.finished() {
                    return;
                };
                let item = ItemOrAccess::Item(Item::Const {
                    name: name.clone(),
                    ty: ty.clone(),
                });
                handler.handle_item_or_access(self, &item);
                if handler.finished() {
                    return;
                };
                self.globals.enter_item(name.0.clone(), item);
            }
            _ => {}
        });

        // enter decl
        provider.with_decl(|d| {
            d.arg_tys
                .iter()
                .chain(&vec![d.ret_ty.clone()])
                .for_each(|x| {
                    self.visit_type_apply(x, handler);
                    if handler.finished() {
                        return;
                    }
                });
        });
        // fix decl type
        {
            provider.with_extern(|x| match x {
                Extern::Extractor {
                    term,
                    func: _,
                    pos: _,
                    infallible: _infallible,
                } => self.globals.fix_decl_type(&term.0, DeclKind::EXTRATOR),
                Extern::Constructor { term, func: _, pos: _ } => {
                    self.globals.fix_decl_type(&term.0, DeclKind::CONSTRUCTOR)
                }
                Extern::Const { name: _, ty: _, pos: _ } => {}
            });
            provider.with_rule(|x| {
                let x = get_patter_target(&x.pattern);
                if let Some(x) = x {
                    self.globals.fix_decl_type(x, DeclKind::CONSTRUCTOR);
                }
            });
            provider.with_extractor(|x| {
                self.globals.fix_decl_type(&x.term.0, DeclKind::EXTRATOR);
            });
        }
        // handle converter
        provider.with_converter(|x| {
            self.visit_type_apply(&x.inner_ty, handler);
            self.visit_type_apply(&x.outer_ty, handler);
        });
        //
        provider.with_extern(|x| match x {
            Extern::Extractor {
                term, func: _, pos: _, ..
            }
            | Extern::Constructor { term, func: _, pos: _ } => {
                let item = ItemOrAccess::Access(Access::DeclExtern {
                    access: term.clone(),
                    def: Box::new(
                        self.globals
                            .query_item(&term.0, |x| x.clone())
                            .unwrap_or_default(),
                    ),
                });
                handler.handle_item_or_access(self, &item);
            }
            Extern::Const { .. } => {}
        });

        provider.with_extractor(|ext| {
            self.globals.enter_scope(|| {
                //
                let decl = self
                    .globals
                    .query_item(&ext.term.0, |x| match x {
                        Item::Decl { .. } => Some(x.clone()),
                        _ => None,
                    })
                    .flatten();
                let decl = match decl {
                    Some(x) => x,
                    None => return,
                };
                match decl {
                    Item::Decl { decl, .. } => {
                        // enter all vars
                        if let Some(name) = ext.args.get(0) {
                            let ty = decl.ret_ty.0.clone();
                            let item = ItemOrAccess::Item(Item::Var {
                                name: name.clone(),
                                ty,
                            });
                            handler.handle_item_or_access(self, &item);
                            if handler.finished() {
                                return;
                            }
                            self.globals.enter_item(name.0.clone(), item);
                        }
                        //
                        self.apply_extractor(&ext.template);
                    }
                    _ => {
                        unreachable!()
                    }
                }
            })
        });
    }

    fn apply_extractor(&self, p: &Pattern) {
        match p {
            Pattern::Var { var: _, pos: _ } => todo!(),
            Pattern::BindPattern { var: _, subpat: _, pos: _ } => todo!(),
            Pattern::ConstInt { val: _, pos: _ } => todo!(),
            Pattern::ConstPrim { val: _, pos: _ } => todo!(),
            Pattern::Term { sym: _, args: _, pos: _ } => todo!(),
            Pattern::Wildcard { pos: _ } => todo!(),
            Pattern::And { subpats: _, pos: _ } => todo!(),
            Pattern::MacroArg { index: _, pos: _ } => todo!(),
        }
    }

    fn visit_type_apply(&self, ty: &Ident, handler: &mut dyn ItemOrAccessHandler) {
        let item = ItemOrAccess::Access(Access::AppleType {
            access: ty.clone(),
            def: Box::new(
                self.globals
                    .query_item(&ty.0, |x| x.clone())
                    .unwrap_or_default(),
            ),
        });
        handler.handle_item_or_access(self, &item);
    }
}
