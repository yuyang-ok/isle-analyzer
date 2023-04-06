use cranelift_isle::error::Errors;
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use super::context::Context;
use crate::{item::Item, send_err};
use lsp_server::{Message, Request, Response};

/// Handle show compiled code
pub fn on_show_compiled_code(context: &Context, request: &Request) {
    let parameters = serde_json::from_value::<Req>(request.params.clone())
        .expect("could not deserialize show compiled code request");
    let fpath = PathBuf::from_str(parameters.fpath.as_str()).unwrap();
    let line = parameters.line;
    let col = parameters.col;

    let mut handler = super::goto_definition::Handler::new(
        url::Url::from_file_path(fpath.clone()).unwrap(),
        line,
        col,
    );
    context.project.run_visitor_for_file(&fpath, &mut handler);

    let item = match handler.result_item_or_access {
        Some(x) => match x {
            crate::item::ItemOrAccess::Item(x) => x,
            crate::item::ItemOrAccess::Access(x) => x.def,
        },
        None => {
            send_err(context, "decl not found".to_string(), request.id.clone());
            return;
        }
    };

    let decl = match &item {
        Item::Decl { decl, .. } => decl,
        _ => {
            send_err(context, "Not a decl".to_string(), request.id.clone());
            return;
        }
    };
    let result = match from_files(context.project.mk_file_paths(), decl.term.0.clone()) {
        Ok(x) => x,
        Err(err) => {
            send_err(
                context,
                format!("compile failed,err:{:?}", err),
                request.id.clone(),
            );
            return;
        }
    };

    context
        .connection
        .sender
        .send(Message::Response(Response::new_ok(
            request.id.clone(),
            result,
        )))
        .unwrap();
}

#[derive(Clone, serde::Deserialize)]
struct Req {
    fpath: String,
    line: u32,
    col: u32,
}

#[derive(Clone, serde::Serialize, Debug)]
pub struct CompileResultAndPos {
    result: String,
    range: lsp_types::Range,
}

fn from_files<P: AsRef<Path>>(
    files: impl IntoIterator<Item = P>,
    name: String,
) -> Result<CompileResultAndPos, Errors> {
    let s = cranelift_isle::compile::from_files(
        files,
        &cranelift_isle::codegen::CodegenOptions {
            exclude_global_allow_pragmas: false,
        },
    )?;
    let mut line = 0;
    let mut col = 0;
    let mut length = 0;
    let match_str = format!("fn constructor_{}", name);
    for (index, l) in s.lines().enumerate() {
        if l.contains(match_str.as_str()) {
            line = index as u32;
            col = l.find(match_str.as_str()).unwrap() as u32;
            length = match_str.len() as u32;
        }
    }
    Ok(CompileResultAndPos {
        result: s,
        range: lsp_types::Range {
            start: lsp_types::Position {
                line: line,
                character: col,
            },
            end: lsp_types::Position {
                line: line,
                character: col + length,
            },
        },
    })
}

#[cfg(test)]
#[test]
fn xxx() {
    let x = from_files(vec![Path::new("./tests/bound_var.isle")], "A".to_string()).unwrap();

    eprintln!("{:?}", x);
}
