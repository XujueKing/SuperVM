@echo off
chcp 65001 >nul
echo.
echo ========================================
echo    SuperVM PDF Generation
echo ========================================
echo.

if not exist "pdf-output" mkdir pdf-output

echo Generating Chinese Whitepaper...
pandoc WHITEPAPER.md -o pdf-output/SuperVM_Whitepaper_CN_v1.0.pdf ^
  --pdf-engine=xelatex ^
  --toc ^
  --toc-depth=2 ^
  --number-sections ^
  -V CJKmainfont="SimSun" ^
  -V geometry:margin=1in ^
  -V fontsize=11pt ^
  -V colorlinks=true ^
  --metadata title="SuperVM Whitepaper" ^
  --metadata author="SuperVM Foundation"

echo.
echo Generating English Whitepaper...
pandoc WHITEPAPER_EN.md -o pdf-output/SuperVM_Whitepaper_EN_v1.0.pdf ^
  --pdf-engine=xelatex ^
  --toc ^
  --toc-depth=2 ^
  --number-sections ^
  -V mainfont="Times New Roman" ^
  -V geometry:margin=1in ^
  -V fontsize=11pt ^
  -V colorlinks=true ^
  --metadata title="SuperVM Whitepaper" ^
  --metadata author="SuperVM Foundation"

echo.
echo Generating Investor Deck PDF...
pandoc docs/INVESTOR-PITCH-DECK.md -o pdf-output/SuperVM_Investor_Deck_v1.0.pdf ^
  --pdf-engine=xelatex ^
  -V CJKmainfont="SimSun" ^
  -V geometry:margin=0.75in ^
  -V fontsize=12pt ^
  -V colorlinks=true ^
  --metadata title="SuperVM Investor Deck"

echo.
echo ========================================
echo    PDF Generation Complete!
echo ========================================
echo.
dir pdf-output\*.pdf
echo.
pause
