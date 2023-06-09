use super::context::*;
use super::project::*;
use crate::item::ItemOrAccess;

use crate::utils::GetPosition;

use lsp_server::*;
use lsp_types::*;

/// Handles go-to-def request of the language server.
pub fn on_go_to_def_request(context: &Context, request: &Request) {
    let parameters = serde_json::from_value::<GotoDefinitionParams>(request.params.clone())
        .expect("could not deserialize go-to-def request");
    let fpath = parameters.text_document_position_params.text_document.uri;
    let loc = parameters.text_document_position_params.position;
    let line = loc.line;
    let col = loc.character;
    log::info!(
        "request is goto definition,fpath:{:?}  line:{} col:{}",
        fpath,
        line,
        col,
    );
    let mut handler = Handler::new(fpath.clone(), line, col);
    context
        .project
        .run_visitor_for_file(&fpath.to_file_path().unwrap(), &mut handler);
    let locations = handler.to_locations();
    let r = Response::new_ok(
        request.id.clone(),
        serde_json::to_value(GotoDefinitionResponse::Array(locations)).unwrap(),
    );
    context
        .connection
        .sender
        .send(Message::Response(r))
        .unwrap();
}

impl Handler {
    fn to_locations(self) -> Vec<lsp_types::Location> {
        if let Some(x) = self.result {
            vec![x]
        } else {
            vec![]
        }
    }
}
pub(crate) struct Handler {
    /// The file we are looking for.
    pub(crate) filepath: url::Url,
    pub(crate) line: u32,
    pub(crate) col: u32,
    pub(crate) result: Option<Location>,

    pub(crate) result_item_or_access: Option<ItemOrAccess>,
}

impl Handler {
    pub(crate) fn new(p: url::Url, line: u32, col: u32) -> Self {
        Self {
            filepath: p,
            line,
            col,
            result: None,

            result_item_or_access: None,
        }
    }
}

impl ItemOrAccessHandler for Handler {
    fn finished(&self) -> bool {
        self.result.is_some()
    }
    fn visit_body(&self) -> bool {
        true
    }
    fn handle_item_or_access(&mut self, p: &Project, item_or_access: &ItemOrAccess) {
        match item_or_access {
            ItemOrAccess::Item(item) => {
                let def_loc = item.def_loc();
                let l = p.mk_location(&(def_loc.0, def_loc.1));
                if let Some(l) = l {
                    if (self as &dyn GetPosition).in_range(&l) {
                        self.result = Some(l.clone());
                        self.result_item_or_access = Some(item_or_access.clone());
                    }
                }
            }
            ItemOrAccess::Access(access) => {
                let (acc_pos, def, length) = access.access_def_loc();
                let l = p.mk_location(&(acc_pos, length));
                if let Some(l) = l {
                    if (self as &dyn GetPosition).in_range(&l) {
                        let l = p.mk_location(&(def, length));
                        if let Some(l) = l {
                            self.result = Some(l);
                            self.result_item_or_access = Some(item_or_access.clone());
                        }
                    }
                }
            }
        }
    }
}

impl GetPosition for Handler {
    fn get_position(&self) -> (url::Url, u32 /* line */, u32 /* col */) {
        (self.filepath.clone(), self.line, self.col)
    }
}
