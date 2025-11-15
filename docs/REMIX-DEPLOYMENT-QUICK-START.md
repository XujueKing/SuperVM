# Remix IDE 部署实战指南 (5 分钟速成)

## 📋 准备清单

- [ ] 浏览器 (Chrome/Edge/Firefox)

- [ ] MetaMask 钱包扩展 (已安装并解锁)

- [ ] Sepolia 测试网账户 (至少 0.01 ETH)

- [ ] 合约文件: `contracts/BN254MultiplyVerifier.sol` (**5210 bytes, 已包含 helper 函数**)

---

## 🚀 部署步骤 (5 分钟)

### 步骤 1: 打开 Remix IDE (30 秒)

1. **浏览器访问**: https://remix.ethereum.org/
2. 等待 Remix 加载完成 (看到左侧文件树)
3. 关闭欢迎弹窗 (如有)

### 步骤 2: 导入合约 (1 分钟)

**方式 A: 文件上传 (推荐)**

1. 点击左侧 "File Explorer" 图标 📁
2. 点击 "contracts" 文件夹右键 → "Upload Files"
3. 选择本地文件:
   ```
   d:\WEB3_AI开发\虚拟机开发\contracts\BN254MultiplyVerifier.sol
   ```
4. 确认上传,文件出现在 `contracts/BN254MultiplyVerifier.sol`

**方式 B: 手动创建 (备用)**

1. 右键 "contracts" 文件夹 → "New File"
2. 命名: `BN254MultiplyVerifier.sol`
3. 打开本地文件,全选复制 (Ctrl+A, Ctrl+C)
4. 粘贴到 Remix 编辑器 (Ctrl+V)
5. 保存 (Ctrl+S)

### 步骤 3: 编译合约 (1 分钟)

1. **点击左侧 "Solidity Compiler" 图标** 🔨 (第二个图标)

2. **配置编译器**:
   - **Compiler**: 自动选择 `0.8.0+` (或手动选择最新稳定版,如 `0.8.20`)
   - **EVM Version**: `default` (保持默认)
   - **Advanced Configurations** (展开):
     - ✅ 启用 **"Enable optimization"**
     - **Runs**: 输入 `200`

3. **点击蓝色 "Compile BN254MultiplyVerifier.sol" 按钮**

4. **验证编译成功**:
   - 看到绿色对勾 ✅ "Compilation successful"
   - 编译器图标上显示绿色对勾
   - 无红色错误提示

**可能的警告 (可忽略)**:

```

Warning: SPDX license identifier not provided in source file...
Warning: This contract has a payable fallback function...

```

### 步骤 4: 配置 MetaMask (1 分钟)

1. **打开 MetaMask 扩展** (点击浏览器右上角图标)

2. **切换到 Sepolia 测试网**:
   - 点击顶部网络选择器 (默认可能是 "Ethereum Mainnet")
   - 下拉选择 "Sepolia test network"
   - 如果没有,点击 "Show/hide test networks" → 启用 "Show test networks"

3. **检查余额**:
   - 确保至少有 **0.01 ETH** (部署大约需要 0.003-0.005 ETH)
   - 如果余额不足,访问水龙头: https://sepoliafaucet.com/

4. **准备私钥 (可选,建议用新账户)**:
   - 如果担心安全,在 MetaMask 创建新账户: "Create Account"
   - 专用于测试,避免使用主账户

### 步骤 5: 部署合约 (2 分钟)

1. **点击左侧 "Deploy & Run Transactions" 图标** 🚀 (第三个图标)

2. **配置部署环境**:
   - **Environment**: 选择 **"Injected Provider - MetaMask"**
     - 如果弹出 MetaMask 连接请求,点击 "Connect" (连接当前账户)
     - 确认 Remix 显示 "Custom (11155111) network" (Sepolia chain ID)
   
3. **选择合约**:
   - **Contract**: 下拉选择 **`BN254MultiplyVerifier - contracts/BN254MultiplyVerifier.sol`**

4. **Gas 配置 (通常自动)**:
   - **Gas Limit**: 自动估算 (~800,000 - 900,000)
   - **Value**: 保持 `0` (合约不需要 ETH)

5. **点击橙色 "Deploy" 按钮** 🟧

6. **MetaMask 确认**:
   - MetaMask 弹窗显示交易详情:
     - **Gas (Gwei)**: 通常 10-50 Gwei (自动)
     - **Gas Limit**: ~850,000
     - **Max fee**: ~0.005 ETH
   - **点击 "Confirm"** ✅

7. **等待部署**:
   - Remix 底部显示 "creation of BN254MultiplyVerifier pending..."
   - 等待 10-30 秒 (取决于网络拥堵)
   - 成功后显示 **绿色对勾 ✅** "creation of BN254MultiplyVerifier"

8. **记录合约地址**:
   - 在 "Deployed Contracts" 区域,展开合约
   - 复制合约地址 (格式: `0x1234...abcd`)
   - 保存到记事本,稍后需要

**示例输出**:

```

creation of BN254MultiplyVerifier pending...
[block:5678901] from: 0xYourAddress to: BN254MultiplyVerifier.(constructor) value: 0 wei
✅ Gas used: 847,234
📍 Contract address: 0x1234567890abcdef1234567890abcdef12345678

```

---

## 📊 Gas 数据记录

**请填写以下信息** (从 Remix 或 MetaMask 获取):

