use cranelift_isle::lexer::Pos;

use clap::Parser;
use crossbeam::channel::select;
use isle_analyzer::reload;
use isle_analyzer::{
    completion::on_completion_request, context::*, document_symbol, goto_definition, hover,
    inlay_hitnt, project::Project, references, rename::on_rename, semantic_tokens, show_rust_code,
};
use log::*;
use lsp_types::notification::Notification;
use lsp_types::request::Request;
use lsp_types::*;
use std::collections::HashMap;

use std::path::*;
use std::str::FromStr;
struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Error
    }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!("{} - {}", record.level(), record.args());
        }
    }
    fn flush(&self) {}
}
const LOGGER: SimpleLogger = SimpleLogger;

pub fn init_log() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Error))
        .unwrap()
}
use lsp_server::*;

#[derive(Parser)]
#[clap(author, version, about)]
struct Options {}

fn main() {
    Options::parse();

    init_log();
    // stdio is used to communicate Language Server Protocol requests and responses.
    // stderr is used for logging (and, when Visual Studio Code is used to communicate with this
    // server, it captures this output in a dedicated "output channel").
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .to_string();
    log::error!(
        "Starting language server '{}' communicating via stdio...",
        exe
    );

    let (connection, io_threads) = Connection::stdio();

    let mut context = Context {
        connection,
        project: Project::empty(),
    };

    let (id, _client_response) = context
        .connection
        .initialize_start()
        .expect("could not start connection initialization");

    let capabilities = serde_json::to_value(lsp_types::ServerCapabilities {
        // The server receives notifications from the client as users open, close,
        // and modify documents.
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                will_save: None,
                will_save_wait_until: None,
                save: Some(
                    SaveOptions {
                        include_text: Some(true),
                    }
                    .into(),
                ),
            },
        )),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: None,
            trigger_characters: Some({
                let mut c = vec![".".to_string()];
                for x in 'a'..='z' {
                    c.push(String::from(x as char));
                }
                for x in 'A'..='Z' {
                    c.push(String::from(x as char));
                }
                c
            }),
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
            completion_item: None,
        }),
        rename_provider: Some(OneOf::Left(true)),
        definition_provider: Some(OneOf::Left(true)),
        type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
        references_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        inlay_hint_provider: Some(OneOf::Left(true)),
        semantic_tokens_provider: Some(
            lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
                lsp_types::SemanticTokensOptions {
                    range: Some(true),
                    full: None,
                    ..Default::default()
                },
            ),
        ),
        ..Default::default()
    })
    .expect("could not serialize server capabilities");
    context
        .connection
        .initialize_finish(
            id,
            serde_json::json!({
                "capabilities": capabilities,
            }),
        )
        .expect("could not finish connection initialization");

    loop {
        select! {
            recv(context.connection.receiver) -> message => {
                match message {
                    Ok(Message::Request(request)) => on_request(&mut context, &request),
                    Ok(Message::Response(response)) => on_response(&context, &response),
                    Ok(Message::Notification(notification)) => {
                        match notification.method.as_str() {
                            lsp_types::notification::Exit::METHOD => break,
                            lsp_types::notification::Cancel::METHOD => {

                            }
                            _ => on_notification(&mut context, &notification   ),
                        }
                    }
                    Err(error) => log::error!("IDE lsp client message error: {:?}", error),
                }
            }
        };
    }
    io_threads.join().expect("I/O threads could not finish");
    log::error!("Shut down language server '{}'.", exe);
}

fn on_request(context: &mut Context, request: &lsp_server::Request) {
    log::error!("receive method:{}", request.method.as_str());
    match request.method.as_str() {
        lsp_types::request::Completion::METHOD => on_completion_request(context, request),
        lsp_types::request::GotoDefinition::METHOD => {
            goto_definition::on_go_to_def_request(context, request);
        }
        lsp_types::request::References::METHOD => {
            references::on_references_request(context, request);
        }
        lsp_types::request::HoverRequest::METHOD => {
            hover::on_hover_request(context, request);
        }
        lsp_types::request::DocumentSymbolRequest::METHOD => {
            document_symbol::on_document_symbol_request(context, request);
        }
        lsp_types::request::SemanticTokensFullRequest::METHOD => {
            semantic_tokens::on_senantic_tokens(context, request);
        }
        lsp_types::request::InlayHintRequest::METHOD => {
            inlay_hitnt::on_inlay_hints(context, request);
        }
        lsp_types::request::Rename::METHOD => {
            on_rename(context, request);
        }
        "isle/reload" => {
            reload::on_reload(context, request);
            send_diag(context);
        }
        "isle/show_compiled_code" => {
            show_rust_code::on_show_compiled_code(context, request);
        }
        _ => log::error!("handle request '{}' from client", request.method),
    }
}

fn on_response(_context: &Context, _response: &Response) {
    log::error!("handle response from client");
}

