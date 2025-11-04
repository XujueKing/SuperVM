use std::time::Instant;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use vm_runtime::{MvccScheduler, MvccSchedulerConfig, Txn};

enum Op {
    Read(Vec<u8>),
    Write(Vec<u8>),
}

fn pct(x: u64, y: u64) -> f64 { if y == 0 { 0.0 } else { (x as f64) / (y as f64) * 100.0 } }

#[test]
fn test_mvcc_scheduler_high_concurrency() {
    // 配置调度器（默认启用自适应 GC）
    let scheduler = MvccScheduler::new_with_config(MvccSchedulerConfig::default());

    // 生成初始数据
    scheduler.batch_write(
        (0..100).map(|i| (format!("k{}", i).into_bytes(), b"0".to_vec())).collect()
    ).unwrap();

    // 高并发混合负载：8 线程 * 2000 交易，共 16K 笔
    let threads = 8u64;
    let tx_per_thread = 2000u64;
    let total = threads * tx_per_thread;

    let start = Instant::now();

    // 为了可重现性，使用固定种子
    // 先确定所有操作，避免闭包内部使用可变 RNG
    let mut plan: Vec<(u64, Op)> = Vec::with_capacity(total as usize);
    for tid in 0..threads {
        let mut rng = StdRng::seed_from_u64(0xC0FFEE + tid);
        for i in 0..tx_per_thread {
            let tx_id = tid * tx_per_thread + i;
            let kidx = rng.gen_range(0..100);
            let key = format!("k{}", kidx).into_bytes();
            if rng.gen_bool(0.3) {
                plan.push((tx_id, Op::Write(key)));
            } else {
                plan.push((tx_id, Op::Read(key)));
            }
        }
    }

    let txns: Vec<_> = plan
        .into_iter()
        .map(|(tx_id, op)| {
            (tx_id, move |txn: &mut Txn| -> anyhow::Result<i32> {
                match &op {
                    Op::Write(key) => {
                        let val = txn
                            .read(&key)
                            .and_then(|v| String::from_utf8(v.clone()).ok())
                            .and_then(|s| s.parse::<i64>().ok())
                            .unwrap_or(0);
                        let new_val = val + 1;
                        txn.write(key.clone(), new_val.to_string().into_bytes());
                        Ok(new_val as i32)
                    }
                    Op::Read(key) => {
                        let _ = txn.read(&key);
                        Ok(0)
                    }
                }
            })
        })
        .collect();

    let batch = scheduler.execute_batch(txns);
    let dur = start.elapsed();

    let tps = total as f64 / dur.as_secs_f64();
    let success_rate = pct(batch.successful, batch.successful + batch.failed);

    println!("\n==== MVCC Scheduler 高并发测试报告 ====");
    println!("总交易数: {}", total);
    println!("成功交易: {} ({:.2}%)", batch.successful, success_rate);
    println!("失败交易: {}", batch.failed);
    println!("冲突次数: {}", batch.conflicts);
    println!("运行时间: {:.2} 秒", dur.as_secs_f64());
    println!("吞吐量: {:.2} TPS", tps);

    // 断言：高成功率 + 合理 TPS（这台机型/CI 可能波动，设置较宽松阈值）
    assert!(success_rate > 95.0, "success_rate too low: {:.2}%", success_rate);
    assert!(tps > 50_000.0, "TPS too low: {:.2}", tps);
}

#[test]
fn test_mvcc_scheduler_hotspot_contention() {
    let scheduler = MvccScheduler::new_with_config(MvccSchedulerConfig::default());

    // 热点 5 个键
    let hot_keys: Vec<_> = (0..5).map(|i| format!("hot{}", i).into_bytes()).collect();
    scheduler.batch_write(hot_keys.iter().map(|k| (k.clone(), b"0".to_vec())).collect()).unwrap();

    // 16 线程 * 500 交易，全部写热点键
    let threads = 16u64;
    let tx_per_thread = 500u64;
    let total = threads * tx_per_thread;

    let start = Instant::now();

    let mut plan2: Vec<(u64, Vec<u8>)> = Vec::with_capacity(total as usize);
    for tid in 0..threads {
        let mut rng = StdRng::seed_from_u64(0xFEEDBEEF + tid);
        for i in 0..tx_per_thread {
            let tx_id = tid * tx_per_thread + i;
            let key = hot_keys[rng.gen_range(0..hot_keys.len())].clone();
            plan2.push((tx_id, key));
        }
    }

    let txns: Vec<_> = plan2
        .into_iter()
        .map(|(tx_id, key)| {
            (tx_id, move |txn: &mut Txn| -> anyhow::Result<i32> {
                let val = txn
                    .read(&key)
                    .and_then(|v| String::from_utf8(v.clone()).ok())
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);
                let new_val = val + 1;
                txn.write(key.clone(), new_val.to_string().into_bytes());
                Ok(new_val as i32)
            })
        })
        .collect();

    let batch = scheduler.execute_batch(txns);
    let dur = start.elapsed();

    let success_rate = pct(batch.successful, batch.successful + batch.failed);

    println!("\n==== MVCC Scheduler 热点冲突测试报告 ====");
    println!("总交易数: {}", total);
    println!("成功交易: {} ({:.2}%)", batch.successful, success_rate);
    println!("失败交易: {}", batch.failed);
    println!("冲突次数: {}", batch.conflicts);
    println!("运行时间: {:.2} 秒", dur.as_secs_f64());

    // 至少应当出现冲突，同时仍有大量成功
    assert!(batch.conflicts > 0, "expected conflicts > 0");
    assert!(batch.successful > batch.failed, "too many failures");
}
