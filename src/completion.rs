// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::item::*;
use super::project::*;
use super::utils::*;
use crate::context::Context;
use lsp_server::*;

use lsp_types::*;

use std::vec;

/// Return a list of completion items corresponding to each one of Move's keywords.
///
/// Currently, this does not filter keywords out based on whether they are valid at the completion
/// request's cursor position, but in the future it ought to. For example, this function returns
/// all specification language keywords, but in the future it should be modified to only do so
/// within a spec block.
fn keywords() -> Vec<CompletionItem> {
    unimplemented!()
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
    let result = handler.result.unwrap_or(vec![]);
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
    completion_on_def: bool,
}

impl Handler {
    pub(crate) fn new(filepath: Url, line: u32, col: u32) -> Self {
        Self {
            filepath,
            line,
            col,
            result: None,
            completion_on_def: false,
        }
    }
}

impl ItemOrAccessHandler for Handler {
    fn visit_body(&self) -> bool {
        true
    }
    fn handle_item_or_access(&mut self, _p: &Project, _item_or_access: &ItemOrAccess) {
        unimplemented!()
    }
    fn finished(&self) -> bool {
        self.result.is_some() || self.completion_on_def
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
            kind: Some(CompletionItemKind::CONSTANT),
            ..Default::default()
        },
    };
    Some(x)
}
