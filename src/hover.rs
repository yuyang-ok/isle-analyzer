use super::context::Context;
use super::goto_definition;
use super::item::*;
use super::utils::*;
use lsp_server::*;
use lsp_types::*;
use std::path::PathBuf;

/// Handles hover request of the language server.
pub fn on_hover_request(context: &Context, request: &Request) {
    let parameters = serde_json::from_value::<HoverParams>(request.params.clone())
        .expect("could not deserialize hover request");
    let fpath = parameters.text_document_position_params.text_document.uri;
    let loc = parameters.text_document_position_params.position;
    let line = loc.line;
    let col = loc.character;
    log::info!(
        "request is hover,fpath:{:?}  line:{} col:{}",
        fpath,
        line,
        col,
    );
    let mut handler = goto_definition::Handler::new(fpath.clone(), line, col);
    context
        .project
        .run_visitor_for_file(&fpath.to_file_path().unwrap(), &mut handler);
    let item = handler.result_item_or_access.clone();
    let hover = item.map(|x| hover_on_item_or_access(&x));
    let hover = hover.map(|x| Hover {
        contents: HoverContents::Scalar(MarkedString::String(x)),
        range: None,
    });
    let r = Response::new_ok(request.id.clone(), serde_json::to_value(hover).unwrap());
    context
        .connection
        .sender
        .send(Message::Response(r))
        .unwrap();
}

fn hover_on_item_or_access(ia: &ItemOrAccess) -> String {
    unimplemented!()
}
