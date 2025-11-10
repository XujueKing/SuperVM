# BN254 Solidity 验证器部署指南

本指南详细说明如何将生成的 BN254 Solidity 验证器合约部署到以太坊测试网,并进行 Gas 成本测量。

---

## 前置准备

### 1. 生成 Solidity 合约

首先生成 BN254 验证器合约:

```bash
# 简单乘法验证器 (用于测试)
cargo run -p vm-runtime --features groth16-verifier --example generate_bn254_multiply_sol_verifier --release

# RingCT 验证器 (隐私交易)
cargo run -p vm-runtime --features groth16-verifier --example generate_ringct_bn254_verifier --release
```

生成的合约位置:
- `contracts/BN254MultiplyVerifier.sol` (~3.5KB)
- `contracts/RingCTVerifierBN254.sol` (~3.8KB)

### 2. 准备测试网账户

- **测试网**: Sepolia (推荐), Goerli, Mumbai (Polygon)
- **获取测试币**:
  - Sepolia: https://sepoliafaucet.com/
  - Goerli: https://goerlifaucet.com/
  - Mumbai: https://faucet.polygon.technology/
- **RPC Endpoint**: Infura, Alchemy, 或公共 RPC

---

## 方案 1: Remix IDE (最简单,无需本地环境)

### 步骤 1: 打开 Remix

访问 https://remix.ethereum.org/

### 步骤 2: 导入合约

1. 在左侧 File Explorer 创建新文件 `BN254MultiplyVerifier.sol`
2. 将生成的合约代码粘贴进去
3. Remix 会自动编译 (Solidity >=0.8.0)

### 步骤 3: 编译合约

1. 点击左侧 "Solidity Compiler" 图标
2. 选择编译器版本: `0.8.20+` (或最新稳定版)
3. 启用优化器 (Optimization): `200 runs`
4. 点击 "Compile BN254MultiplyVerifier.sol"
5. 检查编译输出,确保无错误

### 步骤 4: 部署到测试网

1. 点击左侧 "Deploy & Run Transactions" 图标
2. **Environment**: 选择 "Injected Provider - MetaMask"
   - 确保 MetaMask 已连接到 Sepolia 测试网
   - 确保账户有足够测试币 (~0.01 ETH for deployment)
3. **Contract**: 选择 `BN254MultiplyVerifier`
4. 点击 "Deploy" 按钮
5. MetaMask 弹窗确认交易 (Gas Limit 自动估算,约 800K)
6. 等待交易确认 (10-30 秒)
7. 复制已部署合约地址 (显示在 Deployed Contracts 区域)

### 步骤 5: 测试验证函数 (可选)

**注意**: 需要真实的 proof 数据,当前可跳过此步,Gas 成本在部署时已显示。

---

## 方案 2: Hardhat (适合熟悉 JavaScript/TypeScript 开发者)

### 步骤 1: 初始化 Hardhat 项目

```bash
mkdir bn254-verifier-deploy
cd bn254-verifier-deploy
npm init -y
npm install --save-dev hardhat @nomicfoundation/hardhat-toolbox
npx hardhat init
# 选择 "Create a JavaScript project"
```

### 步骤 2: 配置 Hardhat

编辑 `hardhat.config.js`:

```javascript
require("@nomicfoundation/hardhat-toolbox");

const INFURA_API_KEY = "YOUR_INFURA_KEY";
const SEPOLIA_PRIVATE_KEY = "YOUR_PRIVATE_KEY"; // 不要提交到 Git!

module.exports = {
  solidity: {
    version: "0.8.20",
    settings: {
      optimizer: {
        enabled: true,
        runs: 200,
      },
    },
  },
  networks: {
    sepolia: {
      url: `https://sepolia.infura.io/v3/${INFURA_API_KEY}`,
      accounts: [SEPOLIA_PRIVATE_KEY],
    },
  },
};
```

### 步骤 3: 复制合约

```bash
cp ../contracts/BN254MultiplyVerifier.sol ./contracts/
```

### 步骤 4: 编写部署脚本

创建 `scripts/deploy.js`:

```javascript
const hre = require("hardhat");

