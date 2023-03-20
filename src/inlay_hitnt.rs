use crate::utils::GetPosition;

use super::context::*;
use super::item::*;
use super::project::*;
use cranelift_isle::lexer::Pos;
use lsp_server::*;
use lsp_types::*;

/// Handles go-to-def request of the language server.
pub fn on_inlay_hints(context: &Context, request: &Request) {
    let parameters = serde_json::from_value::<InlayHintParams>(request.params.clone())
        .expect("could not deserialize go-to-def request");
    let fpath = parameters.text_document.uri.clone();
    let mut handler = Handler::new(fpath.clone(), parameters.clone().range);
    let _ = context.project.run_visitor_for_file(
        &parameters.text_document.uri.to_file_path().unwrap(),
        &mut handler,
    );
    let hints = Some(handler.reuslts);
    let r = Response::new_ok(request.id.clone(), serde_json::to_value(hints).unwrap());
    context
        .connection
        .sender
        .send(Message::Response(r))
        .unwrap();
}

struct Handler {
    range: Location,
    reuslts: Vec<InlayHint>,
}

impl Handler {
    fn new(fpath: Url, range: Range) -> Self {
        Self {
            range: lsp_types::Location { uri: fpath, range },
            reuslts: Default::default(),
        }
    }
    fn in_range(&self, project: &Project, pos: Pos) -> bool {
        Location::in_range(&project.mk_location(&pos), &self.range)
    }
}

impl ItemOrAccessHandler for Handler {
    fn handle_item_or_access(&mut self, p: &Project, item: &ItemOrAccess) {
        match item {
            ItemOrAccess::Item(item) => {}
            ItemOrAccess::Access(acc) => {}
        }
    }
    fn visit_body(&self) -> bool {
        true
    }
    fn finished(&self) -> bool {
        false
    }
}

impl std::fmt::Display for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "run visit for inlay hits")
    }
}

fn mk_inlay_hits(pos: Position, label: InlayHintLabel, kind: InlayHintKind) -> InlayHint {
    InlayHint {
        position: pos,
        label,
        kind: Some(kind),
        text_edits: None,
        tooltip: None,
        padding_left: Some(true),
        padding_right: Some(true),
        data: None,
    }
}
