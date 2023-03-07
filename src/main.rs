use std::net::SocketAddr;

use clap::Parser;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use tokio::io::{self};
use crate::peer::{Node, NodeDesc};
use crate::peer::wire_protocol::{NodeService, NodeServiceSet};

mod peer;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Remote IP socket address. E.g. 127.0.0.1:18334
    #[arg(short, long)]
    remote: SocketAddr,
}

fn init_logging() {
    SimpleLogger::new()
        .with_colors(true)
        .with_level(LevelFilter::Debug)
        .with_local_timestamps()
        .init()
        .unwrap();
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    init_logging();
    let args = Args::parse();

    let mut node = Node::new(NodeDesc {
        protocol_version: 70015,
        services: NodeServiceSet(vec![NodeService::NodeNetwork]),
        sub_ver: "/p2p_showcase.bitmagier:1.0".to_string(),
        start_height: 1,
    });

    match node.connect_with(args.remote).await {
        Ok(_) => {
            log::info!("connection to {} established", args.remote);
            node.close_connection(args.remote);
        }
        Err(err) => {
            log::warn!("connection attempt to {} failed: {}", args.remote, err);
        }
    }

    Ok(())
}
