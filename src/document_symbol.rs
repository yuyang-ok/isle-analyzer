use super::context::*;
use super::project::*;

use lsp_server::*;
use lsp_types::*;

pub fn on_document_symbol_request(context: &Context, request: &Request) {
    let parameters = serde_json::from_value::<DocumentSymbolParams>(request.params.clone())
        .expect("could not deserialize document symbol request");
    let fpath = parameters.text_document.uri.to_file_path().unwrap();
    let result = vec![];
    let result = Response::new_ok(
        request.id.clone(),
        serde_json::to_value(DocumentSymbolResponse::Flat(result)).unwrap(),
    );
    context
        .connection
        .sender
        .send(Message::Response(result))
        .unwrap();
}