```yaml
部署信息:
  合约名称: BN254MultiplyVerifier
  合约地址: 0x___________________________
  部署交易哈希: 0x___________________________
  部署区块: _______
  部署时间: 2025-11-09 __:__

Gas 成本:
  Gas Used: _______ (例如: 847,234)
  Gas Price (Gwei): _______ (例如: 25)
  总成本 (ETH): _______ (例如: 0.0021)
  总成本 (USD): _______ (假设 ETH=$2000: $4.2)

网络信息:
  测试网: Sepolia
  Chain ID: 11155111
  区块浏览器: https://sepolia.etherscan.io/address/0x___

```

**在 Etherscan 查看**:
1. 访问: https://sepolia.etherscan.io/
2. 搜索框输入合约地址
3. 查看 "Contract Creation" 交易
4. 记录精确 Gas Used 值

---

## 🧪 测试合约 (可选,5 分钟)

### 方式 1: Remix 直接调用 (需要 proof 数据)

**注意**: 当前没有真实 proof,此步骤仅演示流程,可跳过。

1. 在 "Deployed Contracts" 区域,展开 `BN254MultiplyVerifier`
2. 找到 `verifyProof` 函数 (橙色按钮,表示需要 gas)
3. 点击展开,看到 4 个输入框:
   - `a`: uint256[2]
   - `b`: uint256[2][2]
   - `c`: uint256[2]
   - `input`: uint256[1]

**示例占位符数据** (无效 proof,仅测试接口):

```javascript
// a (G1 点)
["0x0000000000000000000000000000000000000000000000000000000000000001", "0x0000000000000000000000000000000000000000000000000000000000000002"]

// b (G2 点)
[["0x0000000000000000000000000000000000000000000000000000000000000001", "0x0000000000000000000000000000000000000000000000000000000000000002"], ["0x0000000000000000000000000000000000000000000000000000000000000003", "0x0000000000000000000000000000000000000000000000000000000000000004"]]

// c (G1 点)
["0x0000000000000000000000000000000000000000000000000000000000000005", "0x0000000000000000000000000000000000000000000000000000000000000006"]

// input (公共输入: 12 = 3*4)
["0x000000000000000000000000000000000000000000000000000000000000000c"]

```

4. 点击 "transact" (会消耗 gas,约 150K-200K)
5. MetaMask 确认交易
6. 查看返回值 (大概率是 `false`,因为 proof 无效)

### 方式 2: Etherscan "Write Contract" (推荐用于真实 proof)

1. 访问 Etherscan 合约页面
2. "Contract" 标签 → "Write Contract"
3. 连接 MetaMask → "Connect to Web3"
4. 找到 `verifyProof` 函数
5. 输入真实 proof 数据
6. 点击 "Write" → 确认交易
7. 查看 "Logs" 获取验证结果

---

## ✅ 完成检查清单

- [ ] 合约编译成功 (绿色对勾)

- [ ] MetaMask 连接 Sepolia

- [ ] 合约部署成功 (获得地址)

- [ ] 记录 Gas Used 值

- [ ] 在 Etherscan 查看合约

- [ ] (可选) 测试 verifyProof 函数

---

## 📝 下一步操作

### 1. 更新文档

将实测 Gas 数据填入 `docs/DUAL-CURVE-VERIFIER-GUIDE.md`:

```bash

# 编辑文件

code docs\DUAL-CURVE-VERIFIER-GUIDE.md

# 找到 "Gas 成本对比" 章节,更新表格:

| 操作 | Gas 成本 (理论) | Gas 成本 (实测 Sepolia) | 备注 |
|------|----------------|----------------------|------|
| 合约部署 | ~800K | ______ | 实测: 2025-11-09 |
| 单次验证 (1 公共输入) | ~150K-180K | ______ | 需真实 proof |

```

### 2. 提交代码 (可选)

```bash
git add contracts/BN254MultiplyVerifier.sol
git add docs/DEPLOYMENT-GUIDE.md
git add docs/DUAL-CURVE-VERIFIER-GUIDE.md
git commit -m "feat: BN254 Solidity verifier deployed to Sepolia testnet

- Contract address: 0x___

- Deployment gas: ___ 

- Remix IDE deployment successful"

```

### 3. 继续 Phase 2.2 任务

- **Task 3**: 批量验证集成 SuperVM PrivacyPath

- **Task 4**: 24 小时稳定性测试

- **Task 5**: Grafana 生产环境配置

---

## 🆘 常见问题

**Q: MetaMask 没有 "Injected Provider" 选项?**  
A: 刷新 Remix 页面,确保 MetaMask 已解锁。

**Q: 部署失败 "out of gas"?**  
A: 增加 Gas Limit 到 1,000,000 (在 MetaMask 交易确认时手动编辑)。

**Q: 合约地址在哪里看?**  
A: Remix "Deployed Contracts" 区域,或 MetaMask 交易详情 "Contract Address"。

**Q: Sepolia 测试币不够?**  
A: 访问 https://sepoliafaucet.com/, 使用 Alchemy 账户认证可获得更多。

**Q: 如何验证合约源码?**  
A: Etherscan "Contract" → "Verify and Publish" → 粘贴源码 + 编译器版本 (0.8.20) + 优化 (200 runs)。

---

**预计耗时**: 5-10 分钟  
**难度**: ⭐☆☆☆☆ (零基础可完成)  
**成本**: ~$5-10 (假设 ETH=$2000, gas=25 Gwei)

**准备好了吗?** 打开 https://remix.ethereum.org/ 开始部署! 🚀
