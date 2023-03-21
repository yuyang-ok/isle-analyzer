use super::item::*;
use super::project::*;
use cranelift_isle::ast::*;

impl Project {
    pub(crate) fn visit(&self, provider: impl AstProvider, handler: &mut dyn ItemOrAccessHandler) {
        provider.with_pragma(|_| {
            // Nothing here.
        });
        // handle type first.
        provider.with_type(|x| {
            let item = ItemOrAccess::Item(Item::Type { ty: x.clone() });
            handler.handle_item_or_access(self, &item);
            self.context.enter_item(x.name.0.clone(), item);
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
                self.context.enter_item(name.0.clone(), item);
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
            let item = ItemOrAccess::Item(Item::Decl {
                decl: d.clone(),
                kind: DeclKind::default(),
            });
            handler.handle_item_or_access(self, &item);
            if handler.finished() {
                return;
            }
            self.context.enter_item(d.term.0.clone(), item);
        });
        // fix decl type
        {
            provider.with_extern(|x| match x {
                Extern::Extractor {
                    term,
                    func: _,
                    pos: _,
                    infallible: _infallible,
                } => self.context.fix_decl_type(&term.0, DeclKind::EXTRATOR),
                Extern::Constructor {
                    term,
                    func: _,
                    pos: _,
                } => self.context.fix_decl_type(&term.0, DeclKind::CONSTRUCTOR),
                Extern::Const {
                    name: _,
                    ty: _,
                    pos: _,
                } => {}
            });
            provider.with_rule(|x| {
                let name_and_pos = get_rule_target(&x.pattern);
                if let Some((name, pos)) = name_and_pos {
                    self.context.fix_decl_type(name, DeclKind::CONSTRUCTOR);
                }
            });
            provider.with_extractor(|x| {
                self.context.fix_decl_type(&x.term.0, DeclKind::EXTRATOR);
            });
        }

        // handle converter
        provider.with_converter(|x| {
            self.visit_type_apply(&x.inner_ty, handler);
            self.visit_type_apply(&x.outer_ty, handler);
        });
        // handle extern
        provider.with_extern(|x| match x {
            Extern::Extractor {
                term,
                func: _,
                pos: _,
                ..
            }
            | Extern::Constructor {
                term,
                func: _,
                pos: _,
            } => {
                let item = ItemOrAccess::Access(Access::DeclExtern {
                    access: term.clone(),
                    def: self
                        .context
                        .query_item(&term.0, |x| x.clone())
                        .unwrap_or_default(),
                });
                handler.handle_item_or_access(self, &item);
            }
            Extern::Const { .. } => {}
        });
        //
        provider.with_extractor(|ext| {
            self.context.enter_scope(|| {
                let decl = self
                    .context
                    .query_item(&ext.term.0, |x| match x {
                        Item::Decl { .. } => Some(x.clone()),
                        _ => None,
                    })
                    .flatten()
                    .unwrap_or_default();
                let item = ItemOrAccess::Access(Access::ImplExtractor {
                    access: ext.term.clone(),
                    def: decl.clone(),
                });
                handler.handle_item_or_access(self, &item);
                if handler.finished() {
                    return;
                }
                match decl {
                    Item::Decl { decl, .. } => {
                        // enter all vars
                        self.context.enter_scope(|| {
                            for (index, name) in ext.args.iter().enumerate() {
                                let ty = decl
                                    .arg_tys
                                    .get(index)
                                    .map(|x| x.0.clone())
                                    .unwrap_or("".to_string());
                                let item = ItemOrAccess::Item(Item::Var {
                                    name: name.clone(),
                                    ty: ty,
                                });
                                handler.handle_item_or_access(self, &item);
                                self.context.enter_item(name.0.clone(), item)
                            }
                            // 1
                            self.apply_extractor(&ext.template, handler);
                        });
                    }
                    _ => {
                        unreachable!()
                    }
                }
            })
        });
    }

    fn apply_extractor(&self, p: &Pattern, handler: &mut dyn ItemOrAccessHandler) {
        match p {
            Pattern::Var { var, pos: _ } => {
                let item = ItemOrAccess::Access(Access::ExtractVar {
                    access: var.clone(),
                    def: self
                        .context
                        .query_item(&var.0, |x| x.clone())
                        .unwrap_or_default(),
                });
                handler.handle_item_or_access(self, &item);
                if handler.finished() {
                    return;
                }
            }
            Pattern::BindPattern { var, subpat, .. } => {
                let item = ItemOrAccess::Access(Access::ApplyExtractor {
                    access: var.clone(),
                    def: self
                        .context
                        .query_item(&var.0, |x| x.clone())
                        .unwrap_or_default(),
                });
                handler.handle_item_or_access(self, &item);
                if handler.finished() {
                    return;
                }
                self.apply_extractor(subpat.as_ref(), handler);
            }
            Pattern::ConstInt { .. } => {
                // ok
            }
            Pattern::ConstPrim { val, .. } => {
                let item = ItemOrAccess::Access(Access::ApplyConst {
                    access: val.clone(),
                    def: self
                        .context
                        .query_const(&val.0, |x| x.clone())
                        .unwrap_or_default(),
                });

                handler.handle_item_or_access(self, &item);
                if handler.finished() {
                    return;
                }
            }
            Pattern::Term { sym, args, pos: _ } => {
                eprintln!("$$$$$$$$$$$${:?}", sym);

                let item = ItemOrAccess::Access(Access::ApplyExtractor {
                    access: sym.clone(),
                    def: self
                        .context
                        .query_item(&sym.0, |x| x.clone())
                        .unwrap_or_default(),
                });
                handler.handle_item_or_access(self, &item);
                if handler.finished() {
                    return;
                }
                for a in args.iter() {
                    self.apply_extractor(a, handler);
                    if handler.finished() {
                        return;
                    }
                }
            }
            Pattern::Wildcard { pos: _ } => {
                // nothing here.
            }
            Pattern::And { subpats, pos: _ } => {
                for s in subpats.iter() {
                    self.apply_extractor(s, handler);
                    if handler.finished() {
                        return;
                    }
                }
            }
            Pattern::MacroArg { index: _, pos: _ } => {}
        }
    }

    fn visit_type_apply(&self, ty: &Ident, handler: &mut dyn ItemOrAccessHandler) {
        let item = ItemOrAccess::Access(Access::AppleType {
            access: ty.clone(),
            def: self
                .context
                .query_item(&ty.0, |x| x.clone())
                .unwrap_or_default(),
        });
        handler.handle_item_or_access(self, &item);
    }
}
