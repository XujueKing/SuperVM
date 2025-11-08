// SPDX-License-Identifier: GPL-3.0-or-later
// 简易 RocksDB 监控示例：每 5s 打印一次统计

use anyhow::Result;
use std::time::Duration;

#[cfg(feature = "rocksdb-storage")]
use vm_runtime::RocksDBStorage;

fn main() -> Result<()> {
    #[cfg(feature = "rocksdb-storage")]
    {
        let storage = RocksDBStorage::new_default()?;
        loop {
            let _ = storage.print_stats();
            std::thread::sleep(Duration::from_secs(5));
        }
    }

    #[cfg(not(feature = "rocksdb-storage"))]
    {
        eprintln!("rocksdb-storage feature not enabled");
    }

    Ok(())
}
