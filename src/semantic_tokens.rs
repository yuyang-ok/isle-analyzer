use std::{cmp::Ordering, path::PathBuf};

use crate::project::AstProvider;

use super::context::*;
use super::to_lsp_range;
use cranelift_isle::ast::Pattern;
use cranelift_isle::{
    ast::*,
    error::Errors,
    lexer::{Lexer, Token},
};
use lsp_server::*;
use lsp_types::*;

/// Handles go-to-def request of the language server.
pub fn on_senantic_tokens(context: &Context, request: &Request) {
    let parameters = serde_json::from_value::<SemanticTokensParams>(request.params.clone())
        .expect("could not deserialize go-to-def request");
    let fpath = parameters.text_document.uri.to_file_path().unwrap();
    let asts = match context.project.found_file_defs(&fpath) {
        Some(x) => x,
        None => return,
    };
    let mut collector = AstSemanticTokenCollector::new();
    asts.with_def(|d| collector.collect_def(d));
    let mut tokens = collector.to_tokens();
    tokens.extend(match collect_keywords(&fpath) {
        Ok(x) => x,
        Err(_) => vec![],
    });
    tokens.sort_by(|a, b| {
        let o = a.range.start.line.cmp(&b.range.start.line);
        if o == Ordering::Equal {
            a.range.start.character.cmp(&b.range.start.character)
        } else {
            o
        }
    });
    let tokens = {
        let mut v = VecST::new();
        for t in tokens.into_iter() {
            v.push_back(t.range, t.token_type, t.modifiers);
        }
        v
    };
    let results = SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data: tokens.to_tokens(),
    });
    let r = Response::new_ok(
        request.id.clone(),
        serde_json::to_value(Some(results)).unwrap(),
    );
    context
        .connection
        .sender
        .send(Message::Response(r))
        .unwrap();
}

struct AstSemanticTokenCollector {
    results: Vec<TokenRange>,
}

fn collect_keywords(path: &PathBuf) -> Result<Vec<TokenRange>, Errors> {
    let content = std::fs::read_to_string(path.as_path()).unwrap();
    let mut lexer = Lexer::from_str(content.as_str(), path.as_path().to_str().unwrap())?;
    let mut ret = Vec::new();
    while let Some((pos, t)) = lexer.next()? {
        match t {
            Token::Symbol(s) => {
                if super::KEYWORDS.contains(s.as_str()) {
                    ret.push(TokenRange {
                        range: to_lsp_range(&(pos, s.len() as u32)),
                        token_type: TokenTypes::Keyword,
                        modifiers: None,
                    });
                }
            }
            _ => {}
        }
    }
    Ok(ret)
}

struct TokenRange {
    range: Range,
    token_type: TokenTypes,
    modifiers: Option<TokenModifier>,
}

