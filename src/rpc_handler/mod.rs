pub mod abi;
mod handler;

pub use handler::*;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RpcError {
    pub code: String,
    pub status: u16,
    pub message: String,
}
