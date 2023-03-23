use super::item::*;
use super::project::*;
use cranelift_isle::ast::*;

impl Project {
    pub(crate) fn visit(&self, provider: impl AstProvider, handler: &mut dyn ItemOrAccessHandler) {
        provider.with_pragma(|_| {
            // Nothing here.
        });
        // handle primitive type first.
        provider.with_type(|x| {
            match x.ty {
                TypeValue::Primitive(_, _) => {}
                TypeValue::Enum(_, _) => return,
            }
            let item = ItemOrAccess::Item(Item::Type { ty: x.clone() });
            handler.handle_item_or_access(self, &item);
            self.context.enter_item(x.name.0.clone(), item);
            if handler.finished() {
                return;
            }
        });

        // handle enum type after primitive has handled.
        provider.with_type(|x| {
            match x.ty {
                TypeValue::Primitive(_, _) => return,
                TypeValue::Enum(_, _) => {}
            }
            let item = ItemOrAccess::Item(Item::Type { ty: x.clone() });
            handler.handle_item_or_access(self, &item);
            self.context.enter_item(x.name.0.clone(), item);
            if handler.finished() {
                return;
            }
            match &x.ty {
                TypeValue::Primitive(_, _) => {}
                TypeValue::Enum(variants, _) => {
                    for v in variants.iter() {
                        let item = ItemOrAccess::Item(Item::EnumMemberName {
                            name: v.name.clone(),
                        });
                        handler.handle_item_or_access(self, &item);
                        if handler.finished() {
                            return;
                        }
                        for f in v.fields.iter() {
                            let item = ItemOrAccess::Item(Item::EnumMemberField {
                                name: f.name.clone(),
                            });
                            handler.handle_item_or_access(self, &item);
                            if handler.finished() {
                                return;
                            }
                            self.visit_type_apply(&f.ty, handler);
                        }
                    }
                }
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
            for x in d.arg_tys.iter().chain(&vec![d.ret_ty.clone()]) {
                self.visit_type_apply(x, handler);
                if handler.finished() {
                    return;
                }
            }
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
                let item = ItemOrAccess::Access(Access {
                    kind: AccessKind::DeclExtern,
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

        // visit extractor body.
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
                let item = ItemOrAccess::Access(Access {
                    kind: AccessKind::ImplExtractor,
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
                                    .map(|x| x.clone())
                                    .unwrap_or(UNKNOWN_TYPE.clone());
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

        provider.with_rule(|d| {
            let call = || {
                self.apply_matcher(&d.pattern, handler);
                for i in d.iflets.iter() {
                    self.apply_expr(&i.expr, handler);
                    if handler.finished() {
                        return;
                    }
                    self.apply_matcher(&i.pattern, handler);
                }
                self.apply_expr(&d.expr, handler);
            };

            self.context.enter_scope(call);
        });
    }

    fn visit_type_apply(&self, ty: &Ident, handler: &mut dyn ItemOrAccessHandler) {
        let item = ItemOrAccess::Access(Access {
            kind: AccessKind::AppleType,
            access: ty.clone(),
            def: self
                .context
                .query_item(&ty.0, |x| x.clone())
                .unwrap_or_default(),
        });
        handler.handle_item_or_access(self, &item);
    }
}

fn find_variant<'a>(xs: &'a Vec<Variant>, name: &str) -> Option<&'a Variant> {
    for x in xs.iter() {
        if x.name.0.as_str() == name {
            return Some(x);
        }
    }
    None
}

impl Project {
    pub(crate) fn apply_matcher(&self, p: &Pattern, handler: &mut dyn ItemOrAccessHandler) {
        let handle_term = |sym: &Ident,
                           handler: &mut dyn ItemOrAccessHandler|
         -> Option<(
            Vec<Ident>, //  not all them are filed.
            Ident,
        )> {
            match SplitedSymbol::from(sym) {
                SplitedSymbol::One(_) => {
                    let decl = self.context.query_item_clone(&sym.0);
                    let item = ItemOrAccess::Access(Access {
                        kind: AccessKind::ApplyEORC,
                        access: sym.clone(),
                        def: decl.clone(),
                    });
                    handler.handle_item_or_access(self, &item);
                    return match &decl {
                        Item::Decl { decl, .. } => {
                            Some((decl.arg_tys.clone(), decl.ret_ty.clone()))
                        }

                        _ => None,
                    };
                }

                SplitedSymbol::Two([x, y]) => {
                    let def = self
                        .context
                        .query_item(&x.symbol, |x| x.clone())
                        .unwrap_or_default();
                    let item = ItemOrAccess::Access(Access {
                        kind: AccessKind::ApplyEORC,
                        access: sym.clone(),
                        def: def.clone(),
                    });
                    handler.handle_item_or_access(self, &item);
                    if handler.finished() {
                        return None;
                    }
                    match def {
                        Item::Type { ty } => match &ty.ty {
                            TypeValue::Primitive(_, _) => {}
                            TypeValue::Enum(variants, _) => {
                                let v = find_variant(variants, y.symbol.as_str())
                                    .map(|x| x.clone())
                                    .unwrap_or(Variant {
                                        name: y.clone().into(),
                                        fields: vec![],
                                        pos: y.pos,
                                    });

                                let item = ItemOrAccess::Access(Access {
                                    access: y.clone().into(),
                                    kind: AccessKind::ApplyVariant(x.symbol.clone()),
                                    def: Item::EnumVariant { v: v.clone() },
                                });
                                handler.handle_item_or_access(self, &item);
                                if handler.finished() {
                                    return Some((
                                        v.fields.iter().map(|x| x.ty.clone()).collect(),
                                        ty.name.clone(),
                                    ));
                                }
                            }
                        },
                        _ => {}
                    };
                }
            };

            return None;
        };

        match p {
            Pattern::Var { var, .. } => {
                // in top level
                let item = ItemOrAccess::Access(Access {
                    access: var.clone(),
                    def: self.context.query_item_clone(&var.0),
                    kind: AccessKind::ImplConstructor,
                });
                handler.handle_item_or_access(self, &item);
            }
            Pattern::BindPattern { subpat, .. } => {
                self.apply_matcher(subpat.as_ref(), handler);
            }

            Pattern::ConstInt { .. } => {}
            Pattern::ConstPrim { val, .. } => {
                let item = ItemOrAccess::Access(Access {
                    access: val.clone(),
                    kind: AccessKind::ApplyConst,
                    def: self.context.query_const_clone(&val.0),
                });
                handler.handle_item_or_access(self, &item);
            }
            Pattern::Term { sym, args, .. } => {
                let tys = handle_term(sym, handler);
                // first pass.
                let enter_var =
                    |index: usize, var: &Ident, handler: &mut dyn ItemOrAccessHandler| {
                        let ty = tys
                            .as_ref()
                            .map(|x| &x.0)
                            .map(|x| x.get(index))
                            .flatten()
                            .map(|x| x.clone())
                            .unwrap_or(crate::item::UNKNOWN_TYPE.clone());
                        let item = ItemOrAccess::Item(Item::Var {
                            name: var.clone(),
                            ty,
                        });
                        handler.handle_item_or_access(self, &item);
                        self.context.enter_item(var.0.clone(), item);
                    };

                for (index, a) in args.iter().enumerate() {
                    match a {
                        Pattern::Var { var, .. } => {
                            enter_var(index, var, handler);
                        }
                        Pattern::BindPattern { var, .. } => {
                            enter_var(index, var, handler);
                        }
                        Pattern::ConstInt { .. } => {}
                        Pattern::ConstPrim { .. } => self.apply_matcher(a, handler),
                        Pattern::Term { .. } => self.apply_matcher(a, handler),
                        Pattern::Wildcard { .. } => {}
                        Pattern::And { subpats, .. } => {
                            for s in subpats.iter() {
                                self.apply_matcher(s, handler);
                            }
                        }
                        Pattern::MacroArg { .. } => {}
                    }
                }
            }
            Pattern::Wildcard { .. } => {}
            Pattern::And { subpats, .. } => {
                for s in subpats.iter() {
                    self.apply_matcher(s, handler);
                }
            }
            Pattern::MacroArg { .. } => {}
        }
    }
}

impl Project {
    pub(crate) fn apply_expr(&self, e: &Expr, handler: &mut dyn ItemOrAccessHandler) {
        let handle_term =
            |sym: &Ident, handler: &mut dyn ItemOrAccessHandler| match SplitedSymbol::from(sym) {
                SplitedSymbol::One(_) => {
                    let item = ItemOrAccess::Access(Access {
                        kind: AccessKind::ApplyEORC,
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
                }
                SplitedSymbol::Two([x, y]) => {
                    let def = self
                        .context
                        .query_item(&x.symbol, |x| x.clone())
                        .unwrap_or_default();
                    let item = ItemOrAccess::Access(Access {
                        kind: AccessKind::ApplyEORC,

                        access: sym.clone(),
                        def: def.clone(),
                    });
                    handler.handle_item_or_access(self, &item);
                    if handler.finished() {
                        return;
                    }
                    match def {
                        Item::Type { ty } => match &ty.ty {
                            TypeValue::Primitive(_, _) => {}
                            TypeValue::Enum(variants, _) => {
                                let v = find_variant(variants, y.symbol.as_str())
                                    .map(|x| x.clone())
                                    .unwrap_or(Variant {
                                        name: Ident(y.symbol.clone(), y.pos),
                                        fields: vec![],
                                        pos: y.pos,
                                    });

                                let item = ItemOrAccess::Access(Access {
                                    access: y.clone().into(),
                                    kind: AccessKind::ApplyVariant(x.symbol.clone()),
                                    def: Item::EnumVariant { v },
                                });
                                handler.handle_item_or_access(self, &item);
                                if handler.finished() {
                                    return;
                                }
                            }
                        },
                        _ => {}
                    };
                }
            };

        match e {
            Expr::Term { sym, args, .. } => {
                handle_term(sym, handler);
                for e in args.iter() {
                    self.apply_expr(e, handler);
                }
            }
            Expr::Var { name, .. } => {
                let item = ItemOrAccess::Access(Access {
                    access: name.clone(),
                    kind: AccessKind::ApplyVar,
                    def: self
                        .context
                        .query_item(&name.0, |x| x.clone())
                        .unwrap_or_default(),
                });
                handler.handle_item_or_access(self, &item);
            }
            Expr::ConstInt { .. } => {}
            Expr::ConstPrim { val, .. } => {
                let item = ItemOrAccess::Access(Access {
                    access: val.clone(),
                    kind: AccessKind::ApplyConst,
                    def: self
                        .context
                        .query_const(&val.0, |x| x.clone())
                        .unwrap_or_default(),
                });
                handler.handle_item_or_access(self, &item);
            }

            Expr::Let { defs, body, .. } => {
                let call = || {
                    for d in defs.iter() {
                        self.apply_expr(&d.val, handler);
                        if handler.finished() {
                            return;
                        }
                        let item = ItemOrAccess::Item(Item::Var {
                            name: d.var.clone(),
                            ty: d.ty.clone(),
                        });
                        handler.handle_item_or_access(self, &item);
                        if handler.finished() {
                            return;
                        }
                        self.visit_type_apply(&d.ty, handler);
                        if handler.finished() {
                            return;
                        }
                        self.context.enter_item(d.var.0.clone(), item);
                    }
                    self.apply_expr(body.as_ref(), handler);
                };

                self.context.enter_scope(call);
            }
        }
    }
}

impl Project {
    pub(crate) fn apply_extractor(&self, p: &Pattern, handler: &mut dyn ItemOrAccessHandler) {
        match p {
            Pattern::Var { var, pos: _ } => {
                let item = ItemOrAccess::Access(Access {
                    access: var.clone(),
                    kind: AccessKind::ExtractVar,
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
                let item = ItemOrAccess::Access(Access {
                    access: var.clone(),
                    kind: AccessKind::ApplyEORC,
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
                let item = ItemOrAccess::Access(Access {
                    access: val.clone(),
                    kind: AccessKind::ApplyConst,
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
                match SplitedSymbol::from(sym) {
                    SplitedSymbol::One(_) => {
                        let item = ItemOrAccess::Access(Access {
                            access: sym.clone(),
                            kind: AccessKind::ApplyEORC,
                            def: self
                                .context
                                .query_item(&sym.0, |x| x.clone())
                                .unwrap_or_default(),
                        });
                        handler.handle_item_or_access(self, &item);
                        if handler.finished() {
                            return;
                        }
                    }
                    SplitedSymbol::Two([x, y]) => {
                        let def = self
                            .context
                            .query_item(&x.symbol, |x| x.clone())
                            .unwrap_or_default();

                        let item = ItemOrAccess::Access(Access {
                            kind: AccessKind::ApplyEORC,
                            access: sym.clone(),
                            def: def.clone(),
                        });
                        handler.handle_item_or_access(self, &item);
                        if handler.finished() {
                            return;
                        }
                        match def {
                            Item::Type { ty } => match &ty.ty {
                                TypeValue::Primitive(_, _) => {}
                                TypeValue::Enum(variants, _) => {
                                    let v = find_variant(variants, y.symbol.as_str())
                                        .map(|x| x.clone())
                                        .unwrap_or(Variant {
                                            name: Ident(y.symbol.clone(), y.pos),
                                            fields: vec![],
                                            pos: y.pos,
                                        });
                                    let item = ItemOrAccess::Access(Access {
                                        kind: AccessKind::ApplyVariant(x.symbol.clone()),
                                        access: y.clone().into(),
                                        def: Item::EnumVariant { v },
                                    });
                                    handler.handle_item_or_access(self, &item);
                                    if handler.finished() {
                                        return;
                                    }
                                }
                            },
                            _ => {}
                        };
                    }
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
}