#[allow(unused_macros)]
macro_rules! none_as_modifier {
    () => {{
        None as Option<TokenModifier>
    }};
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CollectPatternType {
    Extrator,
    Matcher,
}

impl CollectPatternType {
    fn to_modifier(self) -> Option<TokenModifier> {
        match self {
            CollectPatternType::Extrator => None,
            CollectPatternType::Matcher => Some(TokenModifier::Declaration),
        }
    }
}

impl AstSemanticTokenCollector {
    fn new() -> Self {
        Self { results: vec![] }
    }
    fn to_tokens(self) -> Vec<TokenRange> {
        self.results
    }
    fn collect_def(&mut self, d: &Def) {
        match d {
            Def::Pragma(_) => {}
            Def::Type(x) => self.collect_type(x),
            Def::Rule(x) => self.collect_rule(x),
            Def::Extractor(x) => self.collect_extractor(x),
            Def::Decl(x) => self.collect_decl(x),
            Def::Extern(x) => self.collect_extern(x),
            Def::Converter(x) => self.collect_converter(x),
        }
    }

    fn collect_type(&mut self, d: &Type) {
        self.results.push(TokenRange {
            range: to_lsp_range(&d.name),
            token_type: TokenTypes::Type,
            modifiers: Some(TokenModifier::Declaration),
        });
        match &d.ty {
            TypeValue::Primitive(x, _) => {
                self.results.push(TokenRange {
                    range: to_lsp_range(x),
                    token_type: TokenTypes::Type,
                    modifiers: None,
                });
            }
            TypeValue::Enum(enums, _) => {
                for v in enums.iter() {
                    self.results.push(TokenRange {
                        range: to_lsp_range(&v.name),
                        token_type: TokenTypes::EnumMember,
                        modifiers: Some(TokenModifier::Declaration),
                    });
                    for f in v.fields.iter() {
                        self.results.push(TokenRange {
                            range: to_lsp_range(&f.name),
                            token_type: TokenTypes::EnumMember,
                            modifiers: Some(TokenModifier::Declaration),
                        });
                        self.results.push(TokenRange {
                            range: to_lsp_range(&f.ty),
                            token_type: TokenTypes::Type,
                            modifiers: None,
                        });
                    }
                }
            }
        }
    }

    fn collect_rule(&mut self, d: &Rule) {
        self.collect_pattern(&d.pattern, CollectPatternType::Matcher);
        for i in d.iflets.iter() {
            self.collect_pattern(&i.pattern, CollectPatternType::Matcher);
            self.collect_expr(&i.expr);
        }

        self.collect_expr(&d.expr);
    }

    fn collect_expr(&mut self, e: &Expr) {
        match e {
            Expr::Term { sym, args, .. } => {
                self.results.push(TokenRange {
                    range: to_lsp_range(sym),
                    token_type: TokenTypes::Variable,
                    modifiers: None,
                });
                for a in args.iter() {
                    self.collect_expr(a);
                }
            }
            Expr::Var { name, .. } => {
                self.results.push(TokenRange {
                    range: to_lsp_range(name),
                    token_type: TokenTypes::Variable,
                    modifiers: None,
                });
            }
            Expr::ConstInt { val: _, pos: _ } => {}
            Expr::ConstPrim { val: _, pos: _ } => {}

            Expr::Let { defs, body, .. } => {
                for d in defs.iter() {
                    self.results.push(TokenRange {
                        range: to_lsp_range(&d.var),
                        token_type: TokenTypes::Number,
                        modifiers: Some(TokenModifier::Declaration),
                    });
                    self.results.push(TokenRange {
                        range: to_lsp_range(&d.ty),
                        token_type: TokenTypes::Type,
                        modifiers: Some(TokenModifier::Declaration),
                    });
                }
                self.collect_expr(body.as_ref());
            }
        }
    }
    fn collect_extractor(&mut self, d: &Extractor) {
        self.results.push(TokenRange {
            range: to_lsp_range(&d.term),
            token_type: TokenTypes::Function,
            modifiers: None,
        });
        for a in d.args.iter() {
            self.results.push(TokenRange {
                range: to_lsp_range(a),
                token_type: TokenTypes::Variable,
                modifiers: Some(TokenModifier::Declaration),
            });
        }
        self.collect_pattern(&d.template, CollectPatternType::Extrator);
    }

    fn collect_pattern(&mut self, p: &Pattern, mode: CollectPatternType) {
        match p {
            Pattern::Var { var, .. } => self.results.push(TokenRange {
                range: to_lsp_range(var),
                token_type: TokenTypes::Variable,
                modifiers: mode.to_modifier(),
            }),
            Pattern::BindPattern {
                var,
                subpat,
                pos: _,
            } => {
                self.results.push(TokenRange {
                    range: to_lsp_range(var),
                    token_type: TokenTypes::Variable,
                    modifiers: None,
                });
                self.collect_pattern(subpat.as_ref(), mode);
            }
            Pattern::ConstInt { val: _, pos } => {
                self.results.push(TokenRange {
                    range: to_lsp_range(&(pos.clone(), 0 as u32)),
                    token_type: TokenTypes::Number,
                    modifiers: None,
                });
            }
            Pattern::ConstPrim { val, .. } => {
                self.results.push(TokenRange {
                    range: to_lsp_range(val),
                    token_type: TokenTypes::Number,
                    modifiers: None,
                });
            }
            Pattern::Term { sym, args, pos: _ } => {
                self.results.push(TokenRange {
                    range: to_lsp_range(sym),
                    token_type: TokenTypes::Variable,
                    modifiers: None,
                });
                for a in args.iter() {
                    self.collect_pattern(a, mode);
                }
            }
            Pattern::Wildcard { pos: _ } => {}
            Pattern::And { subpats, pos: _ } => {
                for s in subpats.iter() {
                    self.collect_pattern(s, mode);
                }
            }
            Pattern::MacroArg { .. } => {}
        }
    }

    fn collect_decl(&mut self, d: &Decl) {
        self.results.push(TokenRange {
            range: to_lsp_range(&d.term),
            token_type: TokenTypes::Struct,
            modifiers: Some(TokenModifier::Declaration),
        });
        for t in d.arg_tys.iter() {
            self.results.push(TokenRange {
                range: to_lsp_range(t),
                token_type: TokenTypes::Type,
                modifiers: None,
            });
        }
        self.results.push(TokenRange {
            range: to_lsp_range(&d.ret_ty),
            token_type: TokenTypes::Type,
            modifiers: None,
        });
    }

    fn collect_extern(&mut self, d: &Extern) {
        match d {
            Extern::Extractor { term, .. } => {
                self.results.push(TokenRange {
                    range: to_lsp_range(term),
                    token_type: TokenTypes::Function,
                    modifiers: None,
                });
            }
            Extern::Constructor { term, .. } => {
                self.results.push(TokenRange {
                    range: to_lsp_range(term),
                    token_type: TokenTypes::Function,
                    modifiers: None,
                });
            }
            Extern::Const { name, ty, .. } => {
                self.results.push(TokenRange {
                    range: to_lsp_range(name),
                    token_type: TokenTypes::Variable,
                    modifiers: None,
                });
                self.results.push(TokenRange {
                    range: to_lsp_range(ty),
                    token_type: TokenTypes::Type,
                    modifiers: None,
                });
            }
        }
    }

    fn collect_converter(&mut self, d: &Converter) {
        self.results.push(TokenRange {
            range: to_lsp_range(&d.term),
            token_type: TokenTypes::Function,
            modifiers: Some(TokenModifier::Declaration),
        });
        self.results.push(TokenRange {
            range: to_lsp_range(&d.inner_ty),
            token_type: TokenTypes::Type,
            modifiers: None,
        });

        self.results.push(TokenRange {
            range: to_lsp_range(&d.outer_ty),
            token_type: TokenTypes::Type,
            modifiers: None,
        });
    }
}

#[derive(Debug, Clone, Copy, enum_iterator::Sequence)]
pub enum TokenTypes {
    Struct,
    Function,
    Variable,
    Keyword,
    String,
    Operator,
    EnumMember,
    Type,
    Number,
}

impl TokenTypes {
    #[allow(dead_code)]
    fn to_static_str(self) -> &'static str {
        match self {
            TokenTypes::Struct => "struct",
            TokenTypes::Function => "function",
            TokenTypes::Variable => "variable",
            TokenTypes::Keyword => "keyword",
            TokenTypes::String => "string",
            TokenTypes::Operator => "operator",
            TokenTypes::EnumMember => "enumMember",
            TokenTypes::Type => "type",
            TokenTypes::Number => "number",
        }
    }
    fn to_u32(self) -> u32 {
        self as u32
    }
}

