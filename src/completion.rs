// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]
use super::item::*;
use super::project::*;
use super::utils::*;
use crate::context::Context;

use cranelift_isle::ast::Decl;
use cranelift_isle::ast::Ident;
use cranelift_isle::ast::Type;
use cranelift_isle::ast::TypeValue;
use lsp_server::*;
use lsp_types::*;
use std::vec;

fn keywords() -> Vec<CompletionItem> {
    super::KEYWORDS
        .iter()
        .map(|x| CompletionItem {
            label: (*x).to_string(),
            kind: Some(CompletionItemKind::KEYWORD),

            ..Default::default()
        })
        .collect()
}

/// Sends the given connection a response to a completion request.
///
/// The completions returned depend upon where the user's cursor is positioned.
pub fn on_completion_request(context: &Context, request: &Request) {
    let parameters = serde_json::from_value::<CompletionParams>(request.params.clone())
        .expect("could not deserialize references request");
    let fpath = parameters.text_document_position.text_document.uri;
    let loc = parameters.text_document_position.position;
    let line = loc.line;
    let col = loc.character;
    let mut handler = Handler::new(fpath.clone(), line, col);
    context
        .project
        .run_visitor_for_file(&fpath.to_file_path().unwrap(), &mut handler);

    let mut result = handler.result.unwrap_or(vec![]);
    if result.len() == 0 {
        context.project.context.all_top_items(|x| {
            if let Some(c) = item_to_completion_item(&x) {
                result.push(c);
            }
        });
        result.extend(keywords().into_iter());
    }
    let ret = Some(CompletionResponse::Array(result));
    let r = Response::new_ok(request.id.clone(), serde_json::to_value(ret).unwrap());
    context
        .connection
        .sender
        .send(Message::Response(r))
        .unwrap();
}

pub(crate) struct Handler {
    /// The file we are looking for.
    pub(crate) filepath: Url,
    pub(crate) line: u32,
    pub(crate) col: u32,
    pub(crate) result: Option<Vec<CompletionItem>>,
}

impl Handler {
    pub(crate) fn new(filepath: Url, line: u32, col: u32) -> Self {
        Self {
            filepath,
            line,
            col,
            result: None,
        }
    }
    fn match_loc(&self, range: &Location) -> bool {
        (self as &dyn GetPosition).in_range(range)
    }
}

impl ItemOrAccessHandler for Handler {
    fn visit_body(&self) -> bool {
        true
    }

    fn handle_item_or_access(&mut self, p: &Project, item_or_access: &ItemOrAccess) {
        let push_completion_items = |visitor: &mut Handler, items: Vec<CompletionItem>| {
            if visitor.result.is_none() {
                visitor.result = Some(vec![]);
            }
            items
                .into_iter()
                .for_each(|x| visitor.result.as_mut().unwrap().push(x));
        };

        match item_or_access {
            ItemOrAccess::Item(_item) => {}
            ItemOrAccess::Access(access) => {
                let (access_pos, _, length) = access.access_def_loc();
                let access_loc = p.mk_location(&(access_pos, length));
                if access_loc
                    .as_ref()
                    .map(|l| self.match_loc(l))
                    .unwrap_or(false)
                    == false
                {
                    return;
                }
                match &access.kind {
                    AccessKind::AppleType => {
                        let mut items = vec![];
                        p.context.all_types(|x| {
                            items.push(CompletionItem {
                                label: x.name.0.clone(),
                                kind: Some(CompletionItemKind::STRUCT),
                                ..Default::default()
                            })
                        });
                        push_completion_items(self, items);
                    }
                    AccessKind::DeclExtern => {
                        let mut items = vec![];
                        p.context.all_decl(|x| {
                            items.push(CompletionItem {
                                label: x.term.0.clone(),
                                kind: Some(CompletionItemKind::CLASS),
                                ..Default::default()
                            })
                        });
                        push_completion_items(self, items);
                    }
                    AccessKind::ApplyEORC => {
                        let mut items = vec![];
                        p.context.all_decl(|x| {
                            items.push(CompletionItem {
                                label: x.term.0.clone(),
                                kind: Some(CompletionItemKind::CLASS),
                                ..Default::default()
                            })
                        });
                        p.context.all_types(|x| {
                            if matches!(&x.ty, TypeValue::Enum(_, _)) {
                                items.push(CompletionItem {
                                    label: x.name.0.clone(),
                                    kind: Some(CompletionItemKind::CLASS),
                                    ..Default::default()
                                })
                            }
                        });
                        push_completion_items(self, items);
                    }
                    AccessKind::ExtractVar => {
                        let mut items = vec![];
                        p.context.all_vars(|name, _| {
                            items.push(CompletionItem {
                                label: name.0.clone(),
                                kind: Some(CompletionItemKind::VARIABLE),
                                ..Default::default()
                            })
                        });
                        push_completion_items(self, items);
                    }
                    AccessKind::ApplyConst => {
                        let mut items = vec![];
                        p.context.all_consts(|name, _| {
                            items.push(CompletionItem {
                                label: name.0.clone(),
                                kind: Some(CompletionItemKind::VARIABLE),
                                ..Default::default()
                            })
                        });
                        push_completion_items(self, items);
                    }
                    AccessKind::ImplExtractor => {
                        let mut items = vec![];
                        p.context.all_decl(|name| {
                            items.push(CompletionItem {
                                label: name.term.0.clone(),
                                kind: Some(CompletionItemKind::VARIABLE),
                                ..Default::default()
                            })
                        });
                        push_completion_items(self, items);
                    }
                    AccessKind::ImplConstructor => {
                        let mut items = vec![];
                        p.context.all_decl(|name| {
                            items.push(CompletionItem {
                                label: name.term.0.clone(),
                                kind: Some(CompletionItemKind::VARIABLE),
                                ..Default::default()
                            })
                        });
                        push_completion_items(self, items);
                    }
                    AccessKind::ApplyVariant(name) => {
                        let mut v = None;
                        p.context.all_types(|x| {
                            if x.name.0.as_str() == name.as_str() {
                                match &x.ty {
                                    cranelift_isle::ast::TypeValue::Primitive(_, _) => {}
                                    cranelift_isle::ast::TypeValue::Enum(vs, _) => {
                                        v = Some(vs.clone());
                                    }
                                }
                            }
                        });
                        let mut items = vec![];
                        if let Some(vs) = v {
                            for v in vs.iter() {
                                items.push(CompletionItem {
                                    label: v.name.0.clone(),
                                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                                    ..Default::default()
                                })
                            }
                        }
                        push_completion_items(self, items);
                    }
                    AccessKind::ApplyVar => {
                        let mut items = vec![];
                        p.context.all_vars(|name, _| {
                            items.push(CompletionItem {
                                label: name.0.clone(),
                                kind: Some(CompletionItemKind::VARIABLE),
                                ..Default::default()
                            })
                        });
                        push_completion_items(self, items);
                    }
                };
            }
        }
    }
    fn finished(&self) -> bool {
        self.result.is_some()
    }
}

