use super::item::*;
use cranelift_isle::ast::*;
use cranelift_isle::error::Errors;
use cranelift_isle::lexer::*;
use cranelift_isle::parser::*;
use std::cell::RefCell;
use std::collections::HashMap;

use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;

pub struct Project {
    pub(crate) defs: Defs,
    pub(crate) token_length: TokenLength,
    pub(crate) globals: Globals,
}

impl Project {
    fn new(
        paths: impl IntoIterator<Item = PathBuf>,
    ) -> Result<Self, cranelift_isle::error::Errors> {
        let files: Vec<PathBuf> = paths.into_iter().collect();
        let l = Lexer::from_files(files.clone())?;
        let token_length = TokenLength::new(l.clone())?;
        let defs = parse(l)?;
        Ok(Self {
            defs,
            token_length,
            globals: Globals::new(),
        })
    }
}

impl Project {
    pub(crate) fn mk_location(&self, pos: &Pos) -> lsp_types::Location {
        self.defs
            .filenames
            .get(pos.file)
            .map(|x| {
                let s = x.as_ref().to_string();
                lsp_types::Location {
                    uri: url::Url::from_file_path(PathBuf::from_str(s.as_str()).unwrap()).unwrap(),
                    range: self.token_length.to_lsp_range(pos),
                }
            })
            .unwrap()
    }
}

pub(crate) struct Globals {
    scopes: Rc<RefCell<Vec<Scope>>>,
}

impl Default for Globals {
    fn default() -> Self {
        let x = Self {
            scopes: Rc::new(RefCell::new(vec![Scope::new()])),
        };
        x
    }
}

#[derive(Default, Clone)]
struct Scope {
    items: HashMap<String, Item>,
}

impl Scope {
    fn new() -> Self {
        Self::default()
    }
}

impl Globals {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn enter_item(&self, name: String, item: impl Into<Item>) {
        let item = item.into();
        self.scopes
            .as_ref()
            .borrow_mut()
            .last_mut()
            .unwrap()
            .items
            .insert(name, item);
    }

    pub(crate) fn query_item<R>(
        &self,
        name: &String,
        mut call_back: impl FnMut(&Item) -> R,
    ) -> Option<R> {
        for s in self.scopes.as_ref().borrow().iter().rev() {
            let t = s.items.get(name);
            if t.is_some() {
                return Some(call_back(t.unwrap()));
            }
        }
        None
    }

    pub(crate) fn fix_decl_type(&self, name: &String, decl_ty: u8) {
        let s = || -> Option<()> {
            match self
                .scopes
                .as_ref()
                .borrow_mut()
                .first_mut()
                .unwrap()
                .items
                .get_mut(name)?
            {
                Item::Decl { decl, kind: ty } => {
                    ty.0 = ty.0 | decl_ty;
                }
                _ => {}
            };
            None
        };
        s();
    }
    pub(crate) fn enter_scope(&self, mut x: impl FnMut()) {
        let _guard = ScopesGuarder::new(self);
        self.scopes.as_ref().borrow_mut().push(Scope::new());
        x()
    }
}

impl Project {
    pub(crate) fn file_exists(&self, p: &PathBuf) -> bool {
        let p = p.as_os_str().to_str().unwrap();
        self.defs.filenames.iter().any(|x| x.as_ref() == p)
    }
}

pub trait ItemOrAccessHandler {
    /// Handle this item.
    fn handle_item_or_access(&mut self, p: &Project, _item: &ItemOrAccess) {}

    fn visit_fun_or_spec_body(&self) -> bool;

    /// Visitor should finished.
    fn finished(&self) -> bool;
}

