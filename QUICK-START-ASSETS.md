# SuperVM 资产生成快速开始指南

> 5 分钟内完成：PDF、双语 Pitch Deck、可视化图表。

当前 Pitch Deck 已迁移为双语源：`assets/ppt/pitch-deck-source.md`（中文）与 `assets/ppt/pitch-deck-source-en.md`（英文）。

快速生成 PPT（示例命令）：
```powershell
pandoc assets/ppt/pitch-deck-source.md -o assets/ppt/Quick_CN.pptx --to pptx --slide-level=1
pandoc assets/ppt/pitch-deck-source-en.md -o assets/ppt/Quick_EN.pptx --to pptx --slide-level=1
```

或使用脚本：
```powershell
./scripts/generate-ppt.ps1
```

如果系统尚未安装 Pandoc，请先完成安装步骤。

---

## 📦 步骤 1: 安装必需工具

### 方式 A: 使用 Chocolatey (推荐)

```powershell
# 安装 Pandoc
choco install pandoc -y

# 安装 MiKTeX (PDF 可选增强)
choco install miktex -y

# 安装 Python（若未安装）
choco install python -y

# 安装 Mermaid CLI（可选）
npm install -g @mermaid-js/mermaid-cli
```

### 方式 B: 手动下载安装

1. **下载 Pandoc**:
   - 访问: https://github.com/jgm/pandoc/releases/latest
   - 下载: `pandoc-3.x-windows-x86_64.msi`
   - 安装并重启终端

2. **下载 MiKTeX** (LaTeX):
   - 访问: https://miktex.org/download
   - 下载: `basic-miktex-x64.exe`
   - 安装时选择 "自动安装缺失的包"

3. **验证安装**:
```powershell
pandoc --version
# 应显示: pandoc 3.x.x
```

---

## 🎨 步骤 2: 生成资产

### 2.1 生成 PDF 文档

```powershell
./scripts/generate-pdfs.ps1   # 输出到 assets/pdf
```

输出示例：
- SuperVM_Whitepaper_CN_v1.0.pdf
- SuperVM_Whitepaper_EN_v1.0.pdf

### 2.2 生成 PowerPoint 演示（双语）

```powershell
./scripts/generate-ppt.ps1   # 生成 SuperVM_Pitch_CN.pptx / SuperVM_Pitch_EN.pptx
```

### 2.3 生成视觉图表 (可选)

# 下载 Python: https://www.python.org/downloads/
pip install matplotlib

# 生成图表
python assets/visuals/charts/generate-performance.py
python assets/visuals/charts/generate-gas.py
python assets/visuals/charts/generate-tokenomics.py
---

## 🔧 步骤 3: 手动生成 (无 Pandoc 时的替代方案)

1. **Markdown to PDF**:
   - 访问: https://www.markdowntopdf.com/
   - Export → PDF
### 方式 2: 使用 Google Docs

1. 在 VS Code 中复制 `WHITEPAPER.md` 内容
2. 在 Google Docs 新建文档
3. 粘贴内容 (Markdown 会被自动格式化)
4. 调整格式 (标题、列表、表格)
5. File → Download → PDF

### 方式 3: 生成 PowerPoint (在线)

1. **Slides.com**:
   - 访问: https://slides.com/
   - 创建新演示
   - 复制 `docs/INVESTOR-PITCH-DECK.md` 内容
   - 按幻灯片分隔

2. **Beautiful.AI**:
   - 访问: https://www.beautiful.ai/
   - 使用 AI 模板创建
   - 手动输入内容

---

## 📊 已准备好的源文件

您已经拥有完整的**源文件**,可以直接使用:

### 白皮书源文件 (Markdown)

- ✅ `WHITEPAPER.md` (中文)

**直接分享**: 可以直接将 Markdown 文件上传到 GitHub, GitBook, 或在线 Markdown 阅读器

### 营销素材源文件

- ✅ `docs/SOCIAL-MEDIA-TEMPLATES.md` (社交媒体模板)
- ✅ `docs/INVESTOR-PITCH-DECK.md` (投资者 Deck)
- ✅ `docs/PDF-GENERATION-GUIDE.md` (PDF 生成指南)
- ✅ `docs/VISUAL-ASSETS-GUIDE.md` (视觉资产指南)