impl Into<u32> for TokenTypes {
    fn into(self) -> u32 {
        self.to_u32()
    }
}

#[cfg(test)]
#[test]
fn ts_code() {
    let v: Vec<_> = enum_iterator::all::<TokenTypes>()
        .map(|x| format!("{}", x.to_static_str()))
        .collect();
    println!("const tokenTypes = {:?};", v);
    let v: Vec<_> = enum_iterator::all::<TokenModifier>()
        .map(|x| format!("{}", x.to_static_str()))
        .collect();

    println!("const tokenModifiers = {:?};", v)
}

#[derive(Debug, Clone, Copy, enum_iterator::Sequence)]
pub enum TokenModifier {
    Declaration,
}

impl Into<u32> for TokenModifier {
    fn into(self) -> u32 {
        self.to_u32()
    }
}

impl TokenModifier {
    #[allow(dead_code)] // actual used in test.
    fn to_static_str(self) -> &'static str {
        match self {
            Self::Declaration => "declaration",
        }
    }
    fn to_u32(self) -> u32 {
        self as u32
    }
}

#[derive(Default)]
pub struct VecST {
    tokens: Vec<SemanticToken>,
    last_line: u32,
    last_col_start: u32,
}

impl VecST {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn to_tokens(self) -> Vec<SemanticToken> {
        self.tokens
    }
    pub(crate) fn push_back(
        &mut self,
        range: lsp_types::Range,
        tt: impl Into<u32>,
        mid: Option<impl Into<u32>>,
    ) {
        debug_assert!(
            range.start.line == range.end.line && range.start.character <= range.end.character
        );
        let tt = tt.into();
        let mid = mid.map(|x| x.into()).unwrap_or_default();
        if self.tokens.len() > 0 {
            if self.last_line == range.start.line {
                self.tokens.push(SemanticToken {
                    delta_line: 0,
                    delta_start: range.start.character - self.last_col_start,
                    length: range.end.character - range.start.character,
                    token_type: tt,
                    token_modifiers_bitset: mid,
                });
                self.last_col_start = range.start.character;
            } else {
                self.tokens.push(SemanticToken {
                    delta_line: range.start.line - self.last_line,
                    delta_start: range.start.character,
                    length: range.end.character - range.start.character,
                    token_type: tt,
                    token_modifiers_bitset: mid,
                });
                self.last_line = range.start.line;
                self.last_col_start = range.start.character;
            }
        } else {
            self.tokens.push(SemanticToken {
                delta_line: range.start.line,
                delta_start: range.start.character,
                length: range.end.character - range.start.character,
                token_type: tt,
                token_modifiers_bitset: mid,
            });
            self.last_line = range.start.line;
            self.last_col_start = range.start.character;
        }
    }
}

