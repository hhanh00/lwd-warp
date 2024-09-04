use anyhow::Result;
use lwd_warp::{
    server::{LWDServer, SERVER_DESCRIPTOR_SET},
    POOL,
};
use tokio::sync::mpsc;
use tonic::transport::Server;
use zcash_primitives::consensus::{MainNetwork, NetworkUpgrade, Parameters};
use zcash_warp::{
    coin::connect_lwd,
    lwd::{
        get_compact_block_range, get_tree_state,
        rpc::compact_tx_streamer_server::CompactTxStreamerServer,
    },
    warp::sync::builder::purge_blocks,
};

pub async fn server_main() -> Result<()> {
    dotenv::dotenv()?;

    let connection = POOL.get()?;
    let max_height =
        connection.query_row("SELECT MAX(height) FROM cp_blk", [], |r| r.get::<_, u32>(0))?;

    let addr = "0.0.0.0:8000".parse().unwrap();
    let server = LWDServer::new(max_height).await?;

    tracing::info!("Lightwalletd WARP listening on {}", addr);

    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(SERVER_DESCRIPTOR_SET)
        .build()
        .unwrap();

    Server::builder()
        .add_service(service)
        .add_service(CompactTxStreamerServer::new(server))
        .serve(addr)
        .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn build_db() -> Result<()> {
    let connection = POOL.get()?;
    connection.execute(
        "CREATE TABLE IF NOT EXISTS cp_blk(
        height INTEGER PRIMARY KEY NOT NULL,
        data BLOB NOT NULL)",
        [],
    )?;
    let url = dotenv::var("LWD").unwrap();
    let end_height = str::parse::<u32>(&dotenv::var("END_HEIGHT").unwrap())?;
    let start_height: u32 = MainNetwork
        .activation_height(NetworkUpgrade::Sapling)
        .unwrap()
        .into();
    let mut client = connect_lwd(&url).await?;
    let (s, o) = get_tree_state(&mut client, start_height).await?;
    let (tx, rx) = mpsc::channel(128);
    tokio::spawn(async move {
        let mut block = get_compact_block_range(&mut client, start_height + 1, end_height).await?;
        while let Some(cb) = block.message().await? {
            tx.send(cb).await?;
        }
        Ok::<_, anyhow::Error>(())
    });
    purge_blocks(connection, rx, &s, &o).await?;

    Ok(())
}

#[tokio::main]
pub async fn main() -> Result<()> {
    dotenv::dotenv()?;
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    server_main().await?;
    // build_db().await?;

    Ok(())
}
