// 单元测试 - 状态裁剪功能
// 测试 MVCC 历史版本清理

#[cfg(all(test, feature = "rocksdb-storage"))]
mod state_pruning_tests {
    use crate::storage::RocksDBStorage;
    use crate::MvccStore;
    use tempfile::TempDir;

    #[test]
    fn test_prune_old_versions_basic() {
        let temp_dir = TempDir::new().unwrap();
        let mut rocksdb = RocksDBStorage::new_with_path(temp_dir.path()).unwrap();

        let mvcc = MvccStore::new();

        for version in 0..20 {
            let mut tx = mvcc.begin();
            tx.write(
                b"test_key".to_vec(),
                format!("value_{}", version).into_bytes(),
            );
            tx.commit().unwrap();
        }

        mvcc.flush_to_storage(&mut rocksdb, usize::MAX).unwrap();

        let (pruned_versions, pruned_keys) = mvcc.prune_old_versions(5, &rocksdb);

        assert!(pruned_versions > 0, "应该清理了历史版本");
        assert_eq!(pruned_keys, 1, "应该涉及 1 个键");
        assert!(pruned_versions <= 15, "最多清理 15 个版本 (20 - 5)");
    }

    #[test]
    fn test_prune_multiple_keys() {
        let temp_dir = TempDir::new().unwrap();
        let mut rocksdb = RocksDBStorage::new_with_path(temp_dir.path()).unwrap();

        let mvcc = MvccStore::new();

        for key_id in 0..5 {
            for version in 0..10 {
                let mut tx = mvcc.begin();
                tx.write(
                    format!("key_{}", key_id).into_bytes(),
                    format!("value_{}_{}", key_id, version).into_bytes(),
                );
                tx.commit().unwrap();
            }
        }

        mvcc.flush_to_storage(&mut rocksdb, usize::MAX).unwrap();

        let (pruned_versions, pruned_keys) = mvcc.prune_old_versions(3, &rocksdb);

        assert!(pruned_versions > 0, "应该清理了历史版本");
        assert!(pruned_keys > 0, "应该涉及多个键");
        assert!(pruned_versions <= 5 * 7, "最多清理 35 个版本 (5键 × 7版本)");
    }

    #[test]
    fn test_prune_with_zero_keep() {
        let temp_dir = TempDir::new().unwrap();
        let mut rocksdb = RocksDBStorage::new_with_path(temp_dir.path()).unwrap();

        let mvcc = MvccStore::new();

        for version in 0..10 {
            let mut tx = mvcc.begin();
            tx.write(
                b"test_key".to_vec(),
                format!("value_{}", version).into_bytes(),
            );
            tx.commit().unwrap();
        }

        mvcc.flush_to_storage(&mut rocksdb, usize::MAX).unwrap();

        let (pruned_versions, pruned_keys) = mvcc.prune_old_versions(0, &rocksdb);

        assert!(pruned_versions > 0, "应该清理了所有历史版本");
        assert_eq!(pruned_keys, 1, "应该涉及 1 个键");
    }

    #[test]
    fn test_prune_empty_store() {
        let temp_dir = TempDir::new().unwrap();
        let rocksdb = RocksDBStorage::new_with_path(temp_dir.path()).unwrap();

        let mvcc = MvccStore::new();

        let (pruned_versions, pruned_keys) = mvcc.prune_old_versions(5, &rocksdb);

        assert_eq!(pruned_versions, 0, "空存储不应该清理任何版本");
        assert_eq!(pruned_keys, 0, "空存储不应该涉及任何键");
    }

    #[test]
    fn test_prune_preserve_recent_versions() {
        let temp_dir = TempDir::new().unwrap();
        let mut rocksdb = RocksDBStorage::new_with_path(temp_dir.path()).unwrap();

        let mvcc = MvccStore::new();

        for version in 0..10 {
            let mut tx = mvcc.begin();
            tx.write(
                b"test_key".to_vec(),
                format!("value_{}", version).into_bytes(),
            );
            tx.commit().unwrap();
        }

        mvcc.flush_to_storage(&mut rocksdb, usize::MAX).unwrap();

        let (pruned_versions, pruned_keys) = mvcc.prune_old_versions(10, &rocksdb);

        assert_eq!(pruned_versions, 0, "保留全部版本时不应该清理");
        assert_eq!(pruned_keys, 0, "保留全部版本时不应该涉及任何键");
    }
}
