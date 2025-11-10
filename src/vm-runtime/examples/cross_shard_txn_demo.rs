// Cross-shard transaction demo (two shards in-process)
// Run with: cargo run -p vm-runtime --features cross-shard --example cross_shard_txn_demo
#![cfg(feature = "cross-shard")]

use vm_runtime::shard::service::{server, ShardNode};
use vm_runtime::{ShardCoordinator, ShardConfig, ShardId, Decision};
use vm_runtime::cross_shard_proto::prepare_request::Privacy as _; // placeholder to ensure proto link
use vm_runtime::cross_shard_proto::{PrepareRequest, ObjectVersion, KeyWrite};
use tonic::transport::Server;
use std::net::SocketAddr;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // launch two shard servers
    let shard_endpoints: HashMap<ShardId, String> = vec![(0u16, "127.0.0.1:60051".to_string()), (1u16, "127.0.0.1:60052".to_string())].into_iter().collect();

    // spawn servers
    for (sid, ep) in shard_endpoints.clone() {
        tokio::spawn(async move {
            let addr: SocketAddr = ep.parse().unwrap();
            println!("Shard {} listening on {}", sid, addr);
            let node = ShardNode::new(sid as u16);
            Server::builder().add_service(server(node)).serve(addr).await.unwrap();
        });
    }

    // give servers a moment
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // coordinator config
    let cfg = ShardConfig {
        num_shards: 2,
        local_shard_id: 0,
        shard_endpoints: shard_endpoints.clone(),
    };
    let mut coord = ShardCoordinator::new(cfg);
    coord.connect_all().await?;

    // build two prepare requests (simulate object sets)
    let reqs = vec![
        (0u16, PrepareRequest {
            txn_id: 1,
            shard_id: 0,
            read_set: vec![ObjectVersion { object_id: b"A".to_vec(), version: 1 }],
            write_set: vec![KeyWrite { object_id: b"B".to_vec(), new_value: b"v1".to_vec() }],
            timestamp: 100,
            trace_id_high: 0,
            trace_id_low: 1,
            coordinator_epoch: 1,
            retry_count: 0,
            privacy: None,
        }),
        (1u16, PrepareRequest {
            txn_id: 1,
            shard_id: 1,
            read_set: vec![ObjectVersion { object_id: b"C".to_vec(), version: 1 }],
            write_set: vec![KeyWrite { object_id: b"D".to_vec(), new_value: b"v2".to_vec() }],
            timestamp: 100,
            trace_id_high: 0,
            trace_id_low: 1,
            coordinator_epoch: 1,
            retry_count: 0,
            privacy: None,
        })
    ];

    let prepare_res = coord.prepare_all(reqs).await?;
    for (sid, resp) in &prepare_res {
        println!("Prepare response from shard {}: vote={:?}", sid, resp.vote.as_ref().map(|v| v));
    }

    // commit
    coord.commit_all(vec![0u16,1u16], Decision::Commit, 1, 1).await?;
    println!("Commit broadcast complete");

    Ok(())
}
