use std::path::Path;

use cranelift_isle::{ast::*, lexer::Lexer, parser};

use super::comment::CommentExtrator;

fn fmt(path: impl AsRef<Path>) -> String {
    unimplemented!()
}

#[derive(Default, Clone)]
struct Fmt {
    result: String,
}

impl Fmt {
    fn new() -> Self {
        Self::default()
    }
    fn format_defs(&mut self, d: &Vec<Def>) {
        for d in d.iter() {
            self.format_def(d);
        }
    }

    fn format_def(&mut self, d: &Def) {
        match d {
            Def::Pragma(x) => self.format_pragma(x),
            Def::Type(x) => self.format_type(x),
            Def::Rule(x) => self.format_rule(x),
            Def::Extractor(x) => self.format_extractor(x),
            Def::Decl(x) => self.format_decl(x),
            Def::Extern(x) => self.format_extern(x),
            Def::Converter(x) => self.format_converter(x),
        }
    }
    fn format_pragma(&mut self, d: &Pragma) {
        // nothing here.
    }
    fn format_type(&mut self, d: &Type) {
        //
        self.result.push_str(format!("(type {}", d.name.0).as_str());
        if d.is_extern {
            self.result.push_str(" extern");
        }
        if d.is_nodebug {
            self.result.push_str(" nodebug");
        }

        match &d.ty {
            TypeValue::Primitive(name, _) => self.result.push_str(name.0.as_str()),
            TypeValue::Enum(vs, _) => {
                self.result
                    .push_str(format!("\n{}(enum\n", Self::ident(1)).as_str());
                for v in vs.iter() {
                    self.result
                        .push_str(format!("{}{}(\n", Self::ident(1), v.name.0,).as_str());
                    for f in v.fields.iter() {
                        self.result.push_str(
                            format!("{}({} {})\n", Self::ident(2), f.name.0, f.ty.0).as_str(),
                        );
                    }
                    self.result
                        .push_str(format!("{})\n", Self::ident(1)).as_str());
                }
                self.result.push_str(")");
            }
        }
        self.result.push_str(")\n");
    }
    fn format_rule(&mut self, d: &Rule) {}
    fn format_extractor(&mut self, d: &Extractor) {}
    fn format_decl(&mut self, d: &Decl) {}
    fn format_extern(&mut self, d: &Extern) {}
    fn format_converter(&mut self, d: &Converter) {}

    fn ident(n: usize) -> String {
        "  ".to_string().repeat(n)
    }
}

#[test]
fn test_enum() {
    let mut l = Lexer::from_str(
        r#"

    (type RangeView extern
        (enum
   
          (Empty)
   
          (NonEmpty (index usize) (rest Range))))
    
        "#,
        "",
    )
    .unwrap();
    let a = parser::parse(l).unwrap();
    let mut f = Fmt::new();
    f.format_defs(&a.defs);
    eprintln!("{}", f.result);
}
