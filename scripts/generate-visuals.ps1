# SuperVM Visual Assets Generator
# Usage: .\scripts\generate-visuals.ps1

$ErrorActionPreference = "Stop"

# Ensure working directory is repository root (so relative paths like 'assets' land correctly)
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
Set-Location $repoRoot

Write-Host "Starting SuperVM visual assets generation..." -ForegroundColor Cyan
Write-Host ""

# Create output directories
$visualsDir = Join-Path 'assets' 'visuals'
if (-not (Test-Path $visualsDir)) { New-Item -ItemType Directory -Force -Path $visualsDir | Out-Null }
foreach ($sub in 'diagrams','charts','infographics') {
    $p = Join-Path $visualsDir $sub
    if (-not (Test-Path $p)) { New-Item -ItemType Directory -Force -Path $p | Out-Null }
}

function Write-And-Verify($path, $content) {
    $dir = Split-Path -Parent $path
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    $content | Out-File -FilePath $path -Encoding UTF8
    if (Test-Path $path) {
        $size = (Get-Item $path).Length
        Write-Host "   Wrote: $path ($size bytes)" -ForegroundColor Green
    } else {
        Write-Host "   Write failed: $path" -ForegroundColor Red
    }
}

Write-Host "Output directories ready" -ForegroundColor Green
Write-Host ""

# 1) Mermaid diagrams
Write-Host "[1/5] Generating Mermaid architecture diagram..." -ForegroundColor Yellow

# Create 4-layer architecture Mermaid file
$architectureMmd = @'
graph TB
    subgraph L1["L1 超算节点 (大脑皮层)"]
        L1N1[全状态节点<br/>AWS/Azure]
        L1N2[ZK 证明生成<br/>Groth16/RingCT]
        L1N3[全局共识协调<br/>PoS/BFT]
    end
    
    subgraph L2["L2 矿机节点 (脊髓)"]
        L2N1[MVCC 并行执行<br/>242K TPS]
        L2N2[交易验证<br/>Work-Stealing]
        L2N3[状态同步<br/>Merkle Tree]
    end
    
    subgraph L3["L3 边缘节点 (神经节)"]
        L3N1[Mesh 中继<br/>WiFi/Bluetooth]
        L3N2[本地缓存<br/>RocksDB]
        L3N3[区域协调<br/>P2P Discovery]
    end
    
    subgraph L4["L4 移动终端 (感觉神经元)"]
        L4N1[SPV 轻客户端<br/>Merkle Proof]
        L4N2[环境感知<br/>Network/Battery]
        L4N3[用户交互<br/>Wallet/DApp]
    end
    
    L1 <-->|Internet/Starlink<br/>10-50ms| L2
    L2 <-->|WiFi Mesh<br/>5-10ms| L3
    L3 <-->|Bluetooth/LoRa<br/>2-200ms| L4
    
    style L1 fill:#e74c3c,stroke:#c0392b,stroke-width:3px,color:#fff
    style L2 fill:#3498db,stroke:#2980b9,stroke-width:3px,color:#fff
    style L3 fill:#2ecc71,stroke:#27ae60,stroke-width:3px,color:#fff
    style L4 fill:#f39c12,stroke:#e67e22,stroke-width:3px,color:#fff
'@

Write-And-Verify (Join-Path (Join-Path $visualsDir 'diagrams') 'architecture.mmd') $architectureMmd

# Try mermaid-cli if installed
if (Get-Command mmdc -ErrorAction SilentlyContinue) {
    try {
    mmdc -i (Join-Path (Join-Path $visualsDir 'diagrams') 'architecture.mmd') -o (Join-Path (Join-Path $visualsDir 'diagrams') 'architecture.png') -w 2000 -H 1400 -b transparent
    Write-Host "   架构图已生成 (PNG)" -ForegroundColor Green
    } catch {
    Write-Host "   PNG 生成失败，请手动运行: mmdc -i visuals/diagrams/architecture.mmd -o visuals/diagrams/architecture.png" -ForegroundColor Yellow
    }
} else {
    Write-Host "   mermaid-cli not found, skip PNG generation" -ForegroundColor Yellow
    Write-Host "   Hint: npm install -g @mermaid-js/mermaid-cli" -ForegroundColor Cyan
}

Write-Host "   Mermaid source created: architecture.mmd" -ForegroundColor Green
Write-Host ""

# 2) Gas mechanism flow
Write-Host "[2/5] Generating Gas mechanism flow..." -ForegroundColor Yellow

$gasMmd = @'
graph LR
    A[用户支付 Gas] -->|100%| B{Gas 分配}
    B -->|50%| C[燃烧销毁]
    B -->|30%| D[验证者奖励]
    B -->|20%| E[生态金库]
    
    C -->|减少供应| F[代币升值压力]
    D -->|质押收益| G[8-12% APY]
    E -->|开发资金| H[生态发展]
    
    F -.->|长期价值| I[持币者受益]
    G -.->|吸引质押| I
    H -.->|项目增长| I
    
    style C fill:#e74c3c,stroke:#c0392b,stroke-width:2px,color:#fff
    style F fill:#2ecc71,stroke:#27ae60,stroke-width:2px,color:#fff
    style G fill:#3498db,stroke:#2980b9,stroke-width:2px,color:#fff
    style H fill:#f39c12,stroke:#e67e22,stroke-width:2px,color:#fff
    style I fill:#9b59b6,stroke:#8e44ad,stroke-width:3px,color:#fff
