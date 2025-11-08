#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Update ROADMAP.md Phase 4.3 progress"""

import re

# 读取文件
with open('ROADMAP.md', 'r', encoding='utf-8') as f:
    content = f.read()

print("原文件长度:", len(content))

# 查找目标文本
phase43_pattern = r'##\s*[�]?💾\s*Phase 4\.3:\s*持久化存储集成专项\s*\([📋�]*规划中[📋�]*\)'
matches = re.findall(phase43_pattern, content)
print(f"找到 {len(matches)} 处 Phase 4.3 标题")
if matches:
    print("匹配内容:", matches[0])

# 替换标题行 (处理可能的特殊字符)
new_content = re.sub(
    phase43_pattern,
    '## 💾 Phase 4.3: 持久化存储集成专项 (🚧 进行中 50%)',
    content
)

if new_content != content:
    print("✓ 标题已替换")
else:
    print("✗ 标题未替换 - 尝试简化模式")
    # 尝试更简单的匹配
    new_content = content.replace(
        'Phase 4.3: 持久化存储集成专项 (📋 规划中)',
        'Phase 4.3: 持久化存储集成专项 (🚧 进行中 50%)'
    )
    if new_content != content:
        print("✓ 使用简化模式替换成功")

content = new_content

# 替换元信息行
meta_pattern = r'\*\*时间\*\*:\s*预计\s*3-4\s*周\s*\|\s*\*\*完成度\*\*:\s*0%\s*\|\s*\*\*优先级\*\*:\s*🟡\s*中(?!\s*\|)'
matches2 = re.findall(meta_pattern, content)
print(f"找到 {len(matches2)} 处元信息")

new_content = re.sub(
    meta_pattern,
    '**时间**: 预计 3-4 周 | **完成度**: 50% | **优先级**: 🟡 中 | **最后更新**: 2025-11-07',
    content
)

if new_content != content:
    print("✓ 元信息已替换")
else:
    print("✗ 元信息未替换 - 尝试简化模式")
    new_content = content.replace(
        '**时间**: 预计 3-4 周 | **完成度**: 0% | **优先级**: 🟡 中',
        '**时间**: 预计 3-4 周 | **完成度**: 50% | **优先级**: 🟡 中 | **最后更新**: 2025-11-07'
    )
    if new_content != content:
        print("✓ 使用简化模式替换成功")

# 写入文件
with open('ROADMAP.md', 'w', encoding='utf-8', newline='\n') as f:
    f.write(new_content)

print("✅ ROADMAP.md 已更新")
