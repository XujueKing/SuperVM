# SuperVM 白皮书 PDF 生成指南

> 使用 Pandoc 将 Markdown 转换为专业排版的 PDF

---

## 🎯 快速开始

### 安装依赖

**Windows (PowerShell):**
```powershell
# 安装 Pandoc
choco install pandoc

# 安装 LaTeX (MiKTeX 或 TeX Live)
choco install miktex

# 安装中文字体支持
# 系统自带宋体/黑体即可,或安装思源字体:
choco install sourcehanserif
```

**macOS:**
```bash
brew install pandoc
brew install --cask mactex
```

**Linux (Ubuntu/Debian):**
```bash
sudo apt install pandoc texlive-full fonts-noto-cjk
```

---

## 📄 基础转换命令

### 中文白皮书

```powershell
pandoc WHITEPAPER.md -o SuperVM_Whitepaper_CN_v1.0.pdf `
  --pdf-engine=xelatex `
  --toc `
  --toc-depth=2 `
  --number-sections `
  -V CJKmainfont="SimSun" `
  -V CJKsansfont="SimHei" `
  -V CJKmonofont="FangSong" `
  -V geometry:margin=1in `
  -V fontsize=11pt `
  -V documentclass=article `
  -V papersize=a4 `
  -V colorlinks=true `
  -V linkcolor=blue `
  -V urlcolor=blue `
  -V toccolor=black
```

### 英文白皮书

```powershell
pandoc WHITEPAPER_EN.md -o SuperVM_Whitepaper_EN_v1.0.pdf `
  --pdf-engine=xelatex `
  --toc `
  --toc-depth=2 `
  --number-sections `
  -V mainfont="Times New Roman" `
  -V sansfont="Arial" `
  -V monofont="Courier New" `
  -V geometry:margin=1in `
  -V fontsize=11pt `
  -V documentclass=article `
  -V papersize=letter `
  -V colorlinks=true `
  -V linkcolor=NavyBlue `
  -V urlcolor=RoyalBlue
```

---

## 🎨 专业版本（带封面 + 页眉页脚）

### 步骤 1: 创建封面模板

创建 `cover-page.tex`:

```latex
\begin{titlepage}
    \centering
    \vspace*{2cm}
    
    % Logo (如果有)
    % \includegraphics[width=0.4\textwidth]{logo.png}
    
    \vspace{1cm}
    
    {\Huge \textbf{SuperVM}}
    
    \vspace{0.5cm}
    
    {\LARGE 潘多拉星核技术白皮书}
    
    \vspace{0.3cm}
    
    {\Large Pandora Core: The Web3 Operating System}
    
    \vspace{2cm}
    
    {\Large 打开 Web3 的潘多拉魔盒}
    
    \vspace{1cm}
    
    {\large 版本 1.0 | 2025年1月}
    
    \vfill
    
    {\large 
    \textbf{核心创新:} \\
    242K TPS 性能引擎 • 多链原生融合 • 内置隐私保护 • 神经网络式自组织通信
    }
    
    \vspace{1cm}
    
    {\small
    \textbf{联系方式:} \\
    网站: supervm.io \\
    邮箱: contact@supervm.io \\
    GitHub: github.com/idkbreh/SuperVM
    }
    
    \vspace{0.5cm}
    
    {\footnotesize
    © 2025 SuperVM Foundation. All rights reserved. \\
    本文档受 Creative Commons CC-BY-NC-ND 4.0 协议保护
    }
\end{titlepage}

\newpage
\tableofcontents
\newpage
```

### 步骤 2: 创建页眉页脚模板

创建 `header-footer.tex`:

```latex
% 页眉页脚设置
\usepackage{fancyhdr}
\usepackage{lastpage}

\pagestyle{fancy}
\fancyhf{} % 清空默认设置

% 页眉
\fancyhead[L]{\small SuperVM 技术白皮书 v1.0}
\fancyhead[R]{\small \leftmark}

% 页脚
\fancyfoot[C]{\small 第 \thepage\ 页 共 \pageref{LastPage} 页}
\fancyfoot[R]{\small © 2025 SuperVM Foundation}

% 页眉线
\renewcommand{\headrulewidth}{0.4pt}
\renewcommand{\footrulewidth}{0.4pt}
```

### 步骤 3: 完整转换命令

```powershell
pandoc WHITEPAPER.md -o SuperVM_Whitepaper_CN_Professional_v1.0.pdf `
  --pdf-engine=xelatex `
  --toc `
  --toc-depth=2 `
  --number-sections `
  -V CJKmainfont="SimSun" `
  -V CJKsansfont="SimHei" `
  -V geometry:margin=1in `
  -V fontsize=11pt `
  -V documentclass=article `
  -V papersize=a4 `
  -V colorlinks=true `
  -V linkcolor=blue `
  --include-before-body=cover-page.tex `
  --include-in-header=header-footer.tex `
  --highlight-style=tango `
  --metadata title="SuperVM 技术白皮书" `
  --metadata author="SuperVM 基金会" `
  --metadata date="2025年1月"
