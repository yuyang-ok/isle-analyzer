use super::context::Context;
use super::goto_definition;
use super::item::*;
use super::project::*;
use super::utils::*;
use cranelift_isle::lexer::Pos;
use lsp_server::*;
use lsp_types::*;
use std::collections::HashSet;
use std::path::*;

pub fn on_references_request(context: &mut Context, request: &Request) {
    let parameters = serde_json::from_value::<ReferenceParams>(request.params.clone())
        .expect("could not deserialize references request");
    let fpath = parameters.text_document_position.text_document.uri;
    let loc = parameters.text_document_position.position;
    let line = loc.line;
    let col = loc.character;
    let include_declaration = parameters.context.include_declaration;
    // first find definition.
    let mut goto_definition = goto_definition::Handler::new(fpath.clone(), line, col);

    context
        .project
        .run_visitor_for_file(&fpath.to_file_path().unwrap(), &mut goto_definition);
    let send_err = || {
        let err = format!("{:?}{}:{} not found definition.", fpath.clone(), line, col);
        let r = Response::new_err(request.id.clone(), ErrorCode::UnknownErrorCode as i32, err);
        context
            .connection
            .sender
            .send(Message::Response(r))
            .unwrap();
    };
    let def_loc = match goto_definition.result_pos {
        Some(x) => x,
        None => {
            send_err();
            return;
        }
    };

    let is_local = false;
    let mut handle = Handler::new(def_loc, include_declaration);
    if is_local {
        let _ = context
            .project
            .run_visitor_for_file(&fpath.to_file_path().unwrap(), &mut handle);
    } else {
        context.project.run_full_visitor(&mut handle);
    }
    let locations = handle.to_locations(&context.project);
    let loc = Some(locations.clone());
    let r = Response::new_ok(request.id.clone(), serde_json::to_value(loc).unwrap());
    context
        .connection
        .sender
        .send(Message::Response(r))
        .unwrap();
}

struct Handler {
    def_loc: Pos,
    include_declaration: bool,
    refs: HashSet<Pos>,
}

impl Handler {
    pub(crate) fn new(def_loc: Pos, include_declaration: bool) -> Self {
        Self {
            def_loc,
            include_declaration,
            refs: Default::default(),
        }
    }

    pub(crate) fn to_locations(self, convert_loc: &Project) -> Vec<Location> {
        let mut file_ranges = Vec::with_capacity(self.refs.len() + 1);
        if self.include_declaration {
            file_ranges.push(convert_loc.mk_location(&self.def_loc));
        }
        for x in self.refs.iter() {
            file_ranges.push(convert_loc.mk_location(x));
        }
        file_ranges
    }
}

impl ItemOrAccessHandler for Handler {
    fn handle_item_or_access(
        &mut self,
        p: &super::project::Project,
        item: &crate::item::ItemOrAccess,
    ) {
        match item {
            ItemOrAccess::Item(_) => {}
            ItemOrAccess::Access(access) => match item {
                _ => {
                    let (access, def) = access.access_def_loc();
                    if def == self.def_loc {
                        self.refs.insert(access);
                        return;
                    }
                }
            },
        }
    }
    fn finished(&self) -> bool {
        false
    }
    fn visit_body(&self) -> bool {
        true
    }
}

impl std::fmt::Display for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "find references for {:?}", self.def_loc)
    }
}