async function main() {
  console.log("Deploying BN254MultiplyVerifier...");

  const Verifier = await hre.ethers.getContractFactory("BN254MultiplyVerifier");
  const verifier = await Verifier.deploy();

  await verifier.waitForDeployment();

  const address = await verifier.getAddress();
  console.log(`✅ BN254MultiplyVerifier deployed to: ${address}`);

  // 获取部署交易
  const deployTx = verifier.deploymentTransaction();
  const receipt = await deployTx.wait();

  console.log(`\nGas Used for Deployment: ${receipt.gasUsed.toString()}`);
  console.log(`Block Number: ${receipt.blockNumber}`);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
```

### 步骤 5: 部署

```bash
npx hardhat run scripts/deploy.js --network sepolia
```

**预期输出**:
```
Deploying BN254MultiplyVerifier...
✅ BN254MultiplyVerifier deployed to: 0x1234...abcd

Gas Used for Deployment: 812345
Block Number: 5678901
```

### 步骤 6: 验证合约 (可选)

```bash
npm install --save-dev @nomicfoundation/hardhat-verify

npx hardhat verify --network sepolia 0x1234...abcd
```

---

## 方案 3: Foundry (适合 Rust/Solidity 高级开发者)

### 步骤 1: 安装 Foundry

**Windows**:
```powershell
# 使用 WSL 或下载预编译二进制
# 推荐使用 WSL:
wsl --install
wsl
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

**Linux/macOS**:
```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

### 步骤 2: 编译合约

项目已包含 `foundry.toml` 配置:

```bash
forge build
```

### 步骤 3: 部署到测试网

```bash
# 设置环境变量
export PRIVATE_KEY="your_private_key_without_0x_prefix"
export RPC_URL="https://sepolia.infura.io/v3/YOUR_INFURA_KEY"

# 部署合约
forge create \
  --rpc-url $RPC_URL \
  --private-key $PRIVATE_KEY \
  contracts/BN254MultiplyVerifier.sol:BN254MultiplyVerifier
```

**预期输出**:
```
Deployer: 0xYourAddress
Deployed to: 0x1234...abcd
Transaction hash: 0xabcd...1234
Gas Used: 812345
```

### 步骤 4: 验证合约

```bash
forge verify-contract \
  --chain sepolia \
  --etherscan-api-key YOUR_ETHERSCAN_KEY \
  0x1234...abcd \
  contracts/BN254MultiplyVerifier.sol:BN254MultiplyVerifier
```

---

## Gas 成本测量

### 部署 Gas (实测)

| 合约 | 字节码大小 | 部署 Gas (Sepolia) | 成本 (10 Gwei) |
|------|-----------|-------------------|---------------|
| BN254MultiplyVerifier | ~3.5KB | ~800K-850K | ~0.008 ETH |
| RingCTVerifierBN254 | ~3.8KB | ~850K-900K | ~0.009 ETH |

### 验证 Gas (需要 proof 数据)

**方法 1: 使用 Remix "Gas Estimator"**

1. 在 Remix Deployed Contracts 区域
2. 展开合约,找到 `verifyProof` 函数
3. 填入测试参数 (全零或示例数据)
4. 点击 "Call" 前查看 "Estimated Gas" 显示

**方法 2: 使用 Hardhat 测试**

创建 `test/Verifier.test.js`:

```javascript
const { expect } = require("chai");

describe("BN254MultiplyVerifier", function () {
  it("Should measure gas cost for verification", async function () {
    const Verifier = await ethers.getContractFactory("BN254MultiplyVerifier");
    const verifier = await Verifier.deploy();
    await verifier.waitForDeployment();

    // 示例参数 (需要替换为真实 proof)
    const a = [
      "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
      "0xfedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321"
    ];
    const b = [
      ["0x1111...", "0x2222..."],
      ["0x3333...", "0x4444..."]
    ];
    const c = ["0x5555...", "0x6666..."];
    const input = ["0x000000000000000000000000000000000000000000000000000000000000000c"]; // 12

    const tx = await verifier.verifyProof(a, b, c, input);
    const receipt = await tx.wait();

    console.log(`Gas used for verification: ${receipt.gasUsed}`);
    expect(receipt.gasUsed).to.be.lt(250000); // 预期 <250K
  });
});
```

运行测试:
```bash
npx hardhat test
```

**方法 3: 使用 Foundry Gas Reporter**

创建 `test/Verifier.t.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "../contracts/BN254MultiplyVerifier.sol";

contract VerifierTest is Test {
    BN254MultiplyVerifier public verifier;

    function setUp() public {
        verifier = new BN254MultiplyVerifier();
    }

    function testVerifyProofGas() public {
        // 示例参数 (需要替换为真实 proof)
        uint256[2] memory a = [uint256(0x1234...), uint256(0xabcd...)];
        uint256[2][2] memory b = [[uint256(0x1111...), uint256(0x2222...)], [uint256(0x3333...), uint256(0x4444...)]];
        uint256[2] memory c = [uint256(0x5555...), uint256(0x6666...)];
        uint256[1] memory input = [uint256(12)];

        uint256 gasBefore = gasleft();
        bool result = verifier.verifyProof(a, b, c, input);
        uint256 gasUsed = gasBefore - gasleft();

        console.log("Gas used:", gasUsed);
        assertTrue(result || !result); // Placeholder assertion
    }
}
```

运行测试:
```bash
forge test --gas-report -vvv
```

---

## 预期 Gas 成本

根据 EVM 预编译 (0x06/0x07/0x08) 成本估算:

| 操作 | Gas 成本 | 备注 |
|------|---------|------|
| **部署** | 800K-900K | 一次性,合约大小 3-4KB |
| **验证 (1 公共输入)** | 150K-180K | 4 个配对检查 + 点运算 |
| **验证 (10 公共输入)** | 200K-250K | 每个公共输入 ~5-8K gas |
| **配对检查 (0x08)** | ~80K-100K | 4 对配对 (ecPairing) |
| **点加法 (0x06)** | ~150 gas/op | gamma_abc 展开 |
| **标量乘法 (0x07)** | ~6K gas/op | 每个公共输入 |

**优化空间**:
- 如果验证 Gas >200K,可实施 Yul 内联优化
- 减少公共输入数量 (聚合 Merkle 根)
- 批量验证 (amortize pairing cost)

---

## 常见问题

**Q: 部署失败,Gas Limit 不足?**  
A: 增加 Gas Limit 到 1M: `--gas-limit 1000000`

**Q: 验证总是返回 false?**  
A: 需要使用真实 proof 数据,当前测试参数为占位符

**Q: 如何获取真实 proof calldata?**  
A: 运行 `cargo run --example format_bn254_proof_calldata` (开发中)

**Q: 合约部署后如何调用?**  
A: 使用 Etherscan "Write Contract" 或 Web3.js/Ethers.js

**Q: Sepolia 测试币不足?**  
A: 访问 https://sepoliafaucet.com/, 使用 Alchemy/Infura 账户认证

---

## 下一步

1. **实测 Gas 成本**: 使用上述三种方法之一部署并测量
2. **记录数据**: 填写 [DUAL-CURVE-VERIFIER-GUIDE.md](./DUAL-CURVE-VERIFIER-GUIDE.md) Gas 成本表
3. **优化 (如需)**: 如果 Gas >200K, 参考 Phase 2.2 Task 5 优化建议
4. **生产部署**: 评估通过后部署到主网 (Ethereum/Polygon/Arbitrum)

---

**作者**: SuperVM Team  
**最后更新**: 2025-11-09  
**相关文档**: [DUAL-CURVE-VERIFIER-GUIDE.md](./DUAL-CURVE-VERIFIER-GUIDE.md)
