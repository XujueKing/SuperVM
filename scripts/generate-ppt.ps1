# SuperVM Pitch Deck PPT Generator
# Usage: .\scripts\generate-ppt.ps1

$ErrorActionPreference = "Stop"

# Ensure working directory is repository root so outputs go under assets/
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
Set-Location $repoRoot

Write-Host "Starting SuperVM Investor Pitch Deck (PPT) generation..." -ForegroundColor Cyan
Write-Host ""

# 统一 Pandoc 解析逻辑，允许刚安装后 PATH 未刷新的场景
function Resolve-PandocPath {
   $cmd = Get-Command pandoc -ErrorAction SilentlyContinue
   if ($cmd -and $cmd.Source) { return $cmd.Source }
   $candidates = @(
      'C:\\Program Files\\Pandoc\\pandoc.exe',
      'C:\\Program Files (x86)\\Pandoc\\pandoc.exe',
      "$env:USERPROFILE\\scoop\\apps\\pandoc\\current\\pandoc.exe",
      "$env:LOCALAPPDATA\\Pandoc\\pandoc.exe"
   )
   foreach ($p in $candidates) { if (Test-Path $p) { return $p } }
   return $null
}

$pandocPath = Resolve-PandocPath
if (-not $pandocPath) {
   Write-Host "Error: Pandoc not found (required for PPT generation)" -ForegroundColor Red
   Write-Host "Fix: restart terminal or add Pandoc dir to PATH" -ForegroundColor Yellow
   Write-Host "Example: choco install pandoc / see https://pandoc.org" -ForegroundColor Yellow
   exit 1
}

$pandocDir = Split-Path -Parent $pandocPath
if ($env:Path -notlike "*$pandocDir*") { $env:Path += ";$pandocDir" }

Write-Host "Pandoc detected: $pandocPath" -ForegroundColor Green
Write-Host ""

# 创建输出目录
$outputDir = (Join-Path 'assets' 'ppt')
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

# 从 Markdown 创建优化版 PPT 源文件
Write-Host "Preparing PowerPoint source markdown..." -ForegroundColor Yellow

# 读取原始 Pitch Deck 并优化为 PPT 格式
$pptMarkdown = @'
---
title: "SuperVM"
subtitle: "The Web3 Operating System"
author: "SuperVM Team"
date: "January 2025"
---

# SuperVM {.center}

## 潘多拉星核 (Pandora Core)

**Unlocking the Multi-Chain Future**

---

# The \$2 Billion Problem

## Web3's Fragmentation Crisis

:::: {.columns}
::: {.column width="33%"}
**Security Catastrophe**

- \$2B+ stolen from bridges
- Ronin: \$624M
- Wormhole: \$326M
- Single point of failure
:::

::: {.column width="33%"}
**Performance Ceiling**

- Ethereum: 15 TPS
- 30 min finality
- Cross-chain: 1-60 min
- Wrapped assets
:::

::: {.column width="34%"}
**Developer Hell**

- 200+ blockchains
- 50+ languages
- Rebuild for each chain
- No unified environment
:::
::::

---

# The Vision

## One OS for All Blockchains

> What if Bitcoin, Ethereum, Solana worked together like apps on your phone?

- ✅ No bridges, no wrapping
- ✅ Write once, deploy everywhere
- ✅ Privacy by default
- ✅ Unstoppable network

**We're building the Web3 Operating System.**

---

# 4 Breakthrough Innovations

| Innovation | Breakthrough | Competitor |
|------------|--------------|------------|
| **Native Chain Fusion** | Bitcoin/Ethereum nodes AS components | Bridge contracts (hackable) |
| **242K TPS Engine** | MVCC parallel execution | 10-50K TPS |
| **Built-in Privacy** | Ring + ZK proofs | Privacy add-on |
| **Neural Mesh** | Auto-switch protocols | Internet-only |

---

# Four-Layer Neural Network

