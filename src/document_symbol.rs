use super::context::*;
use crate::project::{get_rule_target, AstProvider};
use lsp_server::*;
use lsp_types::*;
use std::collections::HashMap;

pub fn on_document_symbol_request(context: &Context, request: &Request) {
    let parameters = serde_json::from_value::<DocumentSymbolParams>(request.params.clone())
        .expect("could not deserialize document symbol request");
    let fpath = parameters.text_document.uri.to_file_path().unwrap();
    let mut result = vec![];
    let mut decls = DeclSymbolMap::new();
    let asts = match context.project.found_file_defs(&fpath) {
        Some(x) => x,
        None => {
            return;
        }
    };
    asts.with_type(|t| {
        let l = context.project.mk_location(&t.pos);
        if let Some(l) = l {
            result.push(DocumentSymbol {
                name: t.name.0.clone(),
                detail: None,
                kind: SymbolKind::STRUCT,
                tags: None,
                deprecated: None,
                range: l.range,
                selection_range: l.range,
                children: None,
            });
        }
    });
    asts.with_converter(|t| {
        let l = context.project.mk_location(&t.term.1);
        if let Some(l) = l {
            result.push(DocumentSymbol {
                name: t.term.0.clone(),
                detail: None,
                kind: SymbolKind::STRUCT,
                tags: None,
                deprecated: None,
                range: l.range,
                selection_range: l.range,
                children: None,
            });
        }
    });
    asts.with_decl(|x| {
        let l = context.project.mk_location(&x.term.1);
        if let Some(l) = l {
            decls.insert_decl(x.term.0.clone(), l.range);
        }
    });
    asts.with_rule(|x| {
        let name_and_pos = get_rule_target(&x.pattern);
        if let Some((name, pos)) = name_and_pos {
            let l = context.project.mk_location(&pos);
            if let Some(l) = l {
                decls.insert_decl_rule(
                    name.clone(),
                    DocumentSymbol {
                        name: DeclSymbolMap::rule_name(x.prio),
                        detail: None,
                        kind: SymbolKind::METHOD,
                        tags: None,
                        deprecated: None,
                        range: l.range,
                        selection_range: l.range,
                        children: None,
                    },
                );
            }
        } else {
            panic!("not found.");
        }
    });

    asts.with_extractor(|x| {
        let l = context.project.mk_location(&x.term.1);
        if let Some(l) = l {
            decls.insert_decl_rule(
                x.term.0.clone(),
                DocumentSymbol {
                    name: "extractor".to_string(),
                    detail: None,
                    kind: SymbolKind::METHOD,
                    tags: None,
                    deprecated: None,
                    range: l.range,
                    selection_range: l.range,
                    children: None,
                },
            );
        }
    });
    result.extend(decls.to_document_symbols().into_iter());
    let result = Response::new_ok(
        request.id.clone(),
        serde_json::to_value(DocumentSymbolResponse::Nested(result)).unwrap(),
    );
    context
        .connection
        .sender
        .send(Message::Response(result))
        .unwrap();
}

#[derive(Default)]
struct DeclSymbolMap {
    decls: HashMap<String, DeclSymbol>,
}

impl DeclSymbolMap {
    fn new() -> Self {
        Self::default()
    }

    fn insert_decl(&mut self, name: String, range: Range) {
        self.decls
            .insert(name.clone(), DeclSymbol::new(name, range));
    }

    fn rule_name(prio: Option<i64>) -> String {
        format!(
            "rule_{}",
            prio.map(|x| x.to_string()).unwrap_or("xxx".to_string())
        )
    }

    fn insert_decl_rule(&mut self, name: String, d: DocumentSymbol) {
        if let Some(x) = self.decls.get_mut(&name) {
            x.subs.push(d);
        } else {
            // not found
            // This is maybe implement a constructor in someother file
            // instead of the file that define.
        }
    }

    fn to_document_symbols(self) -> Vec<DocumentSymbol> {
        let mut ret = Vec::with_capacity(self.decls.len());
        for (_, v) in self.decls.into_iter() {
            ret.push(v.to_document_symbols());
        }
        ret
    }
}
struct DeclSymbol {
    range: Range,
    name: String,
    subs: Vec<DocumentSymbol>,
}

impl DeclSymbol {
    fn new(name: String, range: Range) -> Self {
        Self {
            name,
            range,
            subs: vec![],
        }
    }

    fn to_document_symbols(self) -> DocumentSymbol {
        let name = self.name.clone();
        let range = self.range;
        DocumentSymbol {
            name,
            detail: None,
            kind: SymbolKind::OPERATOR,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: Some(self.subs),
        }
    }
}
