use crossbeam::channel::bounded;
use crossbeam::channel::select;

use isle_analyzer::completion::on_completion_request;
use isle_analyzer::context::*;
use isle_analyzer::document_symbol;
use isle_analyzer::goto_definition;
use isle_analyzer::hover;
use isle_analyzer::inlay_hitnt;
use isle_analyzer::project::Project;
use isle_analyzer::references;
use isle_analyzer::semantic_tokens;
use log::*;
use lsp_types::notification::Notification;
use lsp_types::request::Request;
use lsp_types::*;
use std::path::*;
use std::sync::{Arc, Mutex};

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

fn main() {
    for _args in std::env::args().into_iter() {
        // todo handle this.
    }
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
        project: Project::from_walk().unwrap(),
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
    let (diag_sender, diag_receiver) = bounded::<(PathBuf, ())>(1);
    let _diag_sender = Arc::new(Mutex::new(diag_sender));

    loop {
        select! {
            recv(diag_receiver) -> _message => {

            }
            recv(context.connection.receiver) -> message => {
                match message {
                    Ok(Message::Request(request)) => on_request(&mut context, &request),
                    Ok(Message::Response(response)) => on_response(&context, &response),
                    Ok(Message::Notification(notification)) => {
                        match notification.method.as_str() {
                            lsp_types::notification::Exit::METHOD => break,
                            lsp_types::notification::Cancel::METHOD => {
                                // TODO: Currently the server does not implement request cancellation.
                                // It ought to, especially once it begins processing requests that may
                                // take a long time to respond to.
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
            match context.project.update_defs(
                &fpath,
                parameters.content_changes.last().unwrap().text.as_str(),
            ) {
                Ok(_) => {}
                Err(err) => log::error!("update_def failed,err:{:?}", err),
            };
        }
        _ => log::error!("handle request '{}' from client", notification.method),
    }
}
