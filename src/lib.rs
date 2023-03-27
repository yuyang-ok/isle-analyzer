#[macro_use]
extern crate lazy_static;

use lsp_types::{Location, Position, Range};
use utils::GetPosAndLength;

pub mod comment;
pub mod completion;
pub mod context;
pub mod document_symbol;
pub mod fmt;
pub mod goto_definition;
pub mod hover;
#[cfg(test)]
pub mod ide_test;
pub mod inlay_hitnt;
pub mod item;
pub mod project;
pub mod project_visit;
pub mod references;
pub mod reload;
pub mod rename;
pub mod semantic_tokens;
pub mod utils;
use std::collections::HashSet;

pub fn readable_location(l: &Location) -> String {
    format!(
        "{}:{}:({},{})",
        l.uri.to_file_path().unwrap().to_str().unwrap(),
        l.range.start.line,
        l.range.start.character,
        l.range.end.character
    )
}

lazy_static! {
    pub static ref KEYWORDS: HashSet<&'static str> = {
        let mut t = HashSet::new();
        t.insert("rule");
        t.insert("convert");
        t.insert("extractor");
        t.insert("extern");
        t.insert("decl");
        t.insert("infallible");
        t.insert("pragma");
        t.insert("nodebug");
        t.insert("pure");
        t.insert("multi");
        t.insert("partial");
        t.insert("constructor");
        t.insert("type");
        t.insert("primitive");
        t.insert("enum");
        t
    };
}

pub(crate) fn to_lsp_range<T: GetPosAndLength>(x: &T) -> Range {
    let (pos, length) = x.get_pos_and_len();
    let line = (pos.line - 1) as u32;
    let col = pos.col as u32;
    Range {
        start: Position {
            line,
            character: col,
        },
        end: Position {
            line,
            character: col + length,
        },
    }
}
