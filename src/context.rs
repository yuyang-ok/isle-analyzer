use super::project::Project;
use lsp_server::*;

pub struct Context {
    pub connection: Connection,
    pub project: Project,
}
