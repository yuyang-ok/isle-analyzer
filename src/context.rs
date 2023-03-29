use super::project::Project;
use lsp_server::*;

pub struct Context {
    /// lsp connection.
    pub connection: Connection,
    /// loaded project.
    pub project: Project,
}
