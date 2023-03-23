use super::goto_definition;
use super::project::*;
use super::*;
use crate::utils::*;
use log::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!("{} - {}", record.level(), record.args());
        }
    }
    fn flush(&self) {}
}

pub fn init_log() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Trace))
        .unwrap()
}

const LOGGER: SimpleLogger = SimpleLogger;

fn path_to_abs(s: &str) -> PathBuf {
    let x = path_concat(
        std::env::current_dir().unwrap().as_path(),
        PathBuf::from_str(s).unwrap().as_path(),
    );
    x
}

#[test]
fn goto_definition() {
    let file = path_to_abs("./tests/bound_var.isle");
    let p = Project::new(vec![file.clone()]).unwrap();
    let mut handler =
        goto_definition::Handler::new(url::Url::from_file_path(file.clone()).unwrap(), 2, 9);
    p.run_visitor_for_file(&file, &mut handler);
    eprintln!("-----> {:?}", readable_location(&handler.result.unwrap()));
}

#[test]
fn xxx() {
    let x = "hello".to_string();
    let mut m = HashMap::new();
    m.insert(x, ());
    let y = "hello".to_string();
    m.insert(y, ());
    eprintln!("{}", m.len())
}

#[test]
fn completion() {
    init_log();
    let file = path_to_abs("./tests/bound_var.isle");
    let p = Project::new(vec![file.clone()]).unwrap();
    let mut handler =
        completion::Handler::new(url::Url::from_file_path(file.clone()).unwrap(), 3, 9);
    p.run_visitor_for_file(&file, &mut handler);

    for x in handler.result.unwrap() {
        eprintln!("xxxx:{:?}->{:?}", x.label, x.kind)
    }
}
