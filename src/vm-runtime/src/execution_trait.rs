// execution_trait.rs - 统一执行引擎接口 (L1 扩展层)
//
// 模块级别: L1 (Extension Layer)
// 架构作用: 连接 L0 核心层与 L2 适配器层的关键桥梁
//
// 设计理念:
// - 向下封装: 将 L0 的 WASM 执行能力抽象为统一接口
// - 向上暴露: 为 L2 EVM Adapter 等插件提供标准化的执行接口
// - 多引擎: 支持 WASM/EVM/其他虚拟机的统一抽象
//
// 修改此文件需要:
// - 1 名 Maintainer 审批
// - 单元测试覆盖率 90%+
// - 性能测试通过
//
// 版本: v0.1.0
// 创建日期: 2025-11-05

use anyhow::Result;

/// 执行引擎类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    Wasm,
    Evm,
}

/// 执行上下文
pub struct ExecutionContext {
    pub caller: [u8; 20],
    pub contract: [u8; 20],
    pub value: u128,
    pub gas_limit: u64,
    pub block_number: u64,
    pub timestamp: u64,
}

/// 日志事件
#[derive(Debug, Clone)]
pub struct Log {
    pub address: [u8; 20],
    pub topics: Vec<[u8; 32]>,
    pub data: Vec<u8>,
}

/// 状态变更
#[derive(Debug, Clone)]
pub struct StateChange {
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>, // None = delete
}

/// 合约执行结果 (区别于 parallel::ExecutionResult)
pub struct ContractResult {
    pub success: bool,
    pub return_data: Vec<u8>,
    pub gas_used: u64,
    pub logs: Vec<Log>,
    pub state_changes: Vec<StateChange>,
}

/// 统一执行引擎 trait
///
/// 为不同的虚拟机实现提供统一接口:
/// - WasmExecutor: WASM 字节码执行
/// - EvmAdapter: EVM 字节码执行 (通过 revm)
pub trait ExecutionEngine: Send + Sync {
    /// 执行合约代码
    fn execute(
        &self,
        code: &[u8],
        input: &[u8],
        context: &ExecutionContext,
    ) -> Result<ContractResult>;

    /// 获取引擎类型
    fn engine_type(&self) -> EngineType;

    /// 验证代码格式
    fn validate_code(&self, code: &[u8]) -> Result<()>;
}