### 视觉资产源文件

- ✅ `assets/visuals/diagrams/architecture.mmd` (Mermaid 架构图)
- ✅ `assets/visuals/charts/generate-*.py` (Python 图表脚本)

---


### GitHub 上直接查看
1. 推送到 GitHub:
```bash
git commit -m "Add whitepapers"
```

2. 访问: `https://github.com/你的用户名/SuperVM/blob/main/WHITEPAPER.md`

GitHub 会自动渲染 Markdown！

### 使用 GitHub Pages

1. 创建 `docs/index.md`:
```markdown
# SuperVM Documentation

- [中文白皮书](../WHITEPAPER.md)
- [English Whitepaper](../WHITEPAPER_EN.md)
- [Investor Deck](INVESTOR-PITCH-DECK.md)
```

2. 在 GitHub 仓库设置中启用 GitHub Pages
3. 访问: `https://你的用户名.github.io/SuperVM/`

---

## ✅ 完成检查清单

### 文档准备 (已完成 ✅)

- [x] 中文白皮书 (WHITEPAPER.md)
- [x] 英文白皮书 (WHITEPAPER_EN.md)
- [x] 投资者 Pitch Deck (docs/INVESTOR-PITCH-DECK.md)
- [x] 社交媒体模板 (docs/SOCIAL-MEDIA-TEMPLATES.md)
- [x] PDF 生成指南 (docs/PDF-GENERATION-GUIDE.md)
- [x] 视觉资产指南 (docs/VISUAL-ASSETS-GUIDE.md)
- [x] 脚本目录 (scripts/)
- [x] 资源目录 (visuals/)

### 工具安装 (待完成 ⏳)

- [ ] Pandoc (PDF/PPT 生成)
- [ ] MiKTeX (LaTeX 引擎)
- [ ] Python + Matplotlib (图表生成)
- [ ] Mermaid CLI (架构图生成)

### 资产生成 (安装工具后)

- [ ] 生成 PDF 白皮书 (CN/EN)
- [ ] 生成双语 PPT
- [ ] 生成性能对比图 (performance-comparison.png)
- [ ] 生成 Gas 对比图 (gas-comparison.png)
- [ ] 生成代币分配图 (tokenomics.png)
- [ ] 生成架构图 PNG (architecture.png)

---

## 🎯 推荐的行动顺序

### 立即可做 (无需安装)

1. **审查源文件**:
   - 打开 `WHITEPAPER.md` 在 VS Code 中预览
   - 检查内容准确性

2. **在线分享**:
   - 推送到 GitHub 让他人查看
   - 使用 Markdown 转 PDF 在线工具

3. **手动创建 PPT**:
   - 复制 `docs/INVESTOR-PITCH-DECK.md` 内容
   - 在 PowerPoint 或 Google Slides 手动创建

### 安装工具后 (30 分钟)

1. 安装：Pandoc / MiKTeX / Python / Mermaid CLI
2. 运行：`./scripts/generate-pdfs.ps1`、`./scripts/generate-ppt.ps1`、`./scripts/generate-visuals.ps1`
3. 检查：`assets/pdf`、`assets/ppt`、`assets/visuals`

### 完整生成 (1 小时)

1. **安装所有工具** (Pandoc + Python + Mermaid)
2. **运行全部脚本**
3. **在 PowerPoint 中美化 PPT**
4. **导出高质量 PDF 和图片**

---

## 💡 最快路径 (5 分钟)

**如果你只想快速得到一个 PDF 白皮书**:

1. 访问: https://www.markdowntopdf.com/
2. 复制 `WHITEPAPER.md` 全部内容
3. 粘贴到网站
4. 点击 "Convert"
5. 下载 PDF

**完成！** ✅

---

## 📞 需要帮助?

**Pandoc 安装问题**: 
- 参考: https://pandoc.org/installing.html

**Python 图表问题**:
- 参考: `docs/VISUAL-ASSETS-GUIDE.md`

**PPT 美化技巧**:
- 参考: `docs/INVESTOR-PITCH-DECK.md` 末尾的 Appendix

---

**现在就开始吧！** 🚀

完整一键：
```powershell
./scripts/generate-all.ps1
```

选择上面任意一种方式,您的白皮书和营销资产已经准备好了。
