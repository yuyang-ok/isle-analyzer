use cranelift_isle::{
    ast::{Ident, Type},
    lexer::Pos,
};
use lsp_types::Location;
use std::path::*;

pub trait GetPosition {
    fn get_position(
        &self,
    ) -> (
        url::Url,
        u32, // line zero-based
        u32, // column zero-based
    );
}

impl dyn GetPosition {
    pub(crate) fn in_range(&self, range: &Location) -> bool {
        let (filepath, line, col) = self.get_position();
        if filepath != range.uri {
            return false;
        }
        if line < range.range.start.line {
            return false;
        }
        if line == range.range.start.line && col < range.range.start.character {
            return false;
        }
        if line > range.range.end.line {
            return false;
        }
        if line == range.range.end.line && col > range.range.end.character {
            return false;
        }
        true
    }
}
/// Path concat from
pub fn path_concat(p1: &Path, p2: &Path) -> PathBuf {
    let p2: Vec<_> = p2.components().collect();
    let is_abs = match p2.get(0).unwrap() {
        Component::RootDir | Component::Prefix(_) => true,
        _ => false,
    };
    let mut p1: Vec<_> = p1.components().collect();
    normal_path_components(if is_abs {
        &p2
    } else {
        {
            p1.extend(p2);
            &p1
        }
    })
}

pub fn normal_path_components<'a>(x: &Vec<Component<'a>>) -> PathBuf {
    let mut ret = PathBuf::new();
    for v in x {
        match v {
            Component::Prefix(x) => ret.push(x.as_os_str()),
            Component::RootDir => ret.push("/"),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = ret.pop();
            }
            Component::Normal(x) => ret.push(*x),
        }
    }
    if ret.to_str().unwrap() == "" {
        ret.push(".")
    }
    ret
}

impl GetPosition for Location {
    fn get_position(
        &self,
    ) -> (
        url::Url,
        u32, // line zero-based
        u32, // column zero-based
    ) {
        if self.range.start.line == self.range.end.line {
            return (
                self.uri.clone(),
                self.range.start.line,
                (self.range.start.character + self.range.end.character) / 2,
            );
        } else {
            return (
                self.uri.clone(),
                (self.range.start.line + self.range.end.line) / 2,
                0,
            );
        }
    }
}

pub trait GetPosAndLength {
    fn get_pos_and_len(&self) -> (Pos, u32);
}

impl GetPosAndLength for Ident {
    fn get_pos_and_len(&self) -> (Pos, u32) {
        (self.1, self.0.len() as u32)
    }
}

impl GetPosAndLength for (Pos, u32) {
    fn get_pos_and_len(&self) -> (Pos, u32) {
        self.clone()
    }
}

impl GetPosAndLength for (Pos, usize) {
    fn get_pos_and_len(&self) -> (Pos, u32) {
        (self.0.clone(), self.1 as u32)
    }
}

impl GetPosAndLength for Type {
    fn get_pos_and_len(&self) -> (Pos, u32) {
        (self.name.1, self.name.0.len() as u32)
    }
}
