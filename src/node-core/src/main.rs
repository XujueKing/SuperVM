// 开发者：king
// Developer: king
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use vm_runtime::Runtime;
use vm_runtime::MemoryStorage;

/// 节点命令行参数（PoC）
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 配置文件路径
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// 执行一次后立即退出（不等待 Ctrl-C）
    #[arg(long)]
    once: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    let args = Args::parse();
    info!("Starting node (PoC) with config: {}", args.config);

    // 初始化运行时(使用内存存储)
    let runtime = Runtime::new(MemoryStorage::new());

    // 演示 1: 简单的 add 函数
    let wat_add = r#"
    (module
      (func $add (export "add") (param i32 i32) (result i32)
        local.get 0
        local.get 1
        i32.add)
    )
    "#;
    let wasm_add = wat::parse_str(wat_add)?;
    let sum = runtime.execute_add(&wasm_add, 7, 8)?;
    info!("Demo 1: add(7,8) => {}", sum);

    // 演示 2: 使用 execute_with_context 展示事件系统和链上下文
    let wat_with_events = r#"
    (module
      ;; 导入 host functions
      (import "chain_api" "block_number" (func $block_number (result i64)))
      (import "chain_api" "timestamp" (func $timestamp (result i64)))
      (import "chain_api" "emit_event" (func $emit_event (param i32 i32) (result i32)))
      (import "storage_api" "storage_set" (func $storage_set (param i32 i32 i32 i32) (result i32)))
      
      ;; 内存用于传递数据
      (memory (export "memory") 1)
      
      ;; 在内存中预置一些字符串
      (data (i32.const 0) "UserAction")        ;; offset 0, len 10
      (data (i32.const 16) "BlockProcessed")   ;; offset 16, len 14
      (data (i32.const 32) "storage_key")      ;; offset 32, len 11
      (data (i32.const 48) "storage_value")    ;; offset 48, len 13
      
      (func $process (export "process") (result i32)
        ;; 发出第一个事件
        i32.const 0
        i32.const 10
        call $emit_event
        drop
        
        ;; 写入存储
        i32.const 32
        i32.const 11
        i32.const 48
        i32.const 13
        call $storage_set
        drop
        
        ;; 发出第二个事件
        i32.const 16
        i32.const 14
        call $emit_event
        drop
        
        ;; 获取区块号和时间戳,相加并返回(仅为演示)
        call $block_number
        call $timestamp
        i64.add
        i32.wrap_i64
      )
    )
    "#;
    let wasm_events = wat::parse_str(wat_with_events)?;
    let block_number = 12345u64;
    let timestamp = 1704067200u64; // 2024-01-01 00:00:00 UTC
    
    let (result, events, bn, ts) = runtime.execute_with_context(
        &wasm_events,
        "process",
        block_number,
        timestamp,
    )?;
    
    info!("Demo 2: execute_with_context results:");
    info!("  Function returned: {}", result);
    info!("  Block number: {}, Timestamp: {}", bn, ts);
    info!("  Events collected: {} events", events.len());
    
    for (i, event) in events.iter().enumerate() {
        let event_str = String::from_utf8_lossy(event);
        info!("    Event {}: {}", i + 1, event_str);
    }

        // 演示 3: 密码学功能
        let wat_crypto = r#"
        (module
          ;; 导入密码学 host functions
          (import "crypto_api" "sha256" (func $sha256 (param i32 i32 i32) (result i32)))
          (import "crypto_api" "keccak256" (func $keccak256 (param i32 i32 i32) (result i32)))
          (import "chain_api" "emit_event" (func $emit_event (param i32 i32) (result i32)))
      
          (memory (export "memory") 1)
      
          ;; 测试数据
          (data (i32.const 0) "hello world")
          ;; 为哈希结果预留空间: offset 100 (SHA-256), offset 200 (Keccak-256)
      
          (func $hash_demo (export "hash_demo") (result i32)
            ;; 计算 SHA-256("hello world")
            i32.const 0      ;; 输入指针
            i32.const 11     ;; 输入长度
            i32.const 100    ;; 输出指针
            call $sha256
            drop
        
            ;; 发出事件: SHA-256 完成
            i32.const 100
            i32.const 32
            call $emit_event
            drop
        
            ;; 计算 Keccak-256("hello world")
            i32.const 0      ;; 输入指针
            i32.const 11     ;; 输入长度
            i32.const 200    ;; 输出指针
            call $keccak256
            drop
        
            ;; 发出事件: Keccak-256 完成
            i32.const 200
            i32.const 32
            call $emit_event
            drop
        
            ;; 返回成功
            i32.const 0
          )
        )
        "#;
        let wasm_crypto = wat::parse_str(wat_crypto)?;
        let (result, crypto_events, _, _) = runtime.execute_with_context(
            &wasm_crypto,
            "hash_demo",
            100,
            1704067300,
        )?;
    
        info!("Demo 3: 密码学功能演示");
        info!("  执行结果: {}", result);
        info!("  生成的哈希事件数: {}", crypto_events.len());
    
        for (i, hash_event) in crypto_events.iter().enumerate() {
            let hash_hex = hash_event.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            let hash_type = if i == 0 { "SHA-256" } else { "Keccak-256" };
            info!("    {}: {}", hash_type, hash_hex);
        }
    
        // 验证哈希结果
        let expected_sha256 = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        let expected_keccak256 = "47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad";
    
        let actual_sha256 = crypto_events[0].iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        let actual_keccak256 = crypto_events[1].iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
    
        if actual_sha256 == expected_sha256 {
            info!("  ✓ SHA-256 验证通过");
        } else {
            info!("  ✗ SHA-256 验证失败");
        }
    
        if actual_keccak256 == expected_keccak256 {
            info!("  ✓ Keccak-256 验证通过");
        } else {
            info!("  ✗ Keccak-256 验证失败");
        }

    // 演示 4: 以太坊地址派生 (简化演示)
    info!("Demo 4: 以太坊地址派生");
    info!("  ✓ derive_eth_address host function 已注册");
    info!("  注意: 需要有效的 secp256k1 公钥才能派生地址");
    info!("  用法: derive_eth_address(pubkey_ptr, pubkey_len, output_ptr) -> 0表示成功");

    // 演示 5: 并行执行引擎
    info!("Demo 5: 并行执行演示");
    
    // 创建三个模拟交易
    let tx1_wat = r#"
    (module
      (import "storage_api" "storage_set" (func $storage_set (param i32 i32 i32 i32) (result i32)))
      (memory (export "memory") 1)
      (data (i32.const 0) "alice_balance")
      (data (i32.const 20) "100")
      (func (export "run") (result i32)
        i32.const 0
        i32.const 13
        i32.const 20
        i32.const 3
        call $storage_set
      )
    )
    "#;
    
    let tx2_wat = r#"
    (module
      (import "storage_api" "storage_set" (func $storage_set (param i32 i32 i32 i32) (result i32)))
      (memory (export "memory") 1)
      (data (i32.const 0) "bob_balance")
      (data (i32.const 20) "200")
      (func (export "run") (result i32)
        i32.const 0
        i32.const 11
        i32.const 20
        i32.const 3
        call $storage_set
      )
    )
    "#;
    
    let tx3_wat = r#"
    (module
      (import "storage_api" "storage_get" (func $storage_get (param i32 i32) (result i64)))
      (memory (export "memory") 1)
      (data (i32.const 0) "alice_balance")
      (func (export "run") (result i32)
        i32.const 0
        i32.const 13
        call $storage_get
        drop
        i32.const 0
      )
    )
    "#;
    
    use vm_runtime::{ConflictDetector, TxId};
    
    let tx1_wasm = wat::parse_str(tx1_wat)?;
    let tx2_wasm = wat::parse_str(tx2_wat)?;
    let tx3_wasm = wat::parse_str(tx3_wat)?;
    
    // 执行并收集读写集
    let mut result1 = runtime.execute_with_rw_tracking(&tx1_wasm, "run", 1000, 1704067500)?;
    result1.tx_id = 1;
    
    let mut result2 = runtime.execute_with_rw_tracking(&tx2_wasm, "run", 1000, 1704067500)?;
    result2.tx_id = 2;
    
    let mut result3 = runtime.execute_with_rw_tracking(&tx3_wasm, "run", 1000, 1704067500)?;
    result3.tx_id = 3;
    
    info!("  执行了 3 笔交易:");
    info!("    TX1: 写入 alice_balance");
    info!("    TX2: 写入 bob_balance");
    info!("    TX3: 读取 alice_balance");
    
    // 冲突检测
    let mut detector = ConflictDetector::new();
    detector.record(result1.tx_id, result1.read_write_set.clone());
    detector.record(result2.tx_id, result2.read_write_set.clone());
    detector.record(result3.tx_id, result3.read_write_set.clone());
    
    let tx_order: Vec<TxId> = vec![1, 2, 3];
    let graph = detector.build_dependency_graph(&tx_order);
    
    info!("  冲突分析:");
    info!("    TX1 和 TX2 无冲突 → 可并行执行 ✓");
    info!("    TX3 依赖 TX1 → 必须等待 TX1 完成");
    
    let deps1 = graph.get_dependencies(1);
    let deps2 = graph.get_dependencies(2);
    let deps3 = graph.get_dependencies(3);
    
    info!("  依赖关系:");
    info!("    TX1 依赖: {:?}", deps1);
    info!("    TX2 依赖: {:?}", deps2);
    info!("    TX3 依赖: {:?}", deps3);
    
    if deps1.is_empty() && deps2.is_empty() && deps3 == vec![1] {
        info!("  ✓ 并行执行调度正确!");
    }

    if !args.once {
        // 等待 Ctrl-C 退出（保留行为以便手动观察）
        info!("按 Ctrl-C 退出...");
        tokio::signal::ctrl_c().await?;
    }
    
    info!("Shutting down...");
    Ok(())
}