pub trait AstProvider: Clone {
    fn with_defs(&self, call_back: impl FnMut(&Defs));
    fn with_def(&self, mut call_back: impl FnMut(&Def)) {
        self.with_defs(|x| x.defs.iter().for_each(|x| call_back(x)));
    }
    fn with_pragma(&self, mut call_back: impl FnMut(&Pragma)) {
        self.with_def(|x| match x {
            Def::Pragma(x) => call_back(x),
            _ => {}
        })
    }
    fn with_type(&self, mut call_back: impl FnMut(&Type)) {
        self.with_def(|x| match x {
            Def::Type(x) => call_back(x),
            _ => {}
        })
    }
    fn with_rule(&self, mut call_back: impl FnMut(&Rule)) {
        self.with_def(|x| match x {
            Def::Rule(x) => call_back(x),
            _ => {}
        })
    }
    fn with_extractor(&self, mut call_back: impl FnMut(&Extractor)) {
        self.with_def(|x| match x {
            Def::Extractor(x) => call_back(x),
            _ => {}
        })
    }
    fn with_decl(&self, mut call_back: impl FnMut(&Decl)) {
        self.with_def(|x| match x {
            Def::Decl(x) => call_back(x),
            _ => {}
        })
    }
    fn with_extern(&self, mut call_back: impl FnMut(&Extern)) {
        self.with_def(|x| match x {
            Def::Extern(x) => call_back(x),
            _ => {}
        })
    }
    fn with_converter(&self, mut call_back: impl FnMut(&Converter)) {
        self.with_def(|x| match x {
            Def::Converter(x) => call_back(x),
            _ => {}
        })
    }
}

#[derive(Clone)]
struct ProjectAstProvider<'a> {
    p: &'a Project,
}

impl<'a> ProjectAstProvider<'a> {
    fn new(p: &'a Project) -> Self {
        Self { p }
    }
}

impl<'a> AstProvider for ProjectAstProvider<'a> {
    fn with_defs(&self, mut call_back: impl FnMut(&Defs)) {
        call_back(&self.p.defs);
    }
}

#[derive(Default, Clone)]
pub struct TokenLength {
    pos: HashMap<Pos, usize>,
}

impl TokenLength {
    pub(crate) fn to_lsp_range(&self, pos: &Pos) -> lsp_types::Range {
        let length = self.pos.get(pos).map(|x| *x).unwrap_or_default();
        lsp_types::Range {
            start: lsp_types::Position {
                line: pos.line as u32,
                character: pos.col as u32,
            },
            end: lsp_types::Position {
                line: pos.line as u32,
                character: pos.col as u32 + (length as u32),
            },
        }
    }
}

impl TokenLength {
    fn new(mut l: Lexer) -> Result<Self, cranelift_isle::error::Errors> {
        let mut ret = Self::default();
        while let Some((pos, t)) = l.next()? {
            ret.pos.insert(
                pos,
                match t {
                    Token::LParen => 1,
                    Token::RParen => 1,
                    Token::Symbol(x) => x.len(),
                    Token::Int(_) => 0, //  no IDE support on this.
                    Token::At => 1,
                },
            );
        }
        Ok(ret)
    }

    pub(crate) fn update_token_length(
        &mut self,
        file_index: usize,
        context: &str,
    ) -> Result<(), Errors> {
        unimplemented!()
    }
}

struct DummyHandler {}

impl ItemOrAccessHandler for DummyHandler {
    fn visit_fun_or_spec_body(&self) -> bool {
        false
    }
    fn finished(&self) -> bool {
        false
    }
}

pub(crate) fn get_patter_target(p: &Pattern) -> Option<&String> {
    match p {
        Pattern::Var { var, pos } => Some(&var.0),
        Pattern::BindPattern { var, subpat, pos } => Some(&var.0),
        Pattern::Term { sym, args, pos } => Some(&sym.0),
        _ => None,
    }
}

/// RAII type pop on when enter a scope.
#[must_use]
pub(crate) struct ScopesGuarder(Rc<RefCell<Vec<Scope>>>);

impl ScopesGuarder {
    pub(crate) fn new(s: &Globals) -> Self {
        Self(s.scopes.clone())
    }
}

impl Drop for ScopesGuarder {
    fn drop(&mut self) {
        self.0.as_ref().borrow_mut().pop().unwrap();
    }
}
