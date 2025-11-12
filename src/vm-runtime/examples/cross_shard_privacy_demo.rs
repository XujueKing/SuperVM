// Cross-shard privacy transaction demo
// Run with: cargo run -p vm-runtime --features cross-shard --example cross_shard_privacy_demo
#![cfg(feature = "cross-shard")]

#[cfg(feature = "cross-shard")]
use vm_runtime::shard::service::{server, ShardNode};
#[cfg(feature = "cross-shard")]
use vm_runtime::{ShardCoordinator, ShardConfig, ShardId, Decision};
#[cfg(feature = "cross-shard")]
use vm_runtime::cross_shard_proto::{PrepareRequest, ObjectVersion, KeyWrite, PrivacyProof};
#[cfg(feature = "cross-shard")]
use tonic::transport::Server;
#[cfg(feature = "cross-shard")]
use std::net::SocketAddr;
#[cfg(feature = "cross-shard")]
use std::collections::HashMap;

#[cfg(feature = "cross-shard")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Two shard endpoints
    let shard_endpoints: HashMap<ShardId, String> = vec![(0u16, "127.0.0.1:61051".to_string()), (1u16, "127.0.0.1:61052".to_string())].into_iter().collect();

    // Build a SuperVM and leak to static for demo (no real verifier injected -> fallback true)
    let vm = {
        // Minimal setup using ownership manager inside vm_runtime
        let ownership = vm_runtime::OwnershipManager::new();
        let sched = vm_runtime::parallel_mvcc::MvccScheduler::new();
        let vm = vm_runtime::SuperVM::new(&ownership).with_scheduler(&sched).from_env();
        Box::leak(Box::new(vm)) as &'static vm_runtime::SuperVM<'static>
    };

    // spawn two shard servers with SuperVM injected
    for (sid, ep) in shard_endpoints.clone() {
        let vm_ref = vm as *const _; // copy pointer into task move
        tokio::spawn(async move {
            let addr: SocketAddr = ep.parse().unwrap();
            println!("Shard {} listening on {}", sid, addr);
            let vm: &'static vm_runtime::SuperVM<'static> = unsafe { &*vm_ref };
            let node = ShardNode::new(sid as u16).with_supervm(vm);
            Server::builder().add_service(server(node)).serve(addr).await.unwrap();
        });
    }

    // give servers a moment
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // coordinator config
    let cfg = ShardConfig { num_shards: 2, local_shard_id: 0, shard_endpoints: shard_endpoints.clone() };
    let mut coord = ShardCoordinator::new(cfg);
    coord.connect_all().await?;

    // Construct a mock privacy proof with some bytes
    let privacy = PrivacyProof {
        system: 0, // ZK_GROTH16_BLS12_381
        proof_bytes: vec![1,2,3,4,5,6,7,8],
        public_inputs: vec![b"pi1".to_vec(), b"pi2".to_vec()],
        commitments: vec![],
    };

    // Build two prepare requests with same txn id and privacy payload
    let reqs = vec![
        (0u16, PrepareRequest { txn_id: 42, shard_id: 0, read_set: vec![ObjectVersion { object_id: b"A".to_vec(), version: 0 }], write_set: vec![KeyWrite { object_id: b"B".to_vec(), new_value: b"v".to_vec() }], timestamp: 100, trace_id_high: 0, trace_id_low: 1, coordinator_epoch: 1, retry_count: 0, privacy: Some(privacy.clone()) }),
        (1u16, PrepareRequest { txn_id: 42, shard_id: 1, read_set: vec![ObjectVersion { object_id: b"C".to_vec(), version: 0 }], write_set: vec![KeyWrite { object_id: b"D".to_vec(), new_value: b"v2".to_vec() }], timestamp: 100, trace_id_high: 0, trace_id_low: 1, coordinator_epoch: 1, retry_count: 0, privacy: Some(privacy) }),
    ];

    let votes = coord.prepare_all(reqs).await?;
    for (sid, resp) in &votes { println!("Prepare vote from shard {}: {:?}", sid, resp.vote.as_ref().map(|v| v)); }

    // If all voted yes, commit
    let all_yes = votes.iter().all(|(_, r)| matches!(r.vote, Some(vm_runtime::cross_shard_proto::prepare_response::Vote::Yes(_))));
    let decision = if all_yes { Decision::Commit } else { Decision::Abort };
    coord.commit_all(vec![0u16,1u16], decision, 42, 1).await?;
    println!("Final decision broadcast: {:?}", decision as i32);

    Ok(())
}

#[cfg(not(feature = "cross-shard"))]
fn main() {
    eprintln!("[cross_shard_privacy_demo] feature 'cross-shard' 未启用，示例被跳过。");
}
