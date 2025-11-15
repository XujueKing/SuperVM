// Minimal cross-shard gRPC server example
// Requires: cargo run -p vm-runtime --example cross_shard_minimal --features cross-shard

#![cfg(feature = "cross-shard")]

use vm_runtime::shard::service::{server, ShardNode};
use tonic::transport::Server;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = "127.0.0.1:50051".parse()?;
    println!("Starting ShardService gRPC server on {}", addr);
    let node = ShardNode::new(0);
    Server::builder()
        .add_service(server(node))
        .serve(addr)
        .await?;
    Ok(())
}
