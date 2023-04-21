use super::context::*;
use super::send_err;
use crate::project::Project;
use lsp_server::*;
use std::{path::PathBuf, str::FromStr};

/// Handles custom `reload` project form client
pub fn on_reload(context: &mut Context, request: &Request) {
    let req = serde_json::from_value::<Req>(request.params.clone())
        .expect("could not deserialize reload request");
    let req = match req.to_path_buf() {
        Ok(x) => x,
        Err(_) => {
            send_err(context, "file paths not ok".to_string(), request.id.clone());
            return;
        }
    };
    let p = match Project::new(req) {
        Ok(x) => x,
        Err(err) => {
            send_err(
                context,
                format!("load project failed,err:{:?}", err),
                request.id.clone(),
            );
            return;
        }
    };
    context.project = p;
    let r = Response::new_ok(request.id.clone(), serde_json::to_value("Load Ok").unwrap());
    context
        .connection
        .sender
        .send(Message::Response(Response {
            id: request.id.clone(),
            result: Some(serde_json::to_value(r).unwrap()),
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
