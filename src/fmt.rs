#![allow(dead_code)]

use std::{cell::RefCell, path::Path};

use cranelift_isle::{ast::*, error::Errors, lexer::Lexer, parser};

fn fmt(path: impl AsRef<Path>) -> Result<String, Errors> {
    let path = path.as_ref();
    let lexer = Lexer::from_files(vec![path])?;

    let ast = parser::parse(lexer)?;

    unimplemented!()
}

#[derive(Clone)]
struct Fmt {
    result: RefCell<String>,
    d: Vec<Def>,
}

impl Fmt {
    fn new(d: Vec<Def>) -> Self {
        Self {
            result: Default::default(),
            d,
        }
    }
    fn new_line(&self) {
        unimplemented!()
    }
    fn format_defs(self) -> String {
        for d in self.d.iter() {
            self.format_def(d);
            self.new_line();
        }
        self.result.into_inner()
    }

    fn format_def(&self, d: &Def) {
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
    fn format_pragma(&self, _d: &Pragma) {
        // nothing here.
        unreachable!("rightnow ISLE has no Pragma")
    }
    fn format_type(&self, d: &Type) {
        //
        self.push_str(format!("(type {}", d.name.0).as_str());
        if d.is_extern {
            self.push_str(" extern");
        }

        if d.is_nodebug {
            self.push_str(" nodebug");
        }
        match &d.ty {
            TypeValue::Primitive(name, _) => self.push_str(name.0.as_str()),
            TypeValue::Enum(vs, _) => {
                self.push_str(format!("\n{}(enum\n", Self::ident(1)).as_str());
                for v in vs.iter() {
                    self.push_str(format!("{}{}(\n", Self::ident(1), v.name.0,).as_str());
                    for f in v.fields.iter() {
                        self.push_str(
                            format!("{}({} {})\n", Self::ident(2), f.name.0, f.ty.0).as_str(),
                        );
                    }
                    self.push_str(format!("{})\n", Self::ident(1)).as_str());
                }
                self.push_str(")");
            }
        }
        self.push_str(")\n");
    }

    fn format_rule(&self, r: &Rule) {
        self.format_pattern(&r.pattern);
    }
    fn format_pattern(&self, p: &Pattern) {
        match p {
            Pattern::Var { var, pos } => self.push_str(&var.0),
            Pattern::BindPattern { var, subpat, pos } => {
                self.push_str("(");
                self.push_str(var.0.as_str());
                self.push_str(" ");

                self.push_str(")");
            }
            Pattern::ConstInt { val, pos } => todo!(),
            Pattern::ConstPrim { val, pos } => todo!(),
            Pattern::Term { sym, args, pos } => todo!(),
            Pattern::Wildcard { pos } => todo!(),
            Pattern::And { subpats, pos } => todo!(),
            Pattern::MacroArg { index, pos } => todo!(),
        }
    }
    fn format_extractor(&self, e: &Extractor) {
        self.push_str("(extractor ");
        {
            // pattern
            self.push_str("(");
            self.push_str(&e.term.0);
            self.push_str(" ");
            let last = e.args.len() - 1;
            for (index, e) in e.args.iter().enumerate() {
                self.push_str(&e.0);
                if index != last {
                    self.push_str(" ");
                }
            }
            self.push_str(")");
        }
        self.format_pattern(&e.template);
        self.push_str(")");
    }
    fn format_decl(&self, d: &Decl) {
        self.push_str("(rule ");
        if d.pure {
            self.push_str("pure ");
        }
        if d.multi {
            self.push_str("multi ");
        }
        if d.partial {
            self.push_str("partial ");
        }
        self.push_str(d.term.0.as_str());
        self.push_str(" ");
        {
            //args
            self.push_str("(");
            if d.arg_tys.len() > 0 {
                let last = d.arg_tys.len() - 1;
                for (index, a) in d.arg_tys.iter().enumerate() {
                    self.push_str(a.0.as_str());
                    if index != last {
                        self.push_str(",");
                    }
                }
            }
            self.push_str(")");
        }
        self.push_str(" ");
        self.push_str(d.ret_ty.0.as_str());
        self.push_str(")");
    }

    fn format_extern(&self, e: &Extern) {
        self.push_str("(extern ");
        match e {
            Extern::Extractor {
                term,
                func,
                pos,
                infallible,
            } => {
                if *infallible {
                    self.push_str("infallible ");
                }
                self.push_str("extractor ");
                self.push_str(term.0.as_str());
                self.push_str(" ");
                self.push_str(func.0.as_str());
            }

            Extern::Constructor { term, func, pos } => {
                self.push_str("constructor ");
                self.push_str(term.0.as_str());
                self.push_str(" ");
                self.push_str(func.0.as_str());
            }
            Extern::Const { name, ty, pos } => {
                self.push_str(name.0.as_str());
                self.push_str(" ");
                self.push_str(ty.0.as_str());
            }
        }
        self.push_str(")");
    }
    fn format_converter(&self, c: &Converter) {
        self.push_str("(convertor ");
        self.push_str(c.term.0.as_str());
        self.push_str(" ");
        self.push_str(c.inner_ty.0.as_str());
        self.push_str(" ");
        self.push_str(c.outer_ty.0.as_str());
        self.push_str(")");
    }

    fn ident(n: usize) -> String {
        "  ".to_string().repeat(n)
    }

    fn push_str(&self, s: impl AsRef<str>) {
        let s = s.as_ref();
        self.result.borrow_mut().push_str(s);
    }
    fn last_line_length(&self) -> usize {
        self.result
            .borrow()
            .lines()
            .last()
            .map(|x| x.len())
            .unwrap_or(0)
    }
}

#[test]
fn test_fmt() {
    use cranelift_isle::lexer::Lexer;
    use cranelift_isle::parser::parse;
    let l = Lexer::from_str(
        r#"

    (type RangeView extern
        (enum
   
          (Empty)
   
          (NonEmpty (index usize) (rest Range))))
    
        "#,
        "",
    )
    .unwrap();
    let a = parse(l).unwrap();
    let mut f = Fmt::new(a.defs);
    let result = f.format_defs();
    eprintln!("{}", result);
}
