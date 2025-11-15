# SuperVM 工具安装指南

> **执行方案 A 需要安装的工具**

---

## 📋 需要安装的工具

1. **Pandoc** - PDF/PPT 生成核心工具
2. **MiKTeX** - LaTeX 引擎 (PDF 所需)
3. **Python + Matplotlib** - 图表生成 (可选)

---

## 🚀 方式 1: 手动下载安装 (推荐，最可靠)

### 步骤 1: 安装 Pandoc

1. **下载 Pandoc**:
   - 访问: https://github.com/jgm/pandoc/releases/latest
   - 下载: `pandoc-3.1.11.1-windows-x86_64.msi` (或最新版本)
   - 文件大小: ~80 MB

2. **安装**:
   - 双击 `.msi` 文件
   - 按照安装向导操作
   - 安装位置: 默认 `C:\Program Files\Pandoc\`
   - **重要**: 勾选 "Add to PATH"

3. **验证安装**:
   ```powershell
   # 重启 PowerShell 后运行
   pandoc --version
   ```
   应显示: `pandoc 3.1.11.1`

---

### 步骤 2: 安装 MiKTeX (LaTeX)

1. **下载 MiKTeX**:
   - 访问: https://miktex.org/download
   - 下载: `basic-miktex-24.1-x64.exe` (或最新版本)
   - 文件大小: ~280 MB

2. **安装**:
   - 双击 `.exe` 文件
   - 选择 "Install for all users" (推荐)
   - 安装位置: 默认 `C:\Program Files\MiKTeX\`
   - **重要设置**:
     - ✅ "Install missing packages on-the-fly: Yes"
     - ✅ "Automatically install packages"

3. **首次配置**:
   ```powershell
   # 更新包数据库
   mpm --update-db
   
   # 安装中文字体支持
   mpm --install=ctex
   ```

4. **验证安装**:
   ```powershell
   xelatex --version
   ```
   应显示: `XeTeX 3.x`

---

### 步骤 3: 测试 PDF 生成

```powershell
# 运行批处理脚本
.\scripts\generate-pdfs.bat
```

**预期输出**:
```
========================================
   SuperVM PDF Generation
========================================

Generating Chinese Whitepaper...
[等待 30-60 秒，首次运行会下载字体包]
✅ 生成成功

Generating English Whitepaper...
✅ 生成成功

Generating Investor Deck PDF...
✅ 生成成功

========================================
   PDF Generation Complete!
========================================
```

**检查输出文件**:
```powershell
dir pdf-output\*.pdf
```

应该看到 3 个 PDF 文件。

---

## 🔧 方式 2: 使用 Chocolatey (自动化)

### 步骤 1: 安装 Chocolatey

**以管理员身份运行 PowerShell**，然后执行:

```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
```

**验证安装**:
```powershell
choco --version
```

---

### 步骤 2: 使用 Chocolatey 安装工具

```powershell
# 安装 Pandoc
choco install pandoc -y

# 安装 MiKTeX
choco install miktex -y

# 安装 Python (可选，用于图表生成)
choco install python -y
```

**验证安装**:
```powershell
pandoc --version
xelatex --version
python --version
```

---

## 🎨 可选: 安装 Python 图表工具

### 方式 A: 手动安装 Python

1. **下载 Python**:
   - 访问: https://www.python.org/downloads/
   - 下载: `python-3.12.x-amd64.exe`
   - **重要**: 安装时勾选 "Add Python to PATH"

2. **安装 Matplotlib**:
   ```powershell
   pip install matplotlib
   ```

3. **生成图表**:
   ```powershell
   python visuals\charts\generate-performance.py
   python visuals\charts\generate-gas.py
   python visuals\charts\generate-tokenomics.py
   ```

### 方式 B: 使用 Chocolatey

```powershell
choco install python -y
pip install matplotlib
```

---

## 📊 完整工作流程

### 1. 安装工具 (一次性)

```powershell
# 手动下载并安装:
# - Pandoc: https://github.com/jgm/pandoc/releases
# - MiKTeX: https://miktex.org/download

