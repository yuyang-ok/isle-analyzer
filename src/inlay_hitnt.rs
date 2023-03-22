use crate::utils::GetPosition;

use super::context::*;
use super::item::*;
use super::project::*;
use cranelift_isle::ast::Ident;
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

    fn in_range(&self, project: &Project, pos: Pos) -> Option<Location> {
        let l = project.mk_location(&pos);
        if let Some(l) = l {
            if Location::in_range(&l, &self.range) {
                Some(l)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl ItemOrAccessHandler for Handler {
    fn handle_item_or_access(&mut self, p: &Project, item: &ItemOrAccess) {
        match item {
            ItemOrAccess::Item(item) => match item {
                Item::Var { name, ty } => {
                    let name_loc = self.in_range(p, name.1);
                    if let Some(name_loc) = name_loc {
                        self.reuslts.push(mk_inlay_hits(
                            Position {
                                line: name_loc.range.end.line,
                                character: name_loc.range.end.character,
                            },
                            ty_inlay_hints_label_parts(&ty.0, p),
                            InlayHintKind::TYPE,
                        ));
                    }
                }
                _ => {}
            },
            ItemOrAccess::Access(_acc) => {}
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

/// There command should implemented in `LSP` client.
pub enum MoveAnalyzerClientCommands {
    GotoDefinition(Location),
}

impl MoveAnalyzerClientCommands {
    pub(crate) fn to_lsp_command(self) -> Command {
        match self {
            MoveAnalyzerClientCommands::GotoDefinition(x) => Command::new(
                "Goto Definition".to_string(),
                "isle-analyzer.goto_definition".to_string(),
                Some(vec![serde_json::to_value(PathAndRange::from(&x)).unwrap()]),
            ),
        }
    }
}

#[derive(Clone, serde::Serialize)]
pub struct PathAndRange {
    range: Range,
    fpath: String,
}

impl From<&Location> for PathAndRange {
    fn from(value: &Location) -> Self {
        Self {
            range: value.range,
            fpath: value
                .uri
                .to_file_path()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
        }
    }
}

fn ty_inlay_hints_label_parts(ty: &String, p: &Project) -> InlayHintLabel {
    InlayHintLabel::LabelParts(vec![InlayHintLabelPart {
        value: ty.clone(),
        tooltip: Some(InlayHintLabelPartTooltip::String(
            "Go To Definition.".to_string(),
        )),
        location: None,
        command: if let Some(loc) = p.mk_location(&p.context.query_item_clone(ty).def_loc()) {
            Some(MoveAnalyzerClientCommands::GotoDefinition(loc).to_lsp_command())
        } else {
            None
        },
    }])
}
