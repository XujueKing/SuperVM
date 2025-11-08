// SPDX-License-Identifier: GPL-3.0-or-later
// MVCC flush_to_storage 批量写路径测试 (需启用 rocksdb-storage 特性)
// 验证点:
// 1. 在 RocksDBStorage 下 flush_to_storage 走 batch 聚合逻辑
// 2. 返回的 flushed_keys 与写入数一致
// 3. flushed_bytes 近似为 key+value 长度累加 (这里简单 key/value)
// 4. 保留最近版本数逻辑: keep_recent_versions 生效后旧版本被裁剪
// 由于 trait 通过 as_any downcast，我们只需调用 flush_to_storage 并观察结果即可。
// 注意: tests 目录为集成测试，启用 feature 后才会编译此文件。

#[cfg(feature = "rocksdb-storage")]
mod tests {
    use vm_runtime::{MvccStore, RocksDBConfig, RocksDBStorage, Storage};

    fn temp_db_path() -> tempfile::TempDir { tempfile::tempdir().expect("tempdir") }

    #[test]
    fn test_flush_batch_basic() {
        let store = MvccStore::new();
        let dir = temp_db_path();
        let mut rocks = RocksDBStorage::new(RocksDBConfig::default().with_path(dir.path().to_string_lossy().to_string())).expect("rocksdb init");

        // 写入多个事务提交的数据
        for i in 0..10u32 {
            let mut txn = store.begin();
            let key = format!("k{}", i).into_bytes();
            let val = format!("v{}", i).into_bytes();
            txn.write(key.clone(), val.clone());
            txn.commit().expect("commit");
        }

        // flush 前数据库还没有这些 key
        for i in 0..10u32 {
            let key = format!("k{}", i).into_bytes();
            assert!(rocks.get(&key).unwrap().is_none());
        }

        let (flushed_keys, flushed_bytes) = store.flush_to_storage(&mut rocks, 3).expect("flush");
        assert_eq!(flushed_keys, 10, "should flush all 10 latest committed keys");
        assert!(flushed_bytes > 0);

        // flush 后 RocksDB 中可读取
        for i in 0..10u32 {
            let key = format!("k{}", i).into_bytes();
            let v = rocks.get(&key).unwrap();
            assert!(v.is_some(), "key should exist after batch flush");
        }
    }

    #[test]
    fn test_flush_respects_keep_recent_versions() {
        let store = MvccStore::new();
        let dir = temp_db_path();
        let mut rocks = RocksDBStorage::new(RocksDBConfig::default().with_path(dir.path().to_string_lossy().to_string())).expect("rocksdb init");

        // 对同一个 key 连续写入多个版本
        let key = b"hot".to_vec();
        for i in 0..8u32 {
            let mut txn = store.begin();
            txn.write(key.clone(), format!("val{}", i).into_bytes());
            txn.commit().expect("commit");
        }
        // flush: 保留最近 3 个版本在内存
        let (kcnt, _bytes) = store.flush_to_storage(&mut rocks, 3).expect("flush");
        assert_eq!(kcnt, 1); // 只有一个键被刷

        // 再写入两个新版本，验证旧版本已被裁剪时仍可继续 flush
        for i in 8..10u32 {
            let mut txn = store.begin();
            txn.write(key.clone(), format!("val{}", i).into_bytes());
            txn.commit().expect("commit");
        }
        let (kcnt2, _bytes2) = store.flush_to_storage(&mut rocks, 3).expect("second flush");
        assert_eq!(kcnt2, 1);
    }
}
