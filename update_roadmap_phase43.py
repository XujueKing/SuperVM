#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
更新 ROADMAP.md 中 Phase 4.3 的进度
"""

import re

def update_roadmap():
    # 读取文件 (UTF-8)
    with open('ROADMAP.md', 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 更新 Phase 4.3 状态表格
    content = re.sub(
        r'\| \*\*Phase 4\.3\*\* \| \*\*持久化存储集成\*\* \| [^|]+ \| \d+% \| Week \d+/\d+ \|',
        '| **Phase 4.3** | **持久化存储集成** | 🚧 进行中 | 40% | Week 3-4/4 |',
        content
    )
    
    # 更新 Phase 4.3 章节标题
    content = re.sub(
        r'## 💾 Phase 4\.3: 持久化存储集成专项 \([^)]+\)',
        '## 💾 Phase 4.3: 持久化存储集成专项 (🚧 进行中)',
        content
    )
    
    # 更新完成度
    content = re.sub(
        r'(\*\*时间\*\*: 预计 3-4 周 \| \*\*完成度\*\*: )\d+%',
        r'\g<1>40%',
        content
    )
    
    # 写回文件 (保持 UTF-8 with BOM)
    with open('ROADMAP.md', 'w', encoding='utf-8-sig') as f:
        f.write(content)
    
    print("✅ ROADMAP.md 更新成功!")
    print("  - Phase 4.3 状态: 🚧 进行中")
    print("  - 完成度: 40%")
    print("  - 周次: Week 3-4/4")

if __name__ == '__main__':
    update_roadmap()