# 或使用 Chocolatey:
choco install pandoc miktex -y
```

### 2. 生成 PDF

```powershell
# 重启 PowerShell 让 PATH 生效
# 然后运行:
.\scripts\generate-pdfs.bat
```

**首次运行注意事项**:
- MiKTeX 会自动下载缺失的字体包
- 中文字体包 (~100 MB) 下载需要 1-3 分钟
- 请保持网络连接

### 3. 生成 PowerPoint

```powershell
pandoc docs\INVESTOR-PITCH-DECK.md -o pdf-output\SuperVM_Pitch_Deck.pptx --to pptx --slide-level=1
```

### 4. 生成图表 (可选)

```powershell
# 如果安装了 Python
python visuals\charts\generate-performance.py
python visuals\charts\generate-gas.py
python visuals\charts\generate-tokenomics.py
```

---

## 🐛 常见问题排查

### 问题 1: "pandoc: command not found"

**原因**: PATH 环境变量未更新

**解决方案**:
1. 重启 PowerShell 终端
2. 或手动添加到 PATH:
   - 系统属性 → 环境变量
   - 添加: `C:\Program Files\Pandoc`

### 问题 2: "xelatex.exe 不是内部或外部命令"

**原因**: MiKTeX 未安装或 PATH 未更新

**解决方案**:
1. 检查安装: `C:\Program Files\MiKTeX\miktex\bin\x64\`
2. 重启终端
3. 或重新安装 MiKTeX

### 问题 3: PDF 生成失败 "Font 'SimSun' not found"

**原因**: 中文字体包未安装

**解决方案**:
```powershell
# 手动安装字体包
mpm --install=ctex
mpm --install=cjk
mpm --install=xecjk
```

### 问题 4: 首次生成 PDF 很慢 (30-60 秒)

**原因**: MiKTeX 正在下载字体包

**解决方案**:
- 这是正常现象
- 等待完成即可
- 后续生成会很快 (~5 秒)

### 问题 5: "Permission denied" 错误

**原因**: 防火墙或杀毒软件阻止

**解决方案**:
1. 以管理员身份运行 PowerShell
2. 临时关闭杀毒软件
3. 或添加 Pandoc/MiKTeX 到白名单

---

## 📂 预期输出

### PDF 文件 (pdf-output/)

```
pdf-output/
├── SuperVM_Whitepaper_CN_v1.0.pdf      (~3 MB)
├── SuperVM_Whitepaper_EN_v1.0.pdf      (~3 MB)
└── SuperVM_Investor_Deck_v1.0.pdf      (~2 MB)
```

### PowerPoint 文件

```
pdf-output/
└── SuperVM_Pitch_Deck.pptx             (~500 KB)
```

### 图表文件 (visuals/charts/)

```
visuals/charts/
├── performance-comparison.png          (~200 KB)
├── gas-comparison.png                  (~180 KB)
└── tokenomics.png                      (~150 KB)
```

---

## ✅ 安装完成检查清单

安装完成后，运行以下命令验证:

```powershell
# 检查 Pandoc
pandoc --version
# 应显示: pandoc 3.1.x

# 检查 LaTeX
xelatex --version
# 应显示: XeTeX 3.x

# 检查 Python (可选)
python --version
# 应显示: Python 3.12.x

pip list | findstr matplotlib
# 应显示: matplotlib  3.x.x
```

如果所有命令都正常显示版本号，说明安装成功！

---

## 🚀 下一步

**安装完成后**，运行:

```powershell
# 生成所有 PDF
.\scripts\generate-pdfs.bat

# 或使用 PowerShell 版本
powershell -ExecutionPolicy Bypass -File .\scripts\generate-pdfs.ps1

# 生成 PowerPoint
pandoc docs\INVESTOR-PITCH-DECK.md -o pdf-output\SuperVM_Pitch_Deck.pptx --to pptx
```

**预计耗时**:
- 首次运行: 1-3 分钟 (下载字体包)
- 后续运行: 10-30 秒

---

## 📞 需要帮助?

**官方文档**:
- Pandoc: https://pandoc.org/installing.html
- MiKTeX: https://miktex.org/howto/install-miktex

**本地文档**:
- 详细指南: `ASSETS-README.md`
- 快速开始: `QUICK-START-ASSETS.md`
- PDF 指南: `docs/PDF-GENERATION-GUIDE.md`

---

**准备好了吗? 让我们开始安装！** 🚀
