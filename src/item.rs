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

pub const UNKNOWN_POS: Pos = Pos {
    file: 999,
    offset: 0,
    line: 0,
    col: 0,
};

impl Item {
    pub(crate) fn def_loc(&self) -> Pos {
        match self {
            Item::Type { ty } => ty.pos,
            Item::Decl { decl, kind: _ } => decl.term.1,
            Item::Dummy => UNKNOWN_POS,
            Item::Const { name, ty: _ } => name.1,
            Item::Var { name, ty: _ } => name.1,
        }
    }

    pub(crate) fn def_file(&self) -> usize {
        self.def_loc().file
    }

    pub(crate) fn decl_nth_ty(&self, n: usize) -> Option<&Ident> {
        match self {
            Self::Decl { decl, .. } => decl.arg_tys.get(n),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Default)]
pub struct DeclKind(pub(crate) u8);

impl DeclKind {
    pub(crate) const EXTRATOR: u8 = 1;
    pub(crate) const CONSTRUCTOR: u8 = 2;
}

#[derive(Clone)]
pub enum Access {
    AppleType { access: Ident, def: Item },
    DeclExtern { access: Ident, def: Item },
    ApplyExtractor { access: Ident, def: Item },
    ExtractVar { access: Ident, def: Item },
    ApplyConst { access: Ident, def: Item },
    ImplExtractor { access: Ident, def: Item },
}

impl Access {
    pub fn def_item(&self) -> &Item {
        match self {
            Access::AppleType { def, .. } => def,
            Access::DeclExtern { def, .. } => def,
            Access::ApplyExtractor { def, .. } => def,
            Access::ExtractVar { def, .. } => def,
            Access::ApplyConst { def, .. } => def,
            Access::ImplExtractor { def, .. } => def,
        }
    }
}

impl Access {
    pub(crate) fn access_def_loc(&self) -> (Pos, Pos) {
        match self {
            Access::AppleType { access, def } => (access.1, def.def_loc()),
            Access::DeclExtern { access, def } => (access.1, def.def_loc()),
            Access::ApplyExtractor { access, def } => (access.1, def.def_loc()),
            Access::ApplyConst { access, def } => (access.1, def.def_loc()),
            Access::ExtractVar { access, def } => (access.1, def.def_loc()),
            Access::ImplExtractor { access, def } => (access.1, def.def_loc()),
        }
    }
}

impl std::fmt::Display for ItemOrAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemOrAccess::Item(item) => write!(f, "{}", item),
            ItemOrAccess::Access(acc) => write!(f, "{}", acc),
        }
    }
}

impl std::fmt::Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Item::Type { ty } => write!(f, "item_type:{}", ty.name.0.as_str()),
            Item::Decl { decl, kind: _ } => write!(f, "item_decl:{}", decl.term.0.as_str()),
            Item::Dummy => write!(f, "dummy"),
            Item::Const { name, ty: _ } => write!(f, "item_const:{}", name.0.as_str()),
            Item::Var { name, ty: _ } => write!(f, "item_var:{}", name.0.as_str()),
        }
    }
}

impl std::fmt::Display for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Access::AppleType { access, def } => write!(f, "apply type {}->{}", access.0, def),
            Access::DeclExtern { access, def } => write!(f, "decl extern {}->{}", access.0, def),
            Access::ApplyExtractor { access, def } => {
                write!(f, "apply extrator {}->{}", access.0, def)
            }
            Access::ExtractVar { access, def } => write!(f, "extract var {}->{}", access.0, def),
            Access::ApplyConst { access, def } => write!(f, "apply const {}->{}", access.0, def),
            Access::ImplExtractor { access, def } => {
                write!(f, "impl extractor {}->{}", access.0, def)
            }
        }
    }
}
