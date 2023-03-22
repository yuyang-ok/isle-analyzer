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
    Type {
        ty: Type,
    },
    Decl {
        decl: Decl,
        kind: DeclKind,
    },
    Dummy,
    Const {
        name: Ident,
        /// The location define the type.
        ty: Ident,
    },
    Var {
        name: Ident,
        // the location define the Type.
        ty: Ident,
    },
    EnumMemberName {
        name: Ident,
    },
    EnumMemberField {
        name: Ident,
    },
    EnumVariant {
        v: Variant,
    },
}

impl Item {
    pub(crate) fn item_is_ty(&self) -> Option<&Type> {
        match self {
            Item::Type { ty } => Some(ty),
            _ => None,
        }
    }
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

lazy_static! {
    pub static ref UNKNOWN_TYPE: Ident = Ident("".to_string(), UNKNOWN_POS);
}

impl Item {
    pub(crate) fn def_loc(&self) -> Pos {
        match self {
            Item::Type { ty } => ty.pos,
            Item::Decl { decl, kind: _ } => decl.term.1,
            Item::Dummy => UNKNOWN_POS,
            Item::Const { name, ty: _ } => name.1,
            Item::Var { name, ty: _ } => name.1,
            Item::EnumMemberName { name } => name.1,
            Item::EnumMemberField { name } => name.1,
            Item::EnumVariant { v } => v.name.1,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccessKind {
    AppleType,
    DeclExtern,
    ApplyEORC,
    ExtractVar,
    ApplyConst,
    ImplExtractor,
    ImplConstructor,
    ApplyVariant,
    ApplyVar,
}

impl AccessKind {
    fn to_static_str(self) -> &'static str {
        match self {
            AccessKind::AppleType => "apply type",
            AccessKind::DeclExtern => "decl extern",
            AccessKind::ApplyEORC => "apply extrator",
            AccessKind::ExtractVar => "extract var",
            AccessKind::ApplyConst => "apply const",
            AccessKind::ImplExtractor => "impl extractor",
            AccessKind::ApplyVariant => "apply enum member",
            AccessKind::ApplyVar => "apply var",
            AccessKind::ImplConstructor => "impl constructor",
        }
    }
}

#[derive(Clone)]
pub struct Access {
    pub(crate) access: Ident,
    pub(crate) def: Item,
    pub(crate) kind: AccessKind,
}

impl Access {
    pub fn def_item(&self) -> &Item {
        &self.def
    }
}

impl Access {
    pub(crate) fn access_def_loc(&self) -> (Pos, Pos) {
        (self.access.1, self.def.def_loc())
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
            Item::EnumMemberName { name } => write!(f, "enum_member:{}", name.0.as_str()),
            Item::EnumMemberField { name } => write!(f, "enum_field:{}", name.0.as_str()),
            Item::EnumVariant { v } => write!(f, "enum_variant:{}", v.name.0),
        }
    }
}

impl std::fmt::Display for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let access = &self.access;
        let def = &self.def;
        write!(f, "{} {}->{}", self.kind.to_static_str(), access.0, def)
    }
}
