use cranelift_isle::{ast::*, lexer::Pos};

#[derive(Clone)]
pub enum ItemOrAccess {
    Item(Item),
    Access(Access),
}

impl Into<Item> for ItemOrAccess {
    fn into(self) -> Item {
        match self {
            Self::Item(x) => x,
            _ => unreachable!(),
        }
    }
}

impl Into<Access> for ItemOrAccess {
    fn into(self) -> Access {
        match self {
            Self::Access(x) => x,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub enum Item {
    Type { ty: Type },
    Decl { decl: Decl, kind: DeclKind },
    Dummy,
    Const { name: Ident, ty: Ident },
    Var { name: Ident, ty: String },
}

impl Default for Item {
    fn default() -> Self {
        Self::Dummy
    }
}

impl Item {
    pub(crate) fn def_loc(&self) -> Pos {
        unimplemented!()
    }

    pub(crate) fn decl_nth_ty(&self, n: usize) -> Option<&Ident> {
        match self {
            Self::Decl { decl, .. } => decl.arg_tys.get(n),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DeclKind(pub(crate) u8);

impl DeclKind {
    pub(crate) const EXTRATOR: u8 = 1;
    pub(crate) const CONSTRUCTOR: u8 = 2;
}

#[derive(Clone)]
pub enum Access {
    AppleType { access: Ident, def: Box<Item> },
    DeclExtern { access: Ident, def: Box<Item> },
    ApplyExtractor { access: Ident, def: Box<Item> },
}

impl Access {
    pub(crate) fn access_def_loc(&self) -> (Pos, Pos) {
        match self {
            Access::AppleType { access, def } => (access.1, def.def_loc()),
            Access::DeclExtern { access, def } => (access.1, def.def_loc()),
            Access::ApplyExtractor { access, def } => (access.1, def.def_loc()),
        }
    }
}
