use constants::{LOCAL_BLOCKS_CHECKPOINT, PRIMARY_BATCH};
use db_handler::{DBHandler, DBTransaction, InnerBlock, PgHandler};
use networking::{rpc_abi::RpcBlock, rpc_handler::RpcHandler};
use tracing::info;
use utils::blocks_range;

pub async fn load_checkpoint(rpc_node: String, db_handler: PgHandler) -> anyhow::Result<()> {
    let rpc_handler = RpcHandler::new(rpc_node);
    for group in blocks_range(1..LOCAL_BLOCKS_CHECKPOINT + 1, PRIMARY_BATCH) {
        if db_handler
            .get_blocks(group.end as i64, group.end as i64)
            .await
            .is_ok()
        {
            continue;
        }
        let results = rpc_handler.get_blocks(group.start, group.end).unwrap();
        let blocks: Vec<RpcBlock> = results
            .data
            .blocks
            .into_iter()
            .map(|item| item.block)
            .collect();
        let inner_blocks = blocks
            .into_iter()
            .map(|rpc| InnerBlock {
                hash: rpc.hash.clone(),
                sequence: rpc.sequence as i64,
                transactions: rpc
                    .transactions
                    .into_iter()
                    .map(|tx| DBTransaction {
                        hash: tx.hash,
                        serialized_notes: tx.notes.into_iter().map(|n| n.serialized).collect(),
                    })
                    .collect(),
            })
            .collect();
        info!(
            "save blocks from {} to {} in local db",
            group.start, group.end
        );
        let _ = db_handler.save_blocks(inner_blocks).await;
    }
    Ok(())
}
