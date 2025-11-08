// SPDX-License-Identifier: GPL-3.0-or-later
// 简易 RocksDB 监控示例（每 5s 打印一次统计）

use anyhow::Result;
use std::time::Duration;
use vm_runtime::{RocksDBConfig, RocksDBStorage};

fn main() -> Result<()> {
    let cfg = RocksDBConfig::production_optimized();
    let storage = RocksDBStorage::new(cfg)?;
    loop {
        let _ = storage.print_stats();
        std::thread::sleep(Duration::from_secs(5));
    }
}