'@

Write-And-Verify (Join-Path (Join-Path $visualsDir 'diagrams') 'gas-mechanism.mmd') $gasMmd
Write-Host "   Gas flow Mermaid source created" -ForegroundColor Green
Write-Host ""

# 3) Python performance chart
Write-Host "[3/5] Generating Python performance chart..." -ForegroundColor Yellow

$performancePy = @'
import matplotlib.pyplot as plt
import matplotlib
matplotlib.rcParams['font.sans-serif'] = ['SimHei', 'Microsoft YaHei']
matplotlib.rcParams['axes.unicode_minus'] = False

# 数据
chains = ['Bitcoin', 'Ethereum', 'Cardano', 'Solana', 'Visa', 'SuperVM']
tps = [7, 15, 250, 50000, 65000, 242000]
colors = ['#F7931A', '#627EEA', '#0033AD', '#14F195', '#1A1F71', '#E74C3C']

# 创建图表
fig, ax = plt.subplots(figsize=(12, 7))
bars = ax.bar(chains, tps, color=colors, edgecolor='black', linewidth=1.5)

# 添加数值标签
for i, bar in enumerate(bars):
    height = bar.get_height()
    label = f'{int(height):,} TPS' if height < 1000 else f'{int(height/1000)}K TPS'
    ax.text(bar.get_x() + bar.get_width()/2., height,
            label,
            ha='center', va='bottom', fontsize=11, fontweight='bold')

# 高亮 SuperVM
bars[-1].set_edgecolor('#8B0000')
bars[-1].set_linewidth(4)

# 样式
ax.set_ylabel('TPS (每秒交易数)', fontsize=13, fontweight='bold')
ax.set_title('区块链性能对比\\nSuperVM: 242,000 TPS 实测性能', 
             fontsize=16, fontweight='bold', pad=20)
ax.set_yscale('log')
ax.grid(axis='y', alpha=0.3, linestyle='--')
ax.set_ylim(1, 500000)

# 添加注释
ax.annotate('比 Ethereum 快\\n16,133 倍', 
            xy=(5, 242000), xytext=(4.5, 100000),
            arrowprops=dict(arrowstyle='->', color='red', lw=2),
            fontsize=11, color='red', fontweight='bold',
            bbox=dict(boxstyle='round,pad=0.5', facecolor='yellow', alpha=0.7))

plt.tight_layout()
plt.savefig('assets/visuals/charts/performance-comparison.png', dpi=300, bbox_inches='tight')
print('Done: performance chart saved: assets/visuals/charts/performance-comparison.png')
'@

Write-And-Verify (Join-Path (Join-Path $visualsDir 'charts') 'generate-performance.py') $performancePy

if (Get-Command python -ErrorAction SilentlyContinue) {
    try {
    python (Join-Path (Join-Path $visualsDir 'charts') 'generate-performance.py')
    Write-Host "   Performance chart generated (PNG)" -ForegroundColor Green
    } catch {
        Write-Host "   Python script execution failed" -ForegroundColor Yellow
    }
} else {
    Write-Host "   Python not found, skip chart generation" -ForegroundColor Yellow
    Write-Host "   Hint: pip install matplotlib" -ForegroundColor Cyan
}

Write-Host ""

# 4) Gas fee comparison chart
Write-Host "[4/5] Generating Gas fee comparison chart..." -ForegroundColor Yellow

$gasPy = @'
import matplotlib.pyplot as plt
import matplotlib
matplotlib.rcParams['font.sans-serif'] = ['SimHei', 'Microsoft YaHei']
matplotlib.rcParams['axes.unicode_minus'] = False

# 数据
chains = ['Ethereum', 'BSC', 'Polygon', 'Arbitrum', 'Optimism', 'SuperVM']
gas_usd = [15.30, 0.50, 0.05, 0.80, 0.60, 0.01]
colors = ['#627EEA', '#F3BA2F', '#8247E5', '#28A0F0', '#FF0420', '#E74C3C']

# 创建图表
fig, ax = plt.subplots(figsize=(12, 7))
bars = ax.bar(chains, gas_usd, color=colors, edgecolor='black', linewidth=1.5)

# 添加数值标签
for i, bar in enumerate(bars):
    height = bar.get_height()
    ax.text(bar.get_x() + bar.get_width()/2., height,
            f'\${height:.2f}',
            ha='center', va='bottom', fontsize=12, fontweight='bold')

# 高亮 SuperVM
bars[-1].set_edgecolor('#8B0000')
bars[-1].set_linewidth(4)

