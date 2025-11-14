//! L2 Runtime - 运行时后端管理
//!
//! 提供统一的运行时接口,支持自动后端选择和智能降级。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::zkvm::TraceZkVm;

#[cfg(feature = "risc0-poc")]
use crate::risc0_backend::Risc0Backend;

/// 支持的 zkVM 后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    /// Trace zkVM (默认,跨平台)
    Trace,
    
    /// RISC0 zkVM (Linux/WSL only)
    #[cfg_attr(not(all(feature = "risc0-poc", not(target_os = "windows"))), serde(skip))]
    Risc0,
    
    /// Halo2 zkVM (未来支持)
    #[serde(skip)]
    Halo2,
}

impl Default for BackendType {
    fn default() -> Self {
        #[cfg(all(feature = "risc0-poc", not(target_os = "windows")))]
        {
            BackendType::Risc0
        }
        
        #[cfg(not(all(feature = "risc0-poc", not(target_os = "windows"))))]
        {
            BackendType::Trace
        }
    }
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::Trace => write!(f, "Trace"),
            BackendType::Risc0 => write!(f, "RISC0"),
            BackendType::Halo2 => write!(f, "Halo2"),
        }
    }
}

/// L2 运行时配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// 后端类型 (None 表示自动选择)
    pub backend: Option<BackendType>,
    
    /// 是否启用日志
    #[serde(default = "default_enable_logging")]
    pub enable_logging: bool,
    
    /// 是否在开发模式 (RISC0_DEV_MODE)
    #[serde(default)]
    pub dev_mode: bool,
}

fn default_enable_logging() -> bool {
    true
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            backend: None, // 自动选择
            enable_logging: true,
            dev_mode: std::env::var("RISC0_DEV_MODE")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false),
        }
    }
}

/// L2 运行时实例
///
/// 封装 zkVM 后端的统一接口,支持:
/// - 自动后端选择 (Windows → Trace, Linux+feature → RISC0)
/// - 手动指定后端
/// - 运行时配置加载
pub struct L2Runtime {
    backend_type: BackendType,
    config: RuntimeConfig,
}

impl L2Runtime {
    /// 创建指定后端的运行时
    ///
    /// # 参数
    /// - `backend_type`: 后端类型
    ///
    /// # 返回
    /// - `Ok(runtime)`: 成功创建
    /// - `Err(e)`: 后端不可用或初始化失败
    pub fn new(backend_type: BackendType) -> Result<Self> {
        Self::with_config(backend_type, RuntimeConfig::default())
    }

    /// 使用配置创建运行时
    pub fn with_config(backend_type: BackendType, config: RuntimeConfig) -> Result<Self> {
        // 验证后端可用性
        match backend_type {
            BackendType::Trace => {
                // Trace backend 总是可用
                if config.enable_logging {
                    log::info!("Initializing Trace zkVM backend (cross-platform)");
                }
            }
            
            #[cfg(all(feature = "risc0-poc", not(target_os = "windows")))]
            BackendType::Risc0 => {
                if config.enable_logging {
                    let mode = if config.dev_mode { "DEV" } else { "PRODUCTION" };
                    log::info!("Initializing RISC0 zkVM backend (mode: {})", mode);
                }
            }
            
            #[cfg(not(all(feature = "risc0-poc", not(target_os = "windows"))))]
            BackendType::Risc0 => {
                anyhow::bail!(
                    "RISC0 backend requires Linux/WSL and risc0-poc feature. \
                     Current platform: {} (feature enabled: {})",
                    std::env::consts::OS,
                    cfg!(feature = "risc0-poc")
                );
            }
            
            BackendType::Halo2 => {
                anyhow::bail!("Halo2 backend not yet implemented");
            }
        }

        Ok(Self {
            backend_type,
            config,
        })
    }

    /// 自动选择最佳后端
    ///
    /// 选择策略:
    /// 1. Linux + risc0-poc feature → RISC0
    /// 2. 其他平台或无 feature → Trace
    pub fn auto_select() -> Result<Self> {
        let backend_type = BackendType::default();
        let config = RuntimeConfig::default();
        
        if config.enable_logging {
            log::info!(
                "Auto-selected backend: {} (platform: {}, risc0-poc: {})",
                backend_type,
                std::env::consts::OS,
                cfg!(feature = "risc0-poc")
            );
        }
        
        Self::with_config(backend_type, config)
    }

