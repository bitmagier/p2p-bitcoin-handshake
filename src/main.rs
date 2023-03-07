use std::net::SocketAddr;

use clap::Parser;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use tokio::time::{timeout, Duration};
use tokio::io::{self};
use crate::peer::{Node, NodeDesc};
use crate::peer::wire_protocol::{NodeService, NodeServiceSet};

mod peer;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Remote IP socket address. E.g. 127.0.0.1:18445 for a local regression testnet node
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
        protocol_version: 70016, // matches bitcoin core v24
        services: NodeServiceSet(vec![NodeService::NodeNetwork]),
        sub_ver: "/p2p_showcase.bitmagier:1.0".to_string(),
        start_height: 1,
    });

    let handshake_timeout = Duration::from_secs(5);
    match timeout(handshake_timeout, node.connect_with(args.remote)).await {
        Ok(result) => {
            match result {
                Ok(node_desc) => {
                    log::info!("connection + handshake to node @ {} successfully established", args.remote);
                    log::debug!("Remote node details: {:?}", node_desc);
                    node.close_connection(args.remote);
                    log::debug!("connection intentionally closed by us, because this is the end of the showcase");
                }
                Err(err) => {
                    log::warn!("error while communicating with {}: {}", args.remote, err);
                }
            }
        },
        Err(_) => {
            log::warn!("handshake timeout")
        }
    }

    Ok(())
}
