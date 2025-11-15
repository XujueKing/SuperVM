# 引用与致谢 (Attributions)

开发者/作者：King Xujue

本文档集中列出 SuperVM 项目文档中引用的外部论文、开源项目、技术资料及相关致谢，确保合规与透明。

---

## 📚 学术论文与白皮书

### 零知识证明 (zkSNARK)

1. **Groth16: On the Size of Pairing-based Non-interactive Arguments**
   - 作者：Jens Groth
   - 链接：https://eprint.iacr.org/2016/260
   - 引用文档：
     - `docs/research/groth16-study.md`
     - `docs/design/ringct-circuit-design.md`
     - `zk-groth16-test/` 系列报告

2. **Halo: Recursive Proof Composition without a Trusted Setup**
   - 作者：Sean Bowe, Jack Grigg, Daira Hopwood
   - 链接：https://eprint.iacr.org/2019/1021
   - 引用文档：
     - `docs/research/halo2-eval-summary.md`
     - `halo2-eval/README.md`

3. **PLONK: Permutations over Lagrange-bases for Oecumenical Noninteractive arguments of Knowledge**
   - 链接：https://eprint.iacr.org/2019/953
   - 引用文档：
     - `docs/research/zk-evaluation.md`
     - `docs/research/halo2-eval-summary.md`

### 隐私加密技术

4. **CryptoNote v2.0 白皮书**
   - 作者：Nicolas van Saberhagen
   - 链接：https://cryptonote.org/whitepaper.pdf
   - 引用文档：
     - `docs/research/cryptonote-whitepaper-notes.md`
     - `docs/research/monero-study-notes.md`

5. **Ring Confidential Transactions (RingCT)**
   - 链接：https://eprint.iacr.org/2015/1098
   - 引用文档：
     - `docs/design/ringct-circuit-design.md`
     - `docs/research/monero-study-notes.md`

6. **Bulletproofs: Short Proofs for Confidential Transactions**
   - 链接：https://eprint.iacr.org/2017/1066
   - 引用文档：
     - `docs/design/ringct-circuit-design.md`
     - `docs/research/64bit-range-proof-summary.md`

7. **CLSAG: Concise Linkable Spontaneous Anonymous Group Signatures**
   - 链接：https://eprint.iacr.org/2020/018
   - 引用文档：
     - `docs/research/monero-study-notes.md`
     - `zk-groth16-test/RING_SIGNATURE_REPORT.md`

8. **Zero to Monero (第二版)**
   - 链接：https://web.getmonero.org/library/Zero-to-Monero-2-0-0.pdf
   - 引用文档：
     - `docs/research/monero-study-notes.md`
     - `docs/research/cryptonote-whitepaper-notes.md`

9. **Ristretto: Prime Order Elliptic Curve Groups**
   - 作者：Mike Hamburg
   - 链接：https://eprint.iacr.org/2015/673
   - 引用文档：
     - `docs/research/curve25519-dalek-notes.md`

10. **Linkable Spontaneous Anonymous Group Signature (LSAG)**
    - 链接：https://www.semanticscholar.org/paper/Linkable-Spontaneous-Anonymous-Group-Signature-for-Liu-Wei/45b1fa0f4b35d8c5aeb3e11c67de90c52e063e68
    - 引用文档：
      - `docs/design/ringct-circuit-design.md`

---

## 💻 开源项目与库

### zkSNARK 实现

1. **arkworks-rs (arkworks Ecosystem)**
   - 仓库：https://github.com/arkworks-rs/
   - 许可：Apache-2.0 / MIT
   - 用途：Groth16 电路实现、BLS12-381 曲线、R1CS 约束系统
   - 引用文档：
     - `zk-groth16-test/` 全部实现
     - `docs/research/groth16-poc-summary.md`
     - `docs/research/zk-evaluation.md`

2. **Halo2 (Zcash)**
   - 仓库：https://github.com/zcash/halo2
   - 许可：Apache-2.0 / MIT
   - 用途：Halo2 PLONK 电路评估
   - 引用文档：
     - `halo2-eval/` 全部实现
     - `docs/research/halo2-eval-summary.md`

3. **bellman (Zcash - Groth16)**
   - 仓库：https://github.com/zkcrypto/bellman
   - 许可：Apache-2.0 / MIT
   - 用途：Groth16 原理学习参考
   - 引用文档：
     - `docs/research/groth16-study.md`
     - `docs/research/zk-evaluation.md`

4. **librustzcash (Zcash Sapling)**
   - 仓库：https://github.com/zcash/librustzcash
   - 许可：Apache-2.0 / MIT
   - 用途：Sapling 协议与 Groth16 实现参考
   - 引用文档：
     - `docs/research/groth16-poc-summary.md`
     - `docs/design/ringct-circuit-design.md`

### 密码学原语

5. **Monero Project**
   - 仓库：https://github.com/monero-project/monero
   - 许可：BSD-3-Clause
   - 用途：Ring Signature、Stealth Address、CLSAG 实现参考
   - 引用文档：
     - `docs/research/monero-study-notes.md`
     - `docs/research/cryptonote-whitepaper-notes.md`
     - `docs/design/ringct-circuit-design.md`