# 样式
ax.set_ylabel('Gas 费用 (USD)', fontsize=13, fontweight='bold')
ax.set_title('跨链 Gas 费用对比\\nSuperVM: 比 Ethereum 便宜 99.3%', 
             fontsize=16, fontweight='bold', pad=20)
ax.set_ylim(0, max(gas_usd) * 1.3)
ax.grid(axis='y', alpha=0.3, linestyle='--')

# 添加节省百分比
savings = ((gas_usd[0] - gas_usd[-1]) / gas_usd[0]) * 100
ax.text(5, gas_usd[0] * 0.8, f'节省 {savings:.1f}%', 
        fontsize=14, color='red', fontweight='bold',
        bbox=dict(boxstyle='round,pad=0.8', facecolor='yellow', alpha=0.8))

plt.tight_layout()
plt.savefig('assets/visuals/charts/gas-comparison.png', dpi=300, bbox_inches='tight')
print('Done: gas fee chart saved: assets/visuals/charts/gas-comparison.png')
'@

Write-And-Verify (Join-Path (Join-Path $visualsDir 'charts') 'generate-gas.py') $gasPy

if (Get-Command python -ErrorAction SilentlyContinue) {
    try {
    python (Join-Path (Join-Path $visualsDir 'charts') 'generate-gas.py')
    Write-Host "   Gas fee chart generated (PNG)" -ForegroundColor Green
    } catch {
        Write-Host "   Python script execution failed" -ForegroundColor Yellow
    }
}

Write-Host ""

# 5) Token allocation pie chart
Write-Host "[5/5] Generating token allocation pie chart..." -ForegroundColor Yellow

$tokenomicsPy = @'
import matplotlib.pyplot as plt
import matplotlib
matplotlib.rcParams['font.sans-serif'] = ['SimHei', 'Microsoft YaHei']
matplotlib.rcParams['axes.unicode_minus'] = False

# 代币分配
labels = ['生态挖矿\\n40%', '团队\\n20%\\n(4年解锁)', '投资者\\n15%\\n(2年解锁)', '基金会\\n15%', '公开发售\\n10%']
sizes = [40, 20, 15, 15, 10]
colors = ['#3498db', '#e74c3c', '#f39c12', '#2ecc71', '#9b59b6']
explode = (0.1, 0, 0, 0, 0)

fig, ax = plt.subplots(figsize=(10, 8))
wedges, texts, autotexts = ax.pie(sizes, explode=explode, labels=labels, colors=colors,
                                    autopct='%1.1f%%', startangle=90, 
                                    textprops={'fontsize': 12, 'weight': 'bold'},
                                    pctdistance=0.85)

# 设置百分比样式
for autotext in autotexts:
    autotext.set_color('white')
    autotext.set_fontweight('bold')
    autotext.set_fontsize(14)

ax.set_title('\$SUPERVM 代币分配\\n总供应量: 1,000,000,000', 
             fontsize=16, fontweight='bold', pad=25)

# 添加图例
ax.legend(wedges, labels, title='分配类别', loc='center left', 
          bbox_to_anchor=(1, 0, 0.5, 1), fontsize=11)

plt.tight_layout()
plt.savefig('assets/visuals/charts/tokenomics.png', dpi=300, bbox_inches='tight')
print('Done: tokenomics chart saved: assets/visuals/charts/tokenomics.png')
'@

Write-And-Verify (Join-Path (Join-Path $visualsDir 'charts') 'generate-tokenomics.py') $tokenomicsPy

if (Get-Command python -ErrorAction SilentlyContinue) {
    try {
    python (Join-Path (Join-Path $visualsDir 'charts') 'generate-tokenomics.py')
    Write-Host "   Tokenomics chart generated (PNG)" -ForegroundColor Green
    } catch {
        Write-Host "   Python script execution failed" -ForegroundColor Yellow
    }
}

Write-Host ""

# Summary
Write-Host "------------------------------" -ForegroundColor Cyan
Write-Host "Visual assets generation completed" -ForegroundColor Green
Write-Host "------------------------------" -ForegroundColor Cyan
Write-Host ""
Write-Host "Generated files:" -ForegroundColor Cyan
Write-Host ""

Write-Host "  Mermaid sources:" -ForegroundColor Yellow
Get-ChildItem (Join-Path $visualsDir 'diagrams') -Filter *.mmd | ForEach-Object {
    Write-Host "    - $($_.Name)" -ForegroundColor White
}

Write-Host ""
Write-Host "  Charts:" -ForegroundColor Yellow
if (Test-Path (Join-Path $visualsDir 'charts')) {
    Get-ChildItem (Join-Path $visualsDir 'charts') -Filter *.png | ForEach-Object {
    Write-Host "    - $($_.Name)" -ForegroundColor White
    }
}

Write-Host ""
Write-Host "Location: $(Resolve-Path $visualsDir)" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "   1. To render Mermaid PNGs: npm install -g @mermaid-js/mermaid-cli" -ForegroundColor Cyan
Write-Host "   2. To render charts: pip install matplotlib" -ForegroundColor Cyan
Write-Host ""
