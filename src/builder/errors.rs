/// Any errors that can occur when interacting with the NodeManager
#[derive(Debug, thiserror::Error)]
pub enum Error {
    // An error from the NodeManager
    #[error(transparent)]
    NodeManager(#[from] crate::builder::node_manager::Error),

    /// An error from the esbuild API
    #[error(transparent)]
    ESBuild(#[from] crate::builder::esbuild::Error),
}