6. **curve25519-dalek**
   - 仓库：https://github.com/dalek-cryptography/curve25519-dalek
   - 文档：https://docs.rs/curve25519-dalek/
   - 许可：BSD-3-Clause
   - 用途：Curve25519 / Ristretto 椭圆曲线操作学习
   - 引用文档：
     - `docs/research/curve25519-dalek-notes.md`

7. **bulletproofs (dalek)**
   - 仓库：https://github.com/dalek-cryptography/bulletproofs
   - 许可：MIT
   - 用途：Bulletproofs 范围证明参考
   - 引用文档：
     - `docs/research/curve25519-dalek-notes.md`

8. **ed25519-dalek**
   - 仓库：https://github.com/dalek-cryptography/ed25519-dalek
   - 许可：BSD-3-Clause
   - 用途：EdDSA 签名参考
   - 引用文档：
     - `docs/research/curve25519-dalek-notes.md`

### 其他评估库

9. **plonky2 (Polygon Zero)**
   - 仓库：https://github.com/0xPolygonZero/plonky2
   - 许可：Apache-2.0 / MIT
   - 用途：PLONK 实现对比参考
   - 引用文档：
     - `docs/research/zk-evaluation.md`

---

## 🌐 区块链项目与文档

1. **Solana**
   - 官网：https://docs.solana.com/
   - 用途：并行执行与账户锁定机制对比
   - 引用文档：
     - `docs/tech-comparison.md`
     - `docs/INDEX.md`

2. **Aptos**
   - 官网：https://aptos.dev/
   - 用途：Block-STM 乐观并发对比
   - 引用文档：
     - `docs/tech-comparison.md`
     - `docs/INDEX.md`

3. **Sui**
   - 官网：https://docs.sui.io/
   - 用途：对象所有权模型与快速路径对比
   - 引用文档：
     - `docs/tech-comparison.md`
     - `docs/INDEX.md`

4. **Monero**
   - 官网：https://www.getmonero.org/resources/
   - Moneropedia：https://www.getmonero.org/resources/moneropedia/
   - StackExchange：https://monero.stackexchange.com/
   - 用途：隐私技术研究参考
   - 引用文档：
     - `docs/tech-comparison.md`
     - `docs/research/monero-study-notes.md`
     - `docs/INDEX.md`

---

## 🛠️ 技术栈与工具

1. **Rust**
   - 文档：https://doc.rust-lang.org/
   - 许可：Apache-2.0 / MIT
   - 用途：SuperVM 核心开发语言

2. **Wasmtime**
   - 文档：https://docs.wasmtime.dev/
   - 许可：Apache-2.0
   - 用途：WASM 运行时

3. **libp2p**
   - 文档：https://docs.libp2p.io/
   - 许可：Apache-2.0 / MIT
   - 用途：P2P 网络协议（计划）

4. **Tendermint**
   - 文档：https://docs.tendermint.com/
   - 许可：Apache-2.0
   - 用途：共识协议参考（计划）

---

## 🙏 特别致谢

### 社区与贡献者

- **Zcash Foundation**：Powers of Tau 仪式设计与 Groth16 工业化
  - 仓库：https://github.com/ZcashFoundation/powersoftau-attestations
  - 引用：`docs/research/groth16-study.md`, `docs/research/groth16-poc-summary.md`

- **Electric Coin Company (ECC)**：Zcash Sapling 协议设计
  - 官网：https://z.cash/upgrade/sapling/

- **arkworks Contributors**：高质量 zkSNARK 生态维护

- **Monero Core Team & Research Lab**：隐私交易协议设计与持续迭代

- **dalek Cryptography**：高性能椭圆曲线密码学库

### 教程与博客

- **Zero Knowledge Blog**
  - 链接：https://www.zeroknowledgeblog.com/index.php/groth16
  - 用途：Groth16 入门教程
  - 引用：`docs/research/groth16-study.md`

- **Vitalik Buterin's Blog**
  - 链接：https://vitalik.ca/general/2019/09/22/plonk.html
  - 用途：PLONK 原理解析
  - 引用：`docs/research/zk-evaluation.md`

- **Ristretto Group 官网**
  - 链接：https://ristretto.group/
  - 用途：Ristretto 规范
  - 引用：`docs/research/curve25519-dalek-notes.md`

---

## 📖 引用格式说明

本项目研究笔记中对外部资料的引用遵循以下原则：

1. **论文引用**：标注标题、作者、ePrint/arXiv 链接
2. **项目引用**：标注仓库 URL、许可协议、引用目的
3. **文档引用**：标注官方文档链接、访问日期
4. **代码参考**：如有参考或移植外部代码片段，会在源码注释中明确标注来源与许可

---

## ⚖️ 许可声明

**SuperVM 原创内容**：

- 代码：GPL-3.0-or-later（详见 `LICENSE`）

- 文档：原创设计与实验报告遵循同一许可；研究笔记为二次创作，引述资料版权归原作者

**外部资料**：

- 上述列出的论文、项目、文档版权归各自作者/组织所有

- 本项目使用符合各自许可协议（Apache-2.0, MIT, BSD-3-Clause 等）

- 如有遗漏或错误，请联系维护者更正

---

**最后更新**：2025-11-06  
**维护者**：King Xujue (leadbrand@me.com)  
**问题反馈**：https://github.com/XujueKing/SuperVM/issues

---

感谢所有开源贡献者与学术研究者，让密码学与区块链技术得以快速发展！🎉