    /// 从配置文件加载运行时
    ///
    /// # 参数
    /// - `config_path`: 配置文件路径 (TOML 格式)
    ///
    /// # 示例配置
    /// ```toml
    /// backend = "risc0"  # 或 "trace", null (自动选择)
    /// enable_logging = true
    /// dev_mode = false
    /// ```
    pub fn from_config_file(config_path: &str) -> Result<Self> {
        let config_str = std::fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path))?;
        
        let config: RuntimeConfig = toml::from_str(&config_str)
            .with_context(|| format!("Failed to parse config file: {}", config_path))?;
        
        let backend_type = config.backend.unwrap_or_default();
        
        Self::with_config(backend_type, config)
    }

    /// 获取当前后端类型
    pub fn backend_type(&self) -> BackendType {
        self.backend_type
    }

    /// 获取运行时配置
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// 检查后端是否可用
    pub fn is_backend_available(backend_type: BackendType) -> bool {
        match backend_type {
            BackendType::Trace => true,
            
            #[cfg(all(feature = "risc0-poc", not(target_os = "windows")))]
            BackendType::Risc0 => true,
            
            #[cfg(not(all(feature = "risc0-poc", not(target_os = "windows"))))]
            BackendType::Risc0 => false,
            
            BackendType::Halo2 => false,
        }
    }

    /// 列出所有可用后端
    pub fn available_backends() -> Vec<BackendType> {
        let backends = vec![BackendType::Trace];
        
        #[cfg(all(feature = "risc0-poc", not(target_os = "windows")))]
        return vec![BackendType::Trace, BackendType::Risc0];
        
        #[cfg(not(all(feature = "risc0-poc", not(target_os = "windows"))))]
        backends
    }

    /// 创建 Trace zkVM 实例
    pub fn create_trace_vm(&self) -> TraceZkVm {
        TraceZkVm::default()
    }

    /// 创建 RISC0 zkVM 实例
    #[cfg(all(feature = "risc0-poc", not(target_os = "windows")))]
    pub fn create_risc0_vm(&self) -> Risc0Backend {
        Risc0Backend::new()
    }
}

impl std::fmt::Debug for L2Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("L2Runtime")
            .field("backend_type", &self.backend_type)
            .field("dev_mode", &self.config.dev_mode)
            .field("enable_logging", &self.config.enable_logging)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_select_creates_runtime() {
        let runtime = L2Runtime::auto_select().expect("auto_select failed");
        
        // 验证返回了预期的后端
        #[cfg(all(feature = "risc0-poc", not(target_os = "windows")))]
        assert_eq!(runtime.backend_type(), BackendType::Risc0);
        
        #[cfg(not(all(feature = "risc0-poc", not(target_os = "windows"))))]
        assert_eq!(runtime.backend_type(), BackendType::Trace);
    }

    #[test]
    fn test_trace_backend_always_available() {
        assert!(L2Runtime::is_backend_available(BackendType::Trace));
        
        let runtime = L2Runtime::new(BackendType::Trace).expect("Trace backend failed");
        assert_eq!(runtime.backend_type(), BackendType::Trace);
    }

    #[test]
    #[cfg(all(feature = "risc0-poc", not(target_os = "windows")))]
    fn test_risc0_backend_available_on_linux() {
        assert!(L2Runtime::is_backend_available(BackendType::Risc0));
        
        let runtime = L2Runtime::new(BackendType::Risc0).expect("RISC0 backend failed");
        assert_eq!(runtime.backend_type(), BackendType::Risc0);
    }

    #[test]
    #[cfg(not(all(feature = "risc0-poc", not(target_os = "windows"))))]
    fn test_risc0_backend_unavailable_on_windows() {
        assert!(!L2Runtime::is_backend_available(BackendType::Risc0));
        
        let result = L2Runtime::new(BackendType::Risc0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("RISC0 backend requires"));
    }

    #[test]
    fn test_available_backends_includes_trace() {
        let backends = L2Runtime::available_backends();
        assert!(backends.contains(&BackendType::Trace));
    }

    #[test]
    fn test_config_default_values() {
        let config = RuntimeConfig::default();
        assert_eq!(config.backend, None); // 自动选择
        assert_eq!(config.enable_logging, true);
    }

    #[test]
    fn test_backend_type_display() {
        assert_eq!(BackendType::Trace.to_string(), "Trace");
        
        #[cfg(feature = "risc0-poc")]
        assert_eq!(BackendType::Risc0.to_string(), "RISC0");
    }

    #[test]
    fn test_create_trace_vm() {
        let runtime = L2Runtime::new(BackendType::Trace).unwrap();
        let vm = runtime.create_trace_vm();
        
        // 验证 VM 可以正常使用
        use crate::program::FibonacciProgram;
        let program = FibonacciProgram::new(5);
        let witness = vec![0, 1];
        let proof = vm.prove(&program, &witness).expect("prove failed");
        assert_eq!(proof.public_outputs, vec![5]); // fib(5) = 5
    }
}