fn on_notification(context: &mut Context, notification: &lsp_server::Notification) {
    match notification.method.as_str() {
        lsp_types::notification::DidChangeTextDocument::METHOD => {
            let parameters =
                serde_json::from_value::<DidChangeTextDocumentParams>(notification.params.clone())
                    .expect("could not deserialize DidChangeTextDocumentParams request");
            let fpath = parameters.text_document.uri.to_file_path().unwrap();
            update_defs(
                context,
                &fpath,
                parameters.content_changes.last().unwrap().text.as_str(),
            );
        }
        lsp_types::notification::DidSaveTextDocument::METHOD => {
            let parameters =
                serde_json::from_value::<DidSaveTextDocumentParams>(notification.params.clone())
                    .expect("could not deserialize DidChangeTextDocumentParams request");

            let fpath = parameters.text_document.uri.to_file_path().unwrap();
            update_defs(
                context,
                &fpath,
                std::fs::read_to_string(fpath.as_path()).unwrap().as_str(),
            );
            send_diag(context);
        }
        _ => log::error!("handle request '{}' from client", notification.method),
    }
}

fn update_defs(context: &mut Context, fpath: &PathBuf, content: &str) {
    match context.project.update_defs(&fpath, content) {
        Ok(_) => {}
        Err(err) => log::error!("update_def failed,err:{:?}", err),
    };
}

fn send_diag(context: &mut Context) {
    let files = context
        .project
        .get_filenames()
        .iter()
        .map(|x| PathBuf::from_str(x.as_ref()).unwrap())
        .collect::<Vec<_>>();
    match cranelift_isle::compile::from_files(
        &files,
        &cranelift_isle::codegen::CodegenOptions {
            exclude_global_allow_pragmas: false,
        },
    ) {
        Ok(_) => {
            for f in files.iter() {
                let ds = lsp_types::PublishDiagnosticsParams::new(
                    Url::from_file_path(f).unwrap(),
                    vec![],
                    None,
                );
                context
                    .connection
                    .sender
                    .send(lsp_server::Message::Notification(
                        lsp_server::Notification {
                            method: format!(
                                "{}",
                                lsp_types::notification::PublishDiagnostics::METHOD
                            ),
                            params: serde_json::to_value(ds).unwrap(),
                        },
                    ))
                    .unwrap();
            }
        }
        Err(err) => {
            use cranelift_isle::error::Error::*;
            #[derive(Default)]
            struct Diags {
                m: HashMap<PathBuf, Vec<Diagnostic>>,
            }
            impl Diags {
                fn insert(&mut self, p: PathBuf, d: Diagnostic) {
                    if let Some(xxx) = self.m.get_mut(&p) {
                        xxx.push(d);
                    } else {
                        self.m.insert(p, vec![d]);
                    }
                }
                fn mk_empty(&mut self, p: PathBuf) {
                    if let Some(_) = self.m.get_mut(&p) {
                    } else {
                        self.m.insert(p, vec![]);
                    }
                }
            }
            let mut diags = Diags::default();
            for e in err.errors.iter() {
                match e {
                    IoError {
                        error: _,
                        context: _,
                    } => {
                        // TODO
                    }
                    cranelift_isle::error::Error::ParseError { msg, span } => {
                        let file = files[span.to.file].clone();
                        let d = Diagnostic {
                            range: Range {
                                start: pos_to_position(span.from),
                                end: pos_to_position(span.to),
                            },
                            message: format!("{}", msg),
                            ..Default::default()
                        };
                        diags.insert(file, d);
                    }
                    TypeError { msg, span } => {
                        let file = files[span.to.file].clone();
                        let d = Diagnostic {
                            range: Range {
                                start: pos_to_position(span.from),
                                end: pos_to_position(span.to),
                            },
                            message: format!("{}", msg),
                            ..Default::default()
                        };
                        diags.insert(file, d);
                    }
                    UnreachableError { msg, span } => {
                        let file = files[span.to.file].clone();
                        let d = Diagnostic {
                            range: Range {
                                start: pos_to_position(span.from),
                                end: pos_to_position(span.to),
                            },
                            message: format!("{}", msg),
                            ..Default::default()
                        };
                        diags.insert(file, d);
                    }
                    OverlapError { msg, rules } => {
                        for r in rules.iter() {
                            let file = files[r.to.file].clone();
                            let d = Diagnostic {
                                range: Range {
                                    start: pos_to_position(r.from),
                                    end: pos_to_position(r.to),
                                },
                                message: format!("{}", msg),
                                ..Default::default()
                            };
                            diags.insert(file, d);
                        }
                    }
                    ShadowedError { shadowed, mask } => {
                        for r in shadowed.iter().chain(vec![mask]) {
                            let file = files[r.to.file].clone();
                            let d = Diagnostic {
                                range: Range {
                                    start: pos_to_position(r.from),
                                    end: pos_to_position(r.to),
                                },
                                message: format!("{}","The rules can never match because another rule will always match first."),
                                ..Default::default()
                            };
                            diags.insert(file, d);
                        }
                    }
                };
            }

            for f in files.iter() {
                if diags.m.get(f).map(|x| x.len()).unwrap_or(0) == 0 {
                    diags.mk_empty(f.clone());
                }
            }
            for (k, v) in diags.m.into_iter() {
                context
                    .connection
                    .sender
                    .send(lsp_server::Message::Notification(
                        lsp_server::Notification {
                            method: format!(
                                "{}",
                                lsp_types::notification::PublishDiagnostics::METHOD
                            ),
                            params: serde_json::to_value(PublishDiagnosticsParams {
                                uri: Url::from_file_path(k).unwrap(),
                                diagnostics: v,
                                version: None,
                            })
                            .unwrap(),
                        },
                    ))
                    .unwrap();
            }
        }
    };
}

fn pos_to_position(x: Pos) -> Position {
    Position {
        line: (x.line - 1) as u32,
        character: x.col as u32,
    }
}
