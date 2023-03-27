use super::context::Context;
use super::goto_definition;
use super::item::*;
use super::project::Project;
use lsp_server::*;
use lsp_types::*;

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
    let hover = item.map(|x| hover_on_item_or_access(&x, &context.project));
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

fn hover_on_item_or_access(ia: &ItemOrAccess, p: &Project) -> String {
    let item_hover = |item: &Item| -> String {
        let pos = item.def_loc();
        let fpath = p.file_index_path(pos.0.file);
        let comment = if let Some(fpath) = fpath.as_ref() {
            p.comments
                .get(fpath)
                .map(|d| d.get_comment(&pos.0).map(|x| x.as_str()))
                .flatten()
                .unwrap_or("")
        } else {
            ""
        };
        let item_str = match item {
            Item::EnumVariant { v } if v.fields.len() > 0 => {
                let mut s = format!("{}\n\n", v.name.0);
                for f in v.fields.iter() {
                    // FIXME \n not make a newline on UI
                    s.push_str(format!("  {}:{}\n\n", f.name.0, f.ty.0).as_str());
                }
                s
            }
            _ => format!("{}", item),
        };
        format!("{}\n{}", comment, item_str)
    };
    match ia {
        ItemOrAccess::Item(item) => item_hover(item),
        ItemOrAccess::Access(acc) => item_hover(acc.def_item()),
    }
}