impl std::fmt::Display for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "completion,file:{:?} line:{} col:{}",
            self.filepath, self.line, self.col
        )
    }
}

impl GetPosition for Handler {
    fn get_position(&self) -> (Url, u32 /* line */, u32 /* col */) {
        (self.filepath.clone(), self.line, self.col)
    }
}

fn item_to_completion_item(item: &Item) -> Option<CompletionItem> {
    let x = match item {
        Item::Type { ty } => CompletionItem {
            label: ty.name.0.clone(),
            kind: Some(CompletionItemKind::STRUCT),
            ..Default::default()
        },
        Item::Decl { decl, .. } => CompletionItem {
            label: decl.term.0.clone(),
            kind: Some(CompletionItemKind::CONSTRUCTOR),
            ..Default::default()
        },
        Item::Dummy => return None,
        Item::Const { name, .. } => CompletionItem {
            label: name.0.clone(),
            kind: Some(CompletionItemKind::CONSTANT),
            ..Default::default()
        },

        Item::Var { name, .. } => CompletionItem {
            label: name.0.clone(),
            kind: Some(CompletionItemKind::VARIABLE),
            ..Default::default()
        },
        Item::EnumMemberName { name } => CompletionItem {
            label: name.0.clone(),
            kind: Some(CompletionItemKind::FIELD),
            ..Default::default()
        },
        Item::EnumMemberField { name: _ } => return None,
        Item::EnumVariant { v } => CompletionItem {
            label: v.name.0.clone(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            ..Default::default()
        },
    };
    Some(x)
}

impl VisitContext {
    pub(crate) fn all_types(&self, mut call_back: impl FnMut(&Type)) {
        self.all_top_items(|i| match i {
            Item::Type { ty } => {
                call_back(ty);
            }
            _ => {}
        });
    }
    pub(crate) fn all_decl(&self, mut call_back: impl FnMut(&Decl)) {
        self.all_top_items(|i| match i {
            Item::Decl { decl, .. } => {
                call_back(decl);
            }
            _ => {}
        });
    }
    pub(crate) fn all_consts(&self, mut call_back: impl FnMut(&Ident, &Ident)) {
        self.all_top_items(|i| match i {
            Item::Const { name, ty } => {
                call_back(name, ty);
            }
            _ => {}
        });
    }
    pub(crate) fn all_top_items(&self, mut call_back: impl FnMut(&Item)) {
        self.scopes
            .as_ref()
            .borrow()
            .first()
            .unwrap()
            .items
            .iter()
            .for_each(|(_, i)| call_back(i));
    }
    pub(crate) fn all_extractor(&self, call_back: impl FnMut(&Decl)) {
        self.decl_(call_back, DeclKind::EXTRATOR);
    }
    pub(crate) fn all_constructor(&self, call_back: impl FnMut(&Decl)) {
        self.decl_(call_back, DeclKind::CONSTRUCTOR);
    }
    pub(crate) fn decl_(&self, mut call_back: impl FnMut(&Decl), x: u8) {
        self.all_top_items(|i| match i {
            Item::Decl { decl, kind } => {
                if kind.has(x) {
                    call_back(decl);
                }
            }
            _ => {}
        });
    }

    pub(crate) fn all_vars(
        &self,
        mut call_back: impl FnMut(
            &Ident, // name
            &Ident, // ty
        ),
    ) {
        self.innert_most(|x| match x {
            Item::Var { name, ty, .. } => call_back(name, ty),
            _ => {}
        })
    }

    fn innert_most(&self, mut call_back: impl FnMut(&Item)) {
        self.scopes
            .as_ref()
            .borrow()
            .iter()
            .rev()
            .for_each(|x| x.items.values().for_each(|i| call_back(i)))
    }
}
