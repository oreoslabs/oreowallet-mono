pub mod orescriptions;
pub mod rpc_abi;
pub mod rpc_handler;
pub mod socket_message;
pub mod web_abi;

use db_handler::{DBTransaction, InnerBlock, Json};
use rpc_abi::RpcBlock;
pub use ureq;

impl RpcBlock {
    pub fn to_inner(self) -> InnerBlock {
        let RpcBlock {
            hash,
            sequence,
            previous_block_hash: _,
            transactions,
        } = self;
        InnerBlock {
            hash,
            sequence: sequence as i64,
            transactions: Json(
                transactions
                    .into_iter()
                    .map(|tx| DBTransaction {
                        hash: tx.hash,
                        serialized_notes: tx
                            .notes
                            .into_iter()
                            .map(|note| note.serialized)
                            .collect(),
                    })
                    .collect(),
            ),
        }
    }
}
