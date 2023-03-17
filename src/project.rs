use super::item::*;
use cranelift_isle::ast::*;
use cranelift_isle::error::Errors;
use cranelift_isle::lexer::*;
use cranelift_isle::parser::*;
use std::cell::RefCell;
use std::collections::HashMap;

use std::collections::HashSet;
use std::fs::FileType;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;

pub struct Project {
    pub(crate) defs: Defs,
    pub(crate) token_length: TokenLength,
    pub(crate) globals: Globals,
}

impl Project {
    pub fn empty() -> Self {
        Self {
            defs: Defs {
                defs: Default::default(),
                filenames: Default::default(),
                file_texts: Default::default(),
            },
            token_length: Default::default(),
            globals: Default::default(),
        }
    }

    pub fn from_walk() -> Result<Self, cranelift_isle::error::Errors> {
        let mut files = Vec::new();
        for x in walkdir::WalkDir::new(std::env::current_dir().unwrap()) {
            let x = match x {
                Ok(x) => x,
                Err(_) => {
                    continue;
                }
            };
            if x.file_type().is_file() && x.file_name().to_str().unwrap().ends_with(".isle") {
                files.push(x.path().to_path_buf());
            }
        }
        Self::new(files)
    }
    pub fn new(
        paths: impl IntoIterator<Item = PathBuf>,
    ) -> Result<Self, cranelift_isle::error::Errors> {
        let files: Vec<PathBuf> = paths.into_iter().collect();
        let l = Lexer::from_files(files.clone())?;
        let token_length = TokenLength::new(l.clone())?;
        let defs = parse(l)?;
        let x = Self {
            defs,
            token_length,
            globals: Globals::new(),
        };
        let mut dummy = DummyHandler {};
        x.run_full_visitor(&mut dummy);
        Ok(x)
    }

    pub fn run_visitor_for_file(&self, p: &PathBuf, handler: &mut dyn ItemOrAccessHandler) {
        let provider = match self.found_file_defs(p) {
            Some(x) => x,
            None => {
                log::error!("not found defs.");
                return;
            }
        };
        self.visit(provider, handler);
    }
    fn found_file_defs<'a>(&'a self, p: &PathBuf) -> Option<VecDefAstProvider<'a>> {
        let file_index = match self.found_file_index(p) {
            Some(x) => x,
            None => {
                log::error!("file index out found,{:?}", p);
                return None;
            }
        };
        return Some(self.get_vec_def_ast_provider_from_file_index(file_index));
    }

    fn get_vec_def_ast_provider_from_file_index<'a>(
        &'a self,
        file_index: usize,
    ) -> VecDefAstProvider<'a> {
        let mut ret = Vec::new();
        self.defs.defs.iter().for_each(|x| {
            if get_decl_pos(x)
                .map(|p| p.file == file_index)
                .unwrap_or(false)
            {
                ret.push(x);
            }
        });
        VecDefAstProvider::new(ret)
    }

    fn found_file_index(&self, p: &PathBuf) -> Option<usize> {
        for (index, x) in self.defs.filenames.iter().enumerate() {
            if x.to_string() == x.to_string() {
                return Some(index);
            }
        }
        None
    }

    pub fn run_full_visitor(&self, handler: &mut dyn ItemOrAccessHandler) {
        let provider = ProjectAstProvider::new(self);
        self.visit(provider, handler);
    }
}

fn get_decl_pos(d: &Def) -> Option<&Pos> {
    match d {
        Def::Pragma(x) => None,
        Def::Type(x) => Some(&x.pos),
        Def::Rule(x) => Some(&x.pos),
        Def::Extractor(x) => Some(&x.pos),
        Def::Decl(x) => Some(&x.pos),
        Def::Extern(x) => Some(match x {
            Extern::Extractor {
                term,
                func,
                pos,
                infallible,
            } => &term.1,
            Extern::Constructor { term, func, pos } => &term.1,
            Extern::Const { name, ty, pos } => &name.1,
        }),
        Def::Converter(x) => Some(&x.pos),
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
                    uri: url::Url::from_file_path(
                        PathBuf::from_str(s.as_str()).unwrap(), //
                    )
                    .unwrap(),
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
    fn handle_item_or_access(&mut self, p: &Project, _item: &ItemOrAccess);

    fn visit_body(&self) -> bool;

    /// Visitor should finished.
    fn finished(&self) -> bool;
}

pub trait AstProvider: Clone {
    fn with_def(&self, call_back: impl FnMut(&Def));
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
    fn with_def(&self, mut call_back: impl FnMut(&Def)) {
        self.p.defs.defs.iter().for_each(|x| call_back(x));
    }
}

#[derive(Clone)]
struct VecDefAstProvider<'a> {
    defs: Vec<&'a Def>,
}

impl<'a> VecDefAstProvider<'a> {
    fn new(defs: Vec<&'a Def>) -> Self {
        Self { defs }
    }
}

impl<'a> AstProvider for VecDefAstProvider<'a> {
    fn with_def(&self, mut call_back: impl FnMut(&Def)) {
        self.defs.iter().for_each(|x| call_back(*x))
    }
}

#[derive(Default, Clone)]
pub struct TokenLength {
    pos: HashMap<Pos, usize>,
}

impl TokenLength {
    pub(crate) fn to_lsp_range(&self, pos: &Pos) -> lsp_types::Range {
        // ISLE line start with 1
        // col start with 0
        let length = self.pos.get(pos).map(|x| *x).unwrap_or_default();
        lsp_types::Range {
            start: lsp_types::Position {
                line: (pos.line - 1) as u32,
                character: pos.col as u32,
            },
            end: lsp_types::Position {
                line: (pos.line - 1) as u32,
                character: pos.col as u32 + (length as u32),
            },
        }
    }
}

impl TokenLength {
    fn new(mut l: Lexer) -> Result<Self, cranelift_isle::error::Errors> {
        let mut ret = Self::default();
        while let Some((pos, t)) = l.next()? {
            ret.pos.insert(pos, Self::t_len(&t));
        }
        Ok(ret)
    }
    fn t_len(t: &Token) -> usize {
        match t {
            Token::LParen => 1,
            Token::RParen => 1,
            Token::Symbol(x) => x.len(),
            Token::Int(_) => 0, //  no IDE support on this.
            Token::At => 1,
        }
    }

    pub(crate) fn update_token_length(
        &mut self,
        file_index: usize,
        content: &str,
    ) -> Result<(), Errors> {
        let mut del = HashSet::new();
        self.pos.keys().for_each(|k| {
            if k.file == file_index {
                del.insert(k.clone());
            }
        });
        for d in del {
            self.pos.remove(&d);
        }
        let mut l = Lexer::from_str(content, "")?;
        while let Some((mut pos, t)) = l.next()? {
            pos.file = file_index;
            self.pos.insert(pos, Self::t_len(&t));
        }
        Ok(())
    }
}

struct DummyHandler {}

impl ItemOrAccessHandler for DummyHandler {
    fn visit_body(&self) -> bool {
        false
    }
    fn finished(&self) -> bool {
        false
    }
    fn handle_item_or_access(&mut self, _p: &Project, _item: &ItemOrAccess) {}
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
