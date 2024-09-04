use anyhow::Result;
use compact_tx_streamer_server::CompactTxStreamer;
use prost::Message;
use rusqlite::params;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use zcash_warp::lwd::rpc::*;

use crate::POOL;

pub const SERVER_DESCRIPTOR_SET: &[u8] = include_bytes!("cash.z.wallet.sdk.rpc.bin");

pub struct LWDServer {
    max_height: u32,
}

impl LWDServer {
    pub async fn new(max_height: u32) -> Result<LWDServer> {
        let server = LWDServer {
            max_height,
        };

        Ok(server)
    }
}

fn map_tonic_err<E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>>(e: E) -> Status {
    Status::from_error(e.into())
}

pub async fn handle_get_block_range(
    range: BlockRange,
    // tx: Sender<Result<CompactBlock, Status>>,
    // rx: Receiver<Result<CompactBlock, Status>>,
    cutoff: u32,
) -> Result<ReceiverStream<Result<CompactBlock, Status>>> {
    const NBLOCKS: u64 = 10_000;

    let (tx, rx) = mpsc::channel(16);
    let recv = ReceiverStream::new(rx);
    let end = range.end.as_ref().ok_or(anyhow::anyhow!("No end height"))?;
    let end_height = end.height;
    if end_height < cutoff as u64 {
        tokio::spawn(async move {
            let s = range.start.ok_or(anyhow::anyhow!("No start height"))?;
            let start_height = s.height;
            let mut s = start_height;
            while s <= end_height {
                let e = (s + NBLOCKS - 1).min(end_height); // read NBLOCKS at a time
                let mut cbs = vec![]; // cannot hold db statement between await points
                {
                    let connection = POOL.get()?;
                    let mut st = connection.prepare_cached(
                    "SELECT data FROM cp_blk WHERE height >= ?1 AND height <= ?2 ORDER BY height")?;
                    let rows = st.query_map(params![s, e], |r| Ok((r.get::<_, Vec<u8>>(0)?,)))?;
                    for r in rows {
                        let (data,) = r?;
                        let cb = CompactBlock::decode(&*data)?;
                        cbs.push(cb);
                    }
                }
                for cb in cbs {
                    tx.send(Ok(cb)).await?;
                }
                s = e + 1;
            }
            Ok::<_, anyhow::Error>(())
        });
    } else {
        unimplemented!()
    }
    Ok(recv)
}

#[tonic::async_trait]
impl CompactTxStreamer for LWDServer {
    /// Return the height of the tip of the best chain
    async fn get_latest_block(
        &self,
        _request: Request<ChainSpec>,
    ) -> Result<Response<BlockId>, Status> {
        todo!()
    }

    /// Server streaming response type for the GetBlockRange method.
    type GetBlockRangeStream = ReceiverStream<Result<CompactBlock, Status>>;
    type GetPrunedBlockRangeStream = ReceiverStream<Result<CompactBlock, Status>>;

    /// Return the compact block corresponding to the given block identifier
    /// rpc GetBlock(BlockID) returns (CompactBlock) {}
    /// Return a list of consecutive compact blocks
    async fn get_block_range(
        &self,
        request: Request<BlockRange>,
    ) -> Result<Response<Self::GetBlockRangeStream>, Status> {
        // let (tx, rx) = mpsc::channel(16);
        let rep = handle_get_block_range(
            request.into_inner(),
            // tx, rx,
            self.max_height,
        )
        .await
        .map_err(map_tonic_err)?;
        Ok(Response::new(rep))
    }

    /// Server streaming response type for the GetPrunedBlockRange method.
    async fn get_pruned_block_range(
        &self,
        _request: Request<BlockRange>,
    ) -> Result<Response<Self::GetBlockRangeStream>, Status> {
        todo!()
    }
    /// Return the requested full (not compact) transaction (as from zcashd)

    async fn get_transaction(
        &self,
        _request: Request<TxFilter>,
    ) -> Result<Response<RawTransaction>, Status> {
        todo!()
    }

    /// Submit the given transaction to the Zcash network
    async fn send_transaction(
        &self,
        _request: Request<RawTransaction>,
    ) -> Result<Response<SendResponse>, Status> {
        todo!()
    }

    /// Server streaming response type for the GetTaddressTxids method.
    type GetTaddressTxidsStream = ReceiverStream<Result<RawTransaction, Status>>;
    /// Return the txids corresponding to the given t-address within the given block range
    async fn get_taddress_txids(
        &self,
        _request: Request<TransparentAddressBlockFilter>,
    ) -> Result<Response<Self::GetTaddressTxidsStream>, Status> {
        todo!()
    }

    /// GetTreeState returns the note commitment tree state corresponding to the given block.
    /// See section 3.7 of the Zcash protocol specification. It returns several other useful
    /// values also (even though they can be obtained using GetBlock).
    /// The block can be specified by either height or hash.
    async fn get_tree_state(
        &self,
        _request: Request<BlockId>,
    ) -> Result<Response<TreeState>, Status> {
        todo!()
    }

    /// Server streaming response type for the GetAddressUtxosStream method.
    type GetAddressUtxosStreamStream = ReceiverStream<Result<GetAddressUtxosReply, Status>>;
    /// rpc GetAddressUtxos(GetAddressUtxosArg) returns (GetAddressUtxosReplyList) {}
    async fn get_address_utxos_stream(
        &self,
        _request: Request<GetAddressUtxosArg>,
    ) -> Result<Response<Self::GetAddressUtxosStreamStream>, Status> {
        todo!()
    }

    /// Return information about this lightwalletd instance and the blockchain
    async fn get_lightd_info(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<LightdInfo>, Status> {
        todo!()
    }

    /// Testing-only, requires lightwalletd --ping-very-insecure (do not enable in production)
    async fn ping(&self, _request: Request<Duration>) -> Result<Response<PingResponse>, Status> {
        todo!()
    }
}
