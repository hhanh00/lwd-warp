use std::net::{Ipv4Addr, SocketAddrV4};

use anyhow::Result;
use clap::Parser;
use crate::{
    server::{LWDServer, SERVER_DESCRIPTOR_SET}, POOL
};
use tokio::sync::mpsc;
use tonic::transport::Server;
use zcash_primitives::consensus::{MainNetwork, NetworkUpgrade, Parameters};
use zcash_warp::{
    coin::connect_lwd, lwd::{
        get_compact_block_range, get_tree_state,
        rpc::compact_tx_streamer_server::CompactTxStreamerServer,
    }, types::CheckpointHeight, warp::sync::builder::purge_blocks
};
use clap_repl::{reedline::{
    DefaultPrompt, DefaultPromptSegment, FileBackedHistory,
}, ClapEditor};

#[derive(Parser, Clone, Debug)]
#[command(name = "")]
pub enum Command {
    BuildBridges { remote_server: String, height: u32 },
    StartServer { port: u16 },
}

#[tokio::main]
async fn process_command(command: Command) -> Result<()> {
    match command {
        Command::BuildBridges { remote_server, height } => {
            build_db(&remote_server, height).await?;
        }
        Command::StartServer { port } => {
            start_server(port).await?;
        }
    }
    Ok(())
}

pub async fn build_db(url: &str, end_height: u32) -> Result<()> {
    let connection = POOL.get()?;
    connection.execute(
        "CREATE TABLE IF NOT EXISTS cp_blk(
        height INTEGER PRIMARY KEY NOT NULL,
        data BLOB NOT NULL)",
        [],
    )?;
    let start_height: u32 = MainNetwork
        .activation_height(NetworkUpgrade::Sapling)
        .unwrap()
        .into();
    tracing::info!("Starting scan: {start_height}");
    let mut client = connect_lwd(url).await?;
    let (s, o) = get_tree_state(&mut client, CheckpointHeight(start_height)).await?;
    let (tx, rx) = mpsc::channel(128);
    tokio::spawn(async move {
        let mut block = get_compact_block_range(&mut client, start_height + 1, end_height).await?;
        while let Some(cb) = block.message().await? {
            tx.send(cb).await?;
        }
        Ok::<_, anyhow::Error>(())
    });
    purge_blocks(connection, rx, &s, &o).await?;
    tracing::info!("Scan completed");

    Ok(())
}

pub async fn start_server(port: u16) -> Result<()> {
    let connection = POOL.get()?;
    let max_height =
        connection.query_row("SELECT MAX(height) FROM cp_blk", [], |r| r.get::<_, u32>(0))?;

    let addr = std::net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port));
    let server = LWDServer::new(max_height).await?;

    tracing::info!("Lightwalletd WARP listening on {}", addr);

    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(SERVER_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    Server::builder()
        .add_service(service)
        .add_service(CompactTxStreamerServer::new(server))
        .serve(addr)
        .await?;
    Ok(())
}

pub fn cli_main() -> Result<()> {
    let prompt = DefaultPrompt {
        left_prompt: DefaultPromptSegment::Basic("lwd-warp".to_owned()),
        ..DefaultPrompt::default()
    };
    let rl = ClapEditor::<Command>::builder()
        .with_prompt(Box::new(prompt))
        .with_editor_hook(|reed| {
            reed.with_history(Box::new(
                FileBackedHistory::with_file(10000, "/tmp/lwd-warp-history".into())
                    .unwrap(),
            ))
        })
        .build();
    rl.repl(|command| {
        if let Err(e) = process_command(command) {
            tracing::error!("{e}");
        }
    });
    Ok(())
}