\`\`\`
┌─────────────────────────────────────┐
│ L1: 超算 (Cerebral Cortex)           │
│ Full state + ZK proofs              │
├─────────────────────────────────────┤
│ L2: 矿机 (Spinal Cord)               │
│ MVCC execution (242K TPS)           │
├─────────────────────────────────────┤
│ L3: 边缘 (Ganglia)                   │
│ Mesh relay + caching                │
├─────────────────────────────────────┤
│ L4: 移动 (Sensory Neurons)           │
│ SPV clients + sensing               │
└─────────────────────────────────────┘
\`\`\`

**Auto-switch**: Internet → WiFi → Bluetooth → LoRa

---

# No Bridges = No Hacks

## Traditional Bridges vs SuperVM

| Aspect | Bridges ❌ | SuperVM ✅ |
|--------|-----------|----------|
| Trust | Relayer committee | Cryptographic proofs |
| Assets | Wrapped (wBTC) | Native (BTC) |
| Security | \$2B+ hacked | Zero bridge contracts |
| Latency | 1-60 minutes | Real-time |
| Liquidity | Fragmented | Unified |

---

# Market Opportunity

## `$85B` TAM, Growing 40% YoY

- **Total Addressable Market**: `$85B` (2024)
  - Multi-chain infrastructure
  - Cross-chain DEX volume: \$120B annual
  
- **Serviceable Market**: \$35B
  - Multi-chain DeFi
  - Enterprise blockchain

- **Our Target**: \$500M (Year 1-3)
  - 5% market share → \$1.75B opportunity

---

# Competitive Landscape

| Category | Projects | Approach | Limitation |
|----------|----------|----------|------------|
| **Bridges** | Wormhole, LayerZero | Lock-mint | \$2B hacked |
| **L2 Rollups** | Arbitrum, zkSync | Ethereum scaling | Single-chain |
| **Alt-L1s** | Solana, Avalanche | High performance | Isolated |
| **Privacy** | Monero, Zcash | Privacy focus | Low TPS |
| **SuperVM** | **Us** | **Multi-chain OS** | **Early stage** |

**Our Moat**: 3-year tech lead + first-mover + network effects

---

# Business Model

## Triple Revenue Streams

1. **Transaction Fees** (Primary)
   - 0.07% vs Ethereum's 0.5-3%
   - 50% burned, 30% validators, 20% treasury
   - **`$5M` ARR** (Year 1) → **`$50M`** (Year 3)

2. **Enterprise Licensing** (B2B)
   - Private deployments: \$100K-\$500K/year
   - **`$2M` ARR** (Year 2) → **`$10M`** (Year 4)

3. **Developer Platform** (Future)
   - API access, smart contract fees
   - **`$1M` ARR** (Year 3)

**Unit Economics**: LTV/CAC = 48:1

---

# Tokenomics

## `$SUPERVM`: Deflationary Utility Token

**Total Supply**: 1,000,000,000

- 🌐 Ecosystem (40%): Mining rewards
- 👥 Team (20%): 4-year vesting
- 💰 Investors (15%): 2-year vesting
- 🏛️ Foundation (15%): Development
- 🚀 Public (10%): TGE

**Utility**: Gas fees, staking (8-12% APY), governance

**Burn**: 50% of all Gas fees → 500M tokens burned in 5 years

---

# Roadmap

## 18 Months to Mainnet

**2024** ✅ Completed
- MVCC engine (242K TPS)
- ZK verifier (RingCT + Groth16)

**2025** In Progress
- Q1: Bitcoin + Ethereum adapters
- Q2: Native monitoring, 4-layer PoC
- Q4: Public testnet

**2026** Mainnet
- Q2: Token launch (TGE)
- Q4: 100K daily users

---

# Team

## World-Class Experts

**Founder** - CEO & Lead Architect
- 15 years distributed systems
- Former [Company]: [Achievement]
- Ph.D. Computer Science

**CTO** - Core Developer
- 20K+ lines production Rust
- Blockchain since 2016

**Team Stats**
- 8 full-time engineers
- 3 security auditors
- Average 10+ years experience

---

# Traction

## Real Code, Real Performance

**Technical Milestones**:
- ✅ 242,000 TPS (tested)
- ✅ 50.8 RingCT proofs/sec
- ✅ 3,500+ GitHub commits
- ✅ 42K lines Rust code

**Community**:
- 2,400 GitHub stars (+15%/month)
- 800 Discord members
- 120 contributors

---

# Revolutionary Scenarios

## 1. Disaster Response 🌍

**Earthquake cuts Internet**
→ L3 nodes form WiFi Mesh (30 seconds)
→ Local payments continue via Bluetooth
→ Auto-sync when restored

## 2. Censorship Resistance 🔒

**Government blocks Internet**
→ LoRa radio relay (2-20km range)
→ Bypass firewalls
→ Starlink backup

## 3. Financial Inclusion 💰

**Rural Africa, no Internet**
→ Solar-powered nodes
→ Bluetooth to phones
→ Cross-border remittance <\$1 fee

---

# The Ask

## \$5M Seed Round

**Use of Funds**:
- Engineering (48%): \$2.4M
- Security audits (15%): \$750K
- Marketing (10%): \$500K
- Infrastructure (6%): \$300K
- Legal (8%): \$400K
- Operations (13%): \$650K

**Valuation**: \$20M pre-money, \$25M post
**Equity**: 20% (SAFE, 20% discount)

---

# Projected Financials

| Year | Users | Tx Volume | Revenue | Valuation |
|------|-------|-----------|---------|-----------|
| 2025 | 5K | \$10M | \$70K | \$25M |
| 2026 | 50K | \$500M | \$3.5M | \$150M |
| 2027 | 500K | \$5B | \$35M | \$800M |
| 2028 | 2M | \$20B | \$140M | **\$2.5B** |

**Key Milestones**:
1. Testnet 10K users (Month 12)
2. 2 enterprise partnerships (Month 15)
3. Security audit passed (Month 16)

---

# Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Regulatory uncertainty | High | Legal counsel in 3 jurisdictions |
| Technical complexity | Medium | 3-year R&D complete, audits |
| Competition | Medium | Open-source moat, first-mover |
| Adoption slower | High | Ecosystem fund (\$400M) |

**De-Risking**: Testnet before mainnet, gradual rollout, insurance fund

---

# Why Now?

## Perfect Storm of Opportunity

1. **Institutional Wave** (2024-2025)
   - BlackRock's \$10T tokenization
   - Bitcoin ETF approval

2. **Bridge Crisis Awakening**
   - \$2B stolen → demand for security
   - No innovation since 2020

3. **Technical Maturity**
   - ZK-SNARKs production-ready
   - MVCC proven in databases

**Our Timing**: Mainnet Q2 2026 aligns with bull market peak

---

# Join Us

## Unlock Web3's Pandora Box

**What We're Building**:
- Not another blockchain
- Not another bridge
- **The OS for ALL blockchains**

**What We're Asking**:
- \$5M seed funding
- Strategic partners
- Regulatory/legal advisors

**What You Get**:
- 20% equity in \$2.5B+ outcome
- First-mover position
- Impact on 1B+ users

---

# Next Steps {.center}

1. **Deep-dive** technical presentation (2 hours)
2. **GitHub** access + codebase review
3. **Term sheet** within 2 weeks

---

**Contact**:
- 📧 invest@supervm.io
- 🌐 supervm.io/deck
- 💻 github.com/idkbreh/SuperVM

**Thank you. Questions?**

'@

Set-Content -Path (Join-Path $outputDir 'pitch-deck-source.md') -Value $pptMarkdown -Encoding UTF8
Write-Host "   PPT source created" -ForegroundColor Green
Write-Host ""

# Generate bilingual PowerPoint files
Write-Host "Generating bilingual PowerPoints (pptx)..." -ForegroundColor Yellow

$sources = @(
      @{ File = (Join-Path $outputDir 'pitch-deck-source.md'); Out = (Join-Path $outputDir 'SuperVM_Pitch_CN.pptx'); Title = 'SuperVM 投资路演'; Lang='zh-CN' },
      @{ File = (Join-Path $outputDir 'pitch-deck-source-en.md'); Out = (Join-Path $outputDir 'SuperVM_Pitch_EN.pptx'); Title = 'SuperVM Investor Pitch Deck'; Lang='en' }
)

foreach ($s in $sources) {
   if (-not (Test-Path $s.File)) { Write-Host "   Skip missing source: $($s.File)" -ForegroundColor Yellow; continue }
   Write-Host "   -> Building $($s.Out)" -ForegroundColor Cyan
   try {
      $pandocArgs = @(
         $s.File,
         '-o', $s.Out,
         '--to','pptx',
         '--slide-level=1',
         '--metadata',"title=$($s.Title)",
         '--metadata','author=SuperVM Team',
         '--metadata','date=January 2025'
      )
      & $pandocPath @pandocArgs
      if (Test-Path $s.Out) {
          $pptFile = Get-Item $s.Out
          $size = [math]::Round($pptFile.Length / 1MB, 2)
          Write-Host "      Success: $($pptFile.Name) ($size MB)" -ForegroundColor Green
      } else {
          Write-Host "      Failed: output file not found" -ForegroundColor Red
      }
   } catch {
          Write-Host "      Generation error: $_" -ForegroundColor Red
   }
}

Write-Host ""; Write-Host "PPT build finished." -ForegroundColor Green
