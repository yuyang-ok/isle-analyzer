use std::collections::HashMap;

use super::context::*;

use crate::item::ItemOrAccess;

use super::item::*;

use lsp_server::*;
use lsp_types::*;

/// Handles go-to-def request of the language server.
pub fn on_rename(context: &Context, request: &Request) {
    let parameters = serde_json::from_value::<RenameParams>(request.params.clone())
        .expect("could not deserialize go-to-def request");
    let fpath = parameters.text_document_position.text_document.uri;
    let loc = parameters.text_document_position.position;
    let line = loc.line;
    let col = loc.character;
    log::info!(
        "request is goto definition,fpath:{:?}  line:{} col:{}",
        fpath,
        line,
        col,
    );
    let _send_err = || {
        let err = format!("{:?}{}:{} not found definition.", fpath.clone(), line, col);
        let r = Response::new_err(request.id.clone(), ErrorCode::UnknownErrorCode as i32, err);
        context
            .connection
            .sender
            .send(Message::Response(r))
            .unwrap();
    };
    let mut goto_definition = super::goto_definition::Handler::new(fpath.clone(), line, col);
    context
        .project
        .run_visitor_for_file(&fpath.to_file_path().unwrap(), &mut goto_definition);

    let def_loc = match goto_definition.result_item_or_access {
        Some(x) => match x {
            ItemOrAccess::Item(d) => d.def_loc(),
            ItemOrAccess::Access(Access { def, .. }) => def.def_loc(),
        },
        None => return,
    };
    let mut refs = super::references::Handler::new(def_loc, true);

    context.project.run_full_visitor(&mut refs);
    let mut r = Results::default();
    for v in refs.to_locations(&context.project).into_iter() {
        let e = TextEdit {
            range: v.range,
            new_text: parameters.new_name.clone(),
        };
        r.insert_edit(v.uri.clone(), e);
    }

    let r = Response::new_ok(
        request.id.clone(),
        serde_json::to_value(Some(WorkspaceEdit {
            changes: Some(r.edits),
            document_changes: None,
            change_annotations: None,
        }))
        .unwrap(),
    );
    context
        .connection
        .sender
        .send(Message::Response(r))
        .unwrap();
}

#[derive(Default)]
struct Results {
    edits: HashMap<Url, Vec<TextEdit>>,
}

impl Results {
    fn insert_edit(&mut self, u: url::Url, e: TextEdit) {
        if let Some(xxx) = self.edits.get_mut(&u) {
            xxx.push(e);
        } else {
            self.edits.insert(u, vec![e]);
        }
    }
}
