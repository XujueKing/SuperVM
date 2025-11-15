// Phase B: Cross-Shard gRPC module
// This module exposes generated proto types and basic server skeletons.

#[cfg(feature = "cross-shard")]
pub mod proto {
    // 由 build.rs 生成的模块文件名基于 proto 包名, tonic 使用包名生成。
    // 这里我们使用 include! 引入 OUT_DIR 中生成的文件。
    include!(concat!(env!("OUT_DIR"), "/supervm.crossshard.v1.rs"));
}

#[cfg(feature = "cross-shard")]
pub mod service {
    use super::proto::shard_service_server::{ShardService, ShardServiceServer};
    use super::proto::*;
    use crate::{CrossShardMvccExt, MvccScheduler, SuperVM};
    use std::sync::Arc;
    use tonic::{Request, Response, Status};

    #[derive(Debug)]
    pub struct ShardNode {
        pub mvcc: Arc<MvccScheduler>,
        pub ext: Arc<CrossShardMvccExt>,
        pub supervm: Option<&'static SuperVM<'static>>, // 可选挂接 SuperVM (含批量 ZK)
        pub shard_id: u16,
    }

    impl Default for ShardNode {
        fn default() -> Self {
            Self { mvcc: Arc::new(MvccScheduler::new()), ext: Arc::new(CrossShardMvccExt::new(0)), supervm: None, shard_id: 0 }
        }
    }

    impl ShardNode {
        pub fn new(shard_id: u16) -> Self {
            Self { mvcc: Arc::new(MvccScheduler::new()), ext: Arc::new(CrossShardMvccExt::new(shard_id)), supervm: None, shard_id }
        }
        pub fn with_supervm(mut self, vm: &'static SuperVM<'static>) -> Self { self.supervm = Some(vm); self }
    }

    #[tonic::async_trait]
    impl ShardService for ShardNode {
        async fn prepare_txn(
            &self,
            request: Request<PrepareRequest>,
        ) -> Result<Response<PrepareResponse>, Status> {
            let req = request.into_inner();
            let start = std::time::Instant::now();
            let mut privacy_invalid = false;
            // 版本校验：读取本地 MVCC 中的版本与 read_set 期望比较
            for ov in &req.read_set {
                // object_id bytes -> hex string key
                let hex_id = hex::encode(&ov.object_id);
                let version_key = format!("obj_{}_version", hex_id);
                let mut txn = self.mvcc.store().begin();
                let actual_version = txn.read(version_key.as_bytes())
                    .and_then(|b| String::from_utf8(b).ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                if actual_version != ov.version {
                    let vote = Some(prepare_response::Vote::No(VoteNo { txn_id: req.txn_id, reason: format!("version_mismatch object={} expected={} actual={}", hex_id, ov.version, actual_version) }));
                    if let Some(mc) = self.mvcc.store().get_metrics() { mc.record_cross_shard_prepare(start.elapsed().as_secs_f64()*1000.0, false, false); }
                    return Ok(Response::new(PrepareResponse { txn_id: req.txn_id, vote }));
                }
            }
            // 隐私验证（若携带 privacy 且 supervm 存在）
            if let (Some(p), Some(vm)) = (&req.privacy, self.supervm) {
                // 将 public_inputs 拼接为单个字节数组 (简化)
                let mut concat_inputs = Vec::new();
                for pi in &p.public_inputs { concat_inputs.extend_from_slice(pi); }
                if !vm.verify_zk_proof(Some(&p.proof_bytes), Some(&concat_inputs)) {
                    let vote = Some(prepare_response::Vote::No(VoteNo { txn_id: req.txn_id, reason: "invalid_proof".into() }));
                    privacy_invalid = true;
                    if let Some(mc) = self.mvcc.store().get_metrics() { mc.record_cross_shard_prepare(start.elapsed().as_secs_f64()*1000.0, false, true); }
                    return Ok(Response::new(PrepareResponse { txn_id: req.txn_id, vote }));
                }
            }
            let vote = Some(prepare_response::Vote::Yes(VoteYes { txn_id: req.txn_id }));
            if let Some(mc) = self.mvcc.store().get_metrics() { mc.record_cross_shard_prepare(start.elapsed().as_secs_f64()*1000.0, true, privacy_invalid); }
            Ok(Response::new(PrepareResponse { txn_id: req.txn_id, vote }))
        }

        async fn commit_txn(
            &self,
            request: Request<CommitRequest>,
        ) -> Result<Response<CommitResponse>, Status> {
            let req = request.into_inner();
            Ok(Response::new(CommitResponse { txn_id: req.txn_id, status: 0 }))
        }

        async fn abort_txn(
            &self,
            request: Request<AbortRequest>,
        ) -> Result<Response<AbortResponse>, Status> {
            let req = request.into_inner();
            Ok(Response::new(AbortResponse { txn_id: req.txn_id, acknowledged: true }))
        }

        async fn get_object_versions(
            &self,
            _request: Request<VersionRequest>,
        ) -> Result<Response<VersionResponse>, Status> {
            let req = _request.into_inner();
            let mut versions = Vec::with_capacity(req.object_ids.len());
            let mut txn = self.mvcc.store().begin();
            for oid in req.object_ids {
                let hex_id = hex::encode(&oid);
                let version_key = format!("obj_{}_version", hex_id);
                let ver = txn.read(version_key.as_bytes())
                    .and_then(|b| String::from_utf8(b).ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                versions.push(ObjectVersion { object_id: oid, version: ver });
            }
            Ok(Response::new(VersionResponse { versions }))
        }

        type StreamShardEventsStream = 
            tokio_stream::wrappers::ReceiverStream<Result<ShardEvent, Status>>;

        async fn stream_shard_events(
            &self,
            _request: Request<ShardEventRequest>,
        ) -> Result<Response<Self::StreamShardEventsStream>, Status> {
            let (_tx, rx) = tokio::sync::mpsc::channel(4);
            Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
        }
    }

    pub fn server(node: ShardNode) -> ShardServiceServer<ShardNode> { ShardServiceServer::new(node) }
}
