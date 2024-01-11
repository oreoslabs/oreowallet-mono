use thiserror::Error;

#[derive(Debug, Error)]
pub enum OreoError {
    #[error("The account `{0}` is already imported")]
    Duplicate(String),
    #[error("The account `{0}` is not imported yet")]
    NoImported(String),
    #[error("The account `{0}` is not scanned yet")]
    Scanning(String),
    #[error("The node is not synced yet")]
    Syncing,
    #[error("Internal db error")]
    DBError,
    #[error("Ironfish rpc error")]
    RpcError,
}
