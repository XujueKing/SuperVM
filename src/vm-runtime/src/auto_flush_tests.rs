// 单元测试 - Auto-Flush 自动刷新功能
// 测试 MVCC 定期刷新到 RocksDB

#[cfg(all(test, feature = "rocksdb-storage"))]
mod auto_flush_tests {
    use crate::storage::RocksDBStorage;
    use crate::{AutoFlushConfig, MvccStore, Storage};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_auto_flush_start_stop() {
        let temp_dir = TempDir::new().unwrap();
        let storage = RocksDBStorage::new_with_path(temp_dir.path()).unwrap();
        let storage_arc: Arc<Mutex<dyn Storage + Send>> = Arc::new(Mutex::new(storage));

        let mvcc = MvccStore::new();

        let config = AutoFlushConfig {
            interval_secs: 1,
            blocks_per_flush: 50,
            keep_recent_versions: 2,
            flush_on_start: false,
        };

        mvcc.start_auto_flush(config, Arc::clone(&storage_arc))
            .expect("auto flush should start");

        thread::sleep(Duration::from_millis(200));

        mvcc.stop_auto_flush();
        thread::sleep(Duration::from_millis(200));

        assert!(!mvcc.is_auto_flush_running(), "自动刷新线程已停止");
    }

    #[test]
    fn test_auto_flush_interval_trigger() {
        let temp_dir = TempDir::new().unwrap();
        let storage = RocksDBStorage::new_with_path(temp_dir.path()).unwrap();
        let storage_arc: Arc<Mutex<dyn Storage + Send>> = Arc::new(Mutex::new(storage));

        let mvcc = MvccStore::new();

        for i in 0..10 {
            let mut tx = mvcc.begin();
            tx.write(format!("key_{}", i).into_bytes(), vec![i as u8]);
            tx.commit().unwrap();
        }

        let config = AutoFlushConfig {
            interval_secs: 1,
            blocks_per_flush: 0,
            keep_recent_versions: 1,
            flush_on_start: false,
        };
        mvcc.start_auto_flush(config, Arc::clone(&storage_arc))
            .expect("auto flush should start");

        thread::sleep(Duration::from_secs(2));

        mvcc.stop_auto_flush();
        thread::sleep(Duration::from_millis(200));

        let stats = mvcc.get_flush_stats();
        assert!(stats.flush_count > 0, "应该至少触发一次时间间隔刷新");
        assert!(stats.keys_flushed > 0, "应该刷新了一些键");
    }

    #[test]
    fn test_auto_flush_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let storage = RocksDBStorage::new_with_path(temp_dir.path()).unwrap();
        let storage_arc: Arc<Mutex<dyn Storage + Send>> = Arc::new(Mutex::new(storage));

        let mvcc = MvccStore::new();

        for i in 0..5 {
            let mut tx = mvcc.begin();
            tx.write(format!("key_{}", i).into_bytes(), vec![i as u8]);
            tx.commit().unwrap();
        }

        let config = AutoFlushConfig {
            interval_secs: 0,
            blocks_per_flush: 0,
            keep_recent_versions: 2,
            flush_on_start: false,
        };

        mvcc.start_auto_flush(config, Arc::clone(&storage_arc))
            .expect("auto flush should start even when disabled via config");

        thread::sleep(Duration::from_secs(2));

        mvcc.stop_auto_flush();
        thread::sleep(Duration::from_millis(200));

        let stats = mvcc.get_flush_stats();
        assert_eq!(stats.flush_count, 0, "禁用触发条件时不应该刷新");
    }

    #[test]
    fn test_flush_stats_accumulation() {
        let temp_dir = TempDir::new().unwrap();
        let mut rocksdb = RocksDBStorage::new_with_path(temp_dir.path()).unwrap();

        let mvcc = MvccStore::new();

        let initial_stats = mvcc.get_flush_stats();
        assert_eq!(initial_stats.flush_count, 0);
        assert_eq!(initial_stats.keys_flushed, 0);
        assert_eq!(initial_stats.bytes_flushed, 0);

        for i in 0..10 {
            let mut tx = mvcc.begin();
            tx.write(
                format!("key_{}", i).into_bytes(),
                format!("value_{}", i).into_bytes(),
            );
            tx.commit().unwrap();
        }

        let (keys, bytes) = mvcc.manual_flush(&mut rocksdb, 3).unwrap();

        let stats = mvcc.get_flush_stats();
        assert_eq!(stats.flush_count, 1, "应该累计 1 次刷新");
        assert_eq!(stats.keys_flushed, keys as u64, "应该累计刷新的键数");
        assert_eq!(stats.bytes_flushed, bytes, "应该累计刷新的字节数");

        for i in 10..20 {
            let mut tx = mvcc.begin();
            tx.write(
                format!("key_{}", i).into_bytes(),
                format!("value_{}", i).into_bytes(),
            );
            tx.commit().unwrap();
        }

        let (keys2, bytes2) = mvcc.manual_flush(&mut rocksdb, 3).unwrap();

        let stats2 = mvcc.get_flush_stats();
        assert_eq!(stats2.flush_count, 2, "应该累计 2 次刷新");
        assert_eq!(
            stats2.keys_flushed,
            (keys + keys2) as u64,
            "应该累计所有刷新的键数"
        );
        assert_eq!(
            stats2.bytes_flushed,
            bytes + bytes2,
            "应该累计所有刷新的字节数"
        );
    }
}
