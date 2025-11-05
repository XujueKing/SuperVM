$files = @(
    "docs\parallel-execution.md",
    "docs\gc-observability.md", 
    "docs\stress-testing-guide.md"
)

foreach ($file in $files) {
    Write-Host "处理 $file ..."
    $lines = Get-Content -Path $file -Encoding UTF8
    $cleaned = @()
    
    foreach ($line in $lines) {
        # 检查是否包含乱码字符(如鈺、閫、骞、姒等)
        if ($line -match '[\u9482\u9480\u9485\u9489\u9486\u948A\u948C\u948E\u9491\u9493\u9495\u9499\u949A\u949C\u949E\u94A0\u94A2\u94A5\u94A6\u94A8\u94AA\u94AC\u94AE\u94B1\u94B3\u94B5\u94B7\u94B9\u94BB\u94BD\u94BF\u94C1\u94C3\u94C5\u94C6]') {
            # 尝试分割出正确的中文部分
            # 通常格式是: "正确中文乱码文本"
            # 我们只保留开头的正确中文,直到第一个乱码字符
            $cleaned_line = $line -replace '[\u9482\u9480\u9485\u9489\u9486\u948A\u948C\u948E\u9491\u9493\u9495\u9499\u949A\u949C\u949E\u94A0\u94A2\u94A5\u94A6\u94A8\u94AA\u94AC\u94AE\u94B1\u94B3\u94B5\u94B7\u94B9\u94BB\u94BD\u94BF\u94C1\u94C3\u94C5\u94C6].*$', ''
            $cleaned += $cleaned_line
        } else {
            $cleaned += $line
        }
    }
    
    $cleaned | Set-Content -Path $file -Encoding UTF8
    Write-Host " 已清理 $file (从 $($lines.Count) 行处理完成)"
}

Write-Host "`n完成!"