```

---

## 🌟 高级定制选项

### 添加水印

创建 `watermark.tex`:

```latex
\usepackage{draftwatermark}
\SetWatermarkText{CONFIDENTIAL}
\SetWatermarkScale{0.5}
\SetWatermarkColor[gray]{0.9}
```

在转换命令中添加:
```powershell
--include-in-header=watermark.tex
```

### 代码块语法高亮

```powershell
# 查看可用主题
pandoc --list-highlight-styles

# 推荐主题: tango, pygments, kate, monochrome
--highlight-style=tango
```

### 插入图片

在 Markdown 中:
```markdown
![架构图](docs/images/architecture.png){width=80%}
```

确保图片路径相对于 Markdown 文件。

### 自定义 CSS (HTML 转 PDF)

创建 `style.css`:
```css
body {
    font-family: "Noto Serif CJK SC", "SimSun", serif;
    line-height: 1.6;
    max-width: 800px;
    margin: 0 auto;
    padding: 2em;
}

h1 { color: #2c3e50; border-bottom: 2px solid #3498db; }
h2 { color: #34495e; }
code { background-color: #f4f4f4; padding: 2px 5px; }
```

转换命令:
```powershell
pandoc WHITEPAPER.md -o whitepaper.html --css=style.css --standalone
```

---

## 📊 自动化脚本

### PowerShell 批量生成脚本

创建 `scripts/generate-pdfs.ps1`:

```powershell
# SuperVM PDF 生成脚本
# 用法: .\scripts\generate-pdfs.ps1

$ErrorActionPreference = "Stop"

Write-Host "🚀 开始生成 SuperVM PDF 文档..." -ForegroundColor Cyan

# 检查 Pandoc 是否安装
if (-not (Get-Command pandoc -ErrorAction SilentlyContinue)) {
    Write-Host "❌ 错误: 未找到 Pandoc,请先安装: choco install pandoc" -ForegroundColor Red
    exit 1
}

# 创建输出目录
$outputDir = "pdf-output"
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

# 中文白皮书 - 简洁版
Write-Host "📄 生成中文白皮书 (简洁版)..." -ForegroundColor Yellow
pandoc WHITEPAPER.md -o "$outputDir/SuperVM_Whitepaper_CN_v1.0.pdf" `
  --pdf-engine=xelatex `
  --toc `
  --toc-depth=2 `
  --number-sections `
  -V CJKmainfont="SimSun" `
  -V geometry:margin=1in `
  -V fontsize=11pt `
  -V colorlinks=true

# 英文白皮书 - 简洁版
Write-Host "📄 生成英文白皮书 (简洁版)..." -ForegroundColor Yellow
pandoc WHITEPAPER_EN.md -o "$outputDir/SuperVM_Whitepaper_EN_v1.0.pdf" `
  --pdf-engine=xelatex `
  --toc `
  --toc-depth=2 `
  --number-sections `
  -V mainfont="Times New Roman" `
  -V geometry:margin=1in `
  -V fontsize=11pt `
  -V colorlinks=true

# 投资者 Pitch Deck - PDF
Write-Host "📊 生成投资者 Pitch Deck..." -ForegroundColor Yellow
pandoc docs/INVESTOR-PITCH-DECK.md -o "$outputDir/SuperVM_Investor_Deck_v1.0.pdf" `
  --pdf-engine=xelatex `
  -V CJKmainfont="SimSun" `
  -V geometry:margin=0.75in `
  -V fontsize=14pt `
  -V colorlinks=true `
  -V linkcolor=NavyBlue

# 技术文档合集
Write-Host "📚 生成技术文档合集..." -ForegroundColor Yellow
pandoc `
  docs/ARCHITECTURE-INTEGRATION-ANALYSIS.md `
  docs/AUTO-TUNER.md `
  docs/DUAL-CURVE-VERIFIER-GUIDE.md `
  docs/KERNEL-DEFINITION.md `
  -o "$outputDir/SuperVM_Technical_Docs_v1.0.pdf" `
  --pdf-engine=xelatex `
  --toc `
  -V CJKmainfont="SimSun" `
  -V geometry:margin=1in `
  -V fontsize=10pt

Write-Host "✅ 所有 PDF 已生成到: $outputDir" -ForegroundColor Green
Write-Host ""
Write-Host "📂 生成的文件:" -ForegroundColor Cyan
Get-ChildItem $outputDir -Filter *.pdf | ForEach-Object {
    $size = [math]::Round($_.Length / 1MB, 2)
    Write-Host "  • $($_.Name) ($size MB)" -ForegroundColor White
}
```

### Bash 版本 (Linux/macOS)

创建 `scripts/generate-pdfs.sh`:

```bash
#!/bin/bash
# SuperVM PDF 生成脚本

set -e

echo "🚀 开始生成 SuperVM PDF 文档..."

# 检查依赖
command -v pandoc >/dev/null 2>&1 || { echo "❌ 未找到 Pandoc"; exit 1; }

# 创建输出目录
mkdir -p pdf-output

# 中文白皮书
echo "📄 生成中文白皮书..."
pandoc WHITEPAPER.md -o pdf-output/SuperVM_Whitepaper_CN_v1.0.pdf \
  --pdf-engine=xelatex \
  --toc \
  --toc-depth=2 \
  --number-sections \
  -V CJKmainfont="Noto Serif CJK SC" \
  -V geometry:margin=1in \
  -V fontsize=11pt \
  -V colorlinks=true

# 英文白皮书
echo "📄 生成英文白皮书..."
pandoc WHITEPAPER_EN.md -o pdf-output/SuperVM_Whitepaper_EN_v1.0.pdf \
  --pdf-engine=xelatex \
  --toc \
  --toc-depth=2 \
  --number-sections \
  -V mainfont="Times New Roman" \
  -V geometry:margin=1in \
  -V fontsize=11pt \
  -V colorlinks=true

echo "✅ PDF 生成完成: pdf-output/"
ls -lh pdf-output/*.pdf
```

---

## 🔍 质量检查

### 验证 PDF

```powershell
# 检查 PDF 元数据
pdfinfo SuperVM_Whitepaper_CN_v1.0.pdf

# 检查书签/目录
pdftotext -layout SuperVM_Whitepaper_CN_v1.0.pdf - | head -50

# 验证中文字体嵌入
pdffonts SuperVM_Whitepaper_CN_v1.0.pdf
```

### 常见问题排查

**问题 1: 中文显示为方框**
```
解决: 确保安装了中文字体
Windows: 系统默认有宋体/黑体
Linux: sudo apt install fonts-noto-cjk
macOS: 系统自带
```

**问题 2: 代码块溢出页面**
```
解决: 添加 listings 设置
--listings
-V listings-disable-line-numbers=true
```

**问题 3: 图片未显示**
```
解决: 使用绝对路径或确保相对路径正确
![图片](./docs/images/arch.png)
```

---

## 📤 发布清单

- [ ] 生成中英文 PDF (简洁版 + 专业版)
- [ ] 验证所有链接可点击
- [ ] 检查目录页码准确
- [ ] 确认字体嵌入 (pdffonts 命令)
- [ ] 测试在多个 PDF 阅读器打开 (Adobe, Preview, Sumatra)
- [ ] 压缩 PDF 减小文件大小 (可选)
  ```bash
  gs -sDEVICE=pdfwrite -dCompatibilityLevel=1.4 \
     -dPDFSETTINGS=/ebook -dNOPAUSE -dQUIET -dBATCH \
     -sOutputFile=output_compressed.pdf input.pdf
  ```
- [ ] 上传到网站 + GitHub Releases
- [ ] 生成 SHA256 校验和
  ```powershell
  Get-FileHash SuperVM_Whitepaper_CN_v1.0.pdf -Algorithm SHA256
  ```

---

## 🎯 最佳实践

1. **版本控制**: 文件名包含版本号 (v1.0, v1.1)
2. **元数据**: 使用 `--metadata` 添加作者/标题/日期
3. **文件大小**: 目标 < 5MB (压缩图片,优化字体)
4. **可访问性**: 添加 PDF 书签 (--toc)
5. **安全性**: 敏感版本添加水印或密码保护
   ```powershell
   # 使用 qpdf 加密
   qpdf --encrypt user-password owner-password 256 -- input.pdf output.pdf
   ```

---

## 📚 参考资源

- **Pandoc 官方文档**: https://pandoc.org/MANUAL.html
- **LaTeX 中文支持**: https://www.overleaf.com/learn/latex/Chinese
- **PDF 元数据标准**: https://www.pdfa.org/
- **字体推荐**:
  - 中文: 思源宋体 (Noto Serif CJK SC), 方正书宋
  - 英文: Times New Roman, Georgia, Palatino
  - 代码: Fira Code, JetBrains Mono, Consolas

---

**生成示例:**

```powershell
# 一键生成所有版本
.\scripts\generate-pdfs.ps1

# 手动生成单个文件 (高质量)
pandoc WHITEPAPER.md -o whitepaper.pdf `
  --pdf-engine=xelatex `
  --toc `
  --number-sections `
  -V CJKmainfont="SimSun" `
  -V geometry:margin=1in `
  -V fontsize=11pt `
  -V colorlinks=true `
  -V linkcolor=blue `
  --highlight-style=tango `
  --metadata title="SuperVM 白皮书" `
  --metadata author="SuperVM 基金会" `
  --metadata date="$(Get-Date -Format 'yyyy-MM-dd')"
```

🎉 **生成完成后,PDF 将包含:**
- ✅ 专业封面
- ✅ 完整目录 (带页码)
- ✅ 章节编号
- ✅ 可点击链接
- ✅ 语法高亮代码
- ✅ 页眉页脚
- ✅ 嵌入字体 (可跨平台查看)
