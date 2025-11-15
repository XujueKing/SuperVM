# SuperVM 资源目录

> **一键生成所有营销资产和文档的完整指南**

---

## 📋 目录结构（已统一到 assets/）

```
SuperVM/
├── assets/
│   ├── pdf/                 # PDF 输出目录
│   │   ├── SuperVM_Whitepaper_CN_v1.0.pdf
│   │   ├── SuperVM_Whitepaper_EN_v1.0.pdf
│   │   ├── SuperVM_Investor_Deck_v1.0.pdf
│   │   └── SuperVM_Social_Media_Templates_v1.0.pdf
│   │
│   ├── ppt/                 # PowerPoint 输出目录 (双语)
│   │   ├── pitch-deck-source.md           # 中文源 (已清理乱码)
│   │   ├── pitch-deck-source-en.md        # 英文源
│   │   ├── SuperVM_Pitch_CN.pptx          # 中文 PPTX (自动生成)
│   │   ├── SuperVM_Pitch_EN.pptx          # 英文 PPTX (自动生成)
│   │   └── SuperVM_Investor_Pitch_Deck.pptx (旧单语版本，可逐步淘汰)
│   │
│   └── visuals/             # 视觉资产目录
│       ├── diagrams/        # 架构图和流程图（*.mmd / *.png）
│       ├── charts/          # 数据图表（*.py / *.png）
│       └── infographics/    # 信息图 (预留)
│
└── scripts/                 # 自动化生成脚本
    ├── generate-all.ps1     # 一键生成所有资产
    ├── generate-pdfs.ps1    # 生成 PDF 文档
    ├── generate-visuals.ps1 # 生成视觉资产
    └── generate-ppt.ps1     # 生成 PowerPoint
```

---

## 🚀 快速开始

### 方式 1: 一键生成所有资产 (推荐)

```powershell
# 在项目根目录运行
.\scripts\generate-all.ps1
```

这将自动执行:
1. ✅ 生成所有视觉资产 (架构图、图表、信息图)
2. ✅ 生成所有 PDF 文档 (中英文白皮书、投资者 Deck)
3. ✅ 生成 PowerPoint 演示文稿

### 方式 2: 分步生成

```powershell
# 仅生成 PDF
.\scripts\generate-pdfs.ps1

# 仅生成视觉资产
.\scripts\generate-visuals.ps1

# 仅生成 PPT
.\scripts\generate-ppt.ps1
```

---

## 📦 依赖工具

### 必需工具

| 工具 | 用途 | 安装命令 | 状态 |
|------|------|----------|------|
| **Pandoc** | PDF/PPT 生成 | `choco install pandoc` | ⚠️ 必需 |

### 可选工具 (增强输出)

