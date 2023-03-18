use std::{cmp::Ordering, collections::HashMap};

use cranelift_isle::{lexer::Pos};

#[derive(Default)]
pub struct CommentExtrator {
    comments: Vec<Comment>,
}

impl CommentExtrator {
    pub(crate) fn new(content: &str) -> Self {
        if content.len() == 0 {
            return Self::default();
        }
        enum State {
            Init,
            OneSemiColon,
            Comment,
        }
        impl Default for State {
            fn default() -> Self {
                Self::Init
            }
        }
        let mut line = 0;
        let mut col = 0;
        let mut state = State::default();
        const NEW_LINE: u8 = 10;
        const SEMI_COLON: u8 = 59;
        let mut comments = Vec::new();
        let mut comment = String::new();
        let last_index = content.as_bytes().len() - 1;
        for (index, c) in content.as_bytes().iter().enumerate() {
            match state {
                State::Init => match *c {
                    NEW_LINE => {
                        line += 1;
                        col = 0;
                    }
                    SEMI_COLON => {
                        state = State::OneSemiColon;
                        col += 1;
                    }
                    _ => {
                        col += 1;
                    }
                },
                State::OneSemiColon => {
                    if *c == SEMI_COLON {
                        state = State::Comment;
                    } else {
                        state = State::Init;
                    }
                    col += 1;
                }
                State::Comment => {
                    if *c == NEW_LINE || index == last_index {
                        if *c != NEW_LINE {
                            comment.push(*c as char);
                        }
                        // ending
                        let col_ = col - (comment.len() as u32);
                        comments.push(Comment {
                            line,
                            col: col_,
                            content: comment.clone(),
                        });
                        line += 1;
                        col = 0;
                        comment = String::new();
                        state = State::Init;
                    } else if *c == SEMI_COLON {
                        // nothing.
                        col += 1;
                    } else {
                        comment.push(*c as char);
                        col += 1;
                    }
                }
            };
        }
        Self { comments }
    }
}

pub struct Comment {
    pub(crate) line: u32,
    pub(crate) col: u32,
    pub(crate) content: String,
}

pub struct DocumentComments {
    comments: HashMap<Pos, String>,
}

impl DocumentComments {
    pub fn new(extractor: &CommentExtrator, pos: &Vec<Pos>) -> Self {
        enum PosOrComment<'a> {
            Pos(Pos),
            Comment(&'a Comment),
        }
        impl<'a> PosOrComment<'a> {
            fn get_line(&self) -> u32 {
                match self {
                    PosOrComment::Pos(x) => x.line as u32,
                    PosOrComment::Comment(x) => (*x).line,
                }
            }
        }
        impl From<&Pos> for PosOrComment<'_> {
            fn from(value: &Pos) -> Self {
                Self::Pos(value.clone())
            }
        }
        impl<'a> From<&'a Comment> for PosOrComment<'a> {
            fn from(value: &'a Comment) -> Self {
                Self::Comment(value)
            }
        }
        // first sort.
        let mut s = Vec::with_capacity(pos.len() + extractor.comments.len());
        pos.iter().for_each(|x| s.push(PosOrComment::from(x)));
        extractor
            .comments
            .iter()
            .for_each(|x| s.push(PosOrComment::from(x)));
        s.sort_by(|a, b| {
            if a.get_line() > b.get_line() {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        });
        let mut tmp = Vec::new();
        fn make_document_symbol_comment(x: &Vec<&Comment>) -> String {
            let mut ret = String::new();

            for (index, c) in x.iter().enumerate() {
                let kill_ret = if index > 0 {
                    x.get(index - 1)
                        .map(|x| c.line - x.line > 1)
                        .unwrap_or(false)
                } else {
                    false
                };
                if kill_ret {
                    ret = String::new();
                }
                let s = {
                    let mut c = (*c).content.trim().to_string();
                    c.push_str("\n");
                    c
                };
                ret.push_str(s.as_str());
            }
            ret
        }
        let mut comments = HashMap::new();
        s.iter().for_each(|x| match x {
            PosOrComment::Pos(x) => {
                comments.insert(x.clone(), make_document_symbol_comment(&tmp));
                tmp = Vec::new();
            }
            PosOrComment::Comment(s) => {
                tmp.push(*s);
            }
        });
        Self { comments }
    }

    pub(crate) fn get_comment(&self, p: &Pos) -> Option<&String> {
        self.comments.get(p)
    }
}

#[test]
fn test_document_comments() {
    // Include a test from
    let content = include_str!("comment_test.txt");
    let e = CommentExtrator::new(content);
    let first_pos = Pos {
        file: 0,
        offset: 0, // offset here is not used.
        line: 3,
        col: 0,
    };
    let second_pos = Pos {
        file: 0,
        offset: 0, // offset here is not used.
        line: 6,
        col: 0,
    };
    let third_pos = Pos {
        file: 0,
        offset: 0, // offset here is not used.
        line: 9,
        col: 0,
    };
    let d = DocumentComments::new(&e, &vec![first_pos, second_pos, third_pos]);
    assert_eq!(d.comments.get(&first_pos).unwrap(), "hello\nworld\n");
    assert_eq!(d.comments.get(&second_pos).unwrap(), "aaa\nbbb\n");
    assert_eq!(d.comments.get(&third_pos).unwrap(), "cccc\n");
}