#[cfg(test)]
mod test_vec_st {
    use super::*;
    fn assert_semantic_tokens(a: &Vec<SemanticToken>, b: &Vec<SemanticToken>) {
        assert_eq!(a.len(), b.len());
        for (k, (a, b)) in a.iter().zip(b.iter()).enumerate() {
            assert_eq!(a, b, "index at '{}' not equal", k);
        }
    }
    #[test]
    fn test_vec_st() {
        // struct `XXX` copy from https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_semanticTokens
        struct XXX {
            line: u32,
            start_char: u32,
            length: u32,
            token_type: u32,
            token_modifiers: u32,
        }
        impl XXX {
            fn to_range(&self) -> Range {
                Range {
                    start: Position {
                        line: self.line,
                        character: self.start_char,
                    },
                    end: Position {
                        line: self.line,
                        character: self.start_char + self.length,
                    },
                }
            }
        }
        let ss = vec![
            XXX {
                line: 2,
                start_char: 5,
                length: 3,
                token_type: 0,
                token_modifiers: 3,
            },
            XXX {
                line: 2,
                start_char: 10,
                length: 4,
                token_type: 1,
                token_modifiers: 0,
            },
            XXX {
                line: 5,
                start_char: 2,
                length: 7,
                token_type: 2,
                token_modifiers: 0,
            },
        ];
        let mut v = VecST::new();
        for s in ss.iter() {
            eprintln!("range:{:?}", s.to_range());
            v.push_back(s.to_range(), s.token_type, Some(s.token_modifiers));
        }
        let tokens = v.to_tokens();
        assert_semantic_tokens(
            &tokens,
            &vec![
                SemanticToken {
                    delta_line: 2,
                    delta_start: 5,
                    length: 3,
                    token_type: 0,
                    token_modifiers_bitset: 3,
                },
                SemanticToken {
                    delta_line: 0,
                    delta_start: 5,
                    length: 4,
                    token_type: 1,
                    token_modifiers_bitset: 0,
                },
                SemanticToken {
                    delta_line: 3,
                    delta_start: 2,
                    length: 7,
                    token_type: 2,
                    token_modifiers_bitset: 0,
                },
            ],
        )
    }
}
