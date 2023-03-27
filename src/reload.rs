use super::context::*;
use crate::project::Project;
use lsp_server::*;
use std::{path::PathBuf, str::FromStr};

/// Handles go-to-def request of the language server.
pub fn on_reload(context: &mut Context, request: &Request) {
    let req = serde_json::from_value::<Req>(request.params.clone())
        .expect("could not deserialize go-to-def request");
    let send_err = |context: &Context, msg: String| {
        let r = Response::new_err(request.id.clone(), ErrorCode::UnknownErrorCode as i32, msg);
        context
            .connection
            .sender
            .send(Message::Response(r))
            .unwrap();
    };
    let req = match req.to_path_buf() {
        Ok(x) => x,
        Err(_) => {
            send_err(context, "file paths not ok".to_string());
            return;
        }
    };
    let p = match Project::new(req) {
        Ok(x) => x,
        Err(err) => {
            send_err(context, format!("load project failed,err:{:?}", err));
            return;
        }
    };
    context.project = p;
    context
        .connection
        .sender
        .send(Message::Response(Response {
            id: request.id.clone(),
            result: None,
            error: None,
        }))
        .unwrap();
}

#[derive(Clone, Default, serde::Deserialize)]
struct Req {
    files: Vec<String>,
}

impl Req {
    fn to_path_buf(self) -> Result<Vec<PathBuf>, ()> {
        let mut v = Vec::with_capacity(self.files.len());
        for p in self.files.into_iter() {
            v.push(match PathBuf::from_str(p.as_str()) {
                Ok(x) => x,
                Err(_) => return Result::Err(()),
            })
        }
        Result::Ok(v)
    }
}