| 工具 | 用途 | 安装命令 | 状态 |
|------|------|----------|------|
| **Python** | 图表生成 | [下载](https://www.python.org) | 推荐 |
| **Matplotlib** | 图表库 | `pip install matplotlib` | 推荐 |
| **Mermaid CLI** | 架构图 PNG | `npm install -g @mermaid-js/mermaid-cli` | 可选 |

### 安装检查

```powershell
# 检查所有工具
pandoc --version
python --version
pip list | findstr matplotlib
mmdc --version
```

---

## 📄 生成的文档清单

### PDF 文档 (4 个)

1. **SuperVM_Whitepaper_CN_v1.0.pdf** (~30 页)
   - 中文技术白皮书
   - 包含目录、章节编号
   - 适用于: 中文社区、国内投资者

2. **SuperVM_Whitepaper_EN_v1.0.pdf** (~25 页)
   - 英文技术白皮书
   - Executive Summary
   - 适用于: 国际投资者、海外社区

3. **SuperVM_Investor_Deck_v1.0.pdf** (~20 页)
   - 投资者演示文稿 (PDF 版)
   - 包含所有 18 页幻灯片
   - 适用于: 邮件发送、打印

4. **SuperVM_Social_Media_Templates_v1.0.pdf** (~15 页)
   - 社交媒体发布模板汇总
   - Twitter/Medium/Reddit 等
   - 适用于: 营销团队参考

### PowerPoint 演示 (双语 2 + 旧版本 1)

5. **SuperVM_Pitch_CN.pptx**
   - 中文投资者演示
   - Markdown → Pandoc 自动转换
   - 后期：排版优化 / 品牌化

6. **SuperVM_Pitch_EN.pptx**
   - 英文投资者演示
   - 结构与中文版本对齐
   - 建议保持页码同步

7. **SuperVM_Investor_Pitch_Deck.pptx** (Legacy)
   - 旧的单语版本（可在确认双语稿稳定后删除）

---

## 🎨 视觉资产清单

### 架构图 (Mermaid)

1. **architecture.mmd** / **architecture.png**
   - 四层神经网络架构
   - L1-L4 层级关系
   - 通信协议标注

2. **gas-mechanism.mmd** / **gas-mechanism.svg**
   - Gas 燃烧分配流程
   - 50% 燃烧 + 30% 验证者 + 20% 金库

### 数据图表 (Python)

3. **performance-comparison.png**
   - TPS 性能对比柱状图
   - Bitcoin/Ethereum/Solana/Visa/SuperVM
   - 对数刻度显示

4. **gas-comparison.png**
   - Gas 费用对比柱状图
   - 跨链 Gas 费用比较
   - 高亮 99.3% 节省

5. **tokenomics.png**
   - 代币分配饼图
   - 5 大分配类别
   - 百分比标注

---

## 🔧 自定义配置

### PDF 样式自定义

编辑 `scripts/generate-pdfs.ps1`:

```powershell
# 修改字体
-V CJKmainfont="SimSun"      # 中文主字体
-V CJKsansfont="SimHei"      # 中文无衬线字体

# 修改页边距
-V geometry:margin=1in       # 上下左右边距

# 修改字号
-V fontsize=11pt             # 正文字号

# 修改颜色
-V linkcolor=blue            # 链接颜色
```

### 图表样式自定义

编辑 `visuals/charts/generate-*.py`:

```python
# 修改颜色
colors = ['#E74C3C', '#3498DB', ...]

# 修改尺寸
fig, ax = plt.subplots(figsize=(12, 7))

# 修改 DPI
plt.savefig(..., dpi=300)
```

---

## 📊 PowerPoint 后期编辑清单

生成的 PPT 需要在 PowerPoint 中进一步美化:

### 1. 应用品牌配色

**主色**: 潘多拉红 `#E74C3C`
**辅色**:
- L1 红: `#E74C3C`
- L2 蓝: `#3498DB`
- L3 绿: `#2ECC71`
- L4 橙: `#F39C12`

### 2. 添加 Logo

在每页右上角添加 SuperVM Logo

### 3. 插入图表

从 `assets/visuals/charts/` 插入:
- Slide 4: architecture.png (四层架构)
- Slide 7: performance-comparison.png (TPS 对比)
- Slide 9: gas-comparison.png (Gas 对比)
- Slide 10: tokenomics.png (代币分配)

### 4. 调整字体

- 标题: **Montserrat Bold** (英文) / **微软雅黑 Bold** (中文)
- 正文: **Open Sans** (英文) / **微软雅黑** (中文)
- 代码: **Fira Code**

### 5. 添加动画 (可选)

- 标题: 淡入
- 列表: 逐条出现
- 图表: 擦除

---

## 🌐 发布清单

### 网站上传

- [ ] 上传 `SuperVM_Whitepaper_CN_v1.0.pdf` 到 `supervm.io/whitepaper-cn`
- [ ] 上传 `SuperVM_Whitepaper_EN_v1.0.pdf` 到 `supervm.io/whitepaper`
- [ ] 上传 `SuperVM_Investor_Pitch_Deck.pptx` 到内部投资者页面
- [ ] 生成 SHA256 校验和并公布

```powershell
Get-FileHash assets\pdf\SuperVM_Whitepaper_CN_v1.0.pdf -Algorithm SHA256
```

### GitHub Release

- [ ] 创建 Release `v1.0-whitepaper`
- [ ] 附件: 所有 PDF 文件
- [ ] Release Notes: 链接到 CHANGELOG.md

### 社交媒体

- [ ] Twitter/X: 发布 Thread (使用 `docs/SOCIAL-MEDIA-TEMPLATES.md`)
- [ ] Medium: 发布长文
- [ ] Reddit: r/CryptoCurrency + r/ethereum 发帖
- [ ] Discord: @everyone 公告

### 投资者沟通

- [ ] 发送 Pitch Deck 给目标 VC
- [ ] 附带: 白皮书 PDF + GitHub 链接
- [ ] 安排技术深潜会议

---

## 🐛 故障排除

### 问题 1: "未找到 Pandoc"

**解决方案**:
```powershell
# 安装 Pandoc
choco install pandoc

# 或手动下载
# https://pandoc.org/installing.html
```

### 问题 2: "中文显示为方框"

**解决方案**:
- Windows: 系统已内置宋体/黑体
- 检查字体: 控制面板 → 字体

### 问题 3: "Python 脚本执行失败"

**解决方案**:
```powershell
# 安装 matplotlib
pip install matplotlib

# 验证安装
python -c "import matplotlib; print(matplotlib.__version__)"
```

### 问题 4: "PowerPoint 打开报错"

**解决方案**:
- 确保使用 PowerPoint 2016 或更高版本
- 或使用 Google Slides / LibreOffice Impress 打开

---

## 📈 质量检查

### PDF 检查

- [ ] 目录页码正确
- [ ] 所有链接可点击
- [ ] 中文字体正常显示
- [ ] 代码块格式正确
- [ ] 文件大小 < 10MB

### PPT 检查

- [ ] 所有幻灯片正常显示
- [ ] 表格对齐
- [ ] 图表清晰
- [ ] 没有乱码
- [ ] 动画流畅

### 图表检查

- [ ] 分辨率 >= 300 DPI
- [ ] 颜色对比度足够
- [ ] 标签清晰可读
- [ ] 数据准确
- [ ] 品牌色一致

---

## 🔄 更新流程

### 更新白皮书内容

1. 编辑 `WHITEPAPER.md` 或 `WHITEPAPER_EN.md`
2. 运行 `.\scripts\generate-pdfs.ps1`
3. 版本号递增: v1.0 → v1.1
4. 更新 CHANGELOG.md

### 更新 Pitch Deck（双语流程）

1. 中文：编辑 `assets/ppt/pitch-deck-source.md`
2. 英文：编辑 `assets/ppt/pitch-deck-source-en.md`
3. 运行 `./scripts/generate-ppt.ps1` 自动生成/覆盖 `SuperVM_Pitch_CN.pptx` 和 `SuperVM_Pitch_EN.pptx`
4. 在 PPT 中进行视觉设计与品牌统一
5. 若发布新版本，使用语义化命名：`SuperVM_Pitch_CN_v1.1.pptx` / `SuperVM_Pitch_EN_v1.1.pptx`
6. 可在根目录添加 `PITCH-DECK-CHANGELOG.md` 记录迭代

### 更新图表数据

1. 编辑 `visuals/charts/generate-*.py` 中的数据
2. 运行 `.\scripts\generate-visuals.ps1`
3. 替换 PPT 中的旧图表

---

## 📞 支持

**文档问题**: 参考 `docs/PDF-GENERATION-GUIDE.md`

**视觉资产**: 参考 `docs/VISUAL-ASSETS-GUIDE.md`

**社交媒体**: 参考 `docs/SOCIAL-MEDIA-TEMPLATES.md`

**技术问题**: 提交 GitHub Issue

---

## 🎯 下一步行动

**立即执行**:
```powershell
# 一键生成所有资产
.\scripts\generate-all.ps1
```

**预计耗时**: 2-5 分钟 (取决于系统性能)

**预期输出**:
- ✅ 4 个 PDF 文档
- ✅ 2 个 PowerPoint 演示（中 / 英）
- ✅ 5+ 个可视化图表
- ✅ 2 个 Mermaid 源文件

---

**准备好打开 Web3 的潘多拉魔盒了吗?** 🚀

*"We're not building another blockchain. We're building the OS for ALL blockchains."*
