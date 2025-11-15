#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
统一 ROADMAP.md 编码为 UTF-8 (without BOM) 并更新 Phase 4.3 进度
"""
import re

def main():
    print("📝 开始处理 ROADMAP.md...")
    
    # 读取文件 (自动处理 BOM)
    try:
        with open('ROADMAP.md', 'r', encoding='utf-8-sig') as f:
            content = f.read()
        print("✅ 文件读取成功 (UTF-8-sig)")
    except Exception as e:
        print(f"❌ 读取失败: {e}")
        return
    
    original_content = content
    
    # 更新1: Phase 4.3 表格行
    pattern1 = r'\| \*\*Phase 4\.3\*\* \| \*\*持久化存储集成\*\* \| [^|]+ \| \d+% \| Week \d+/\d+ \|'
    replacement1 = '| **Phase 4.3** | **持久化存储集成** | 🚧 进行中 | 40% | Week 3-4/4 |'
    content = re.sub(pattern1, replacement1, content)
    if pattern1 != content:
        print("✅ 更新表格: Phase 4.3 进度 35% -> 40%")
    
    # 更新2: Phase 4.3 章节标题
    pattern2 = r'## 💾 Phase 4\.3: 持久化存储集成专项 \([^)]+\)'
    replacement2 = '## 💾 Phase 4.3: 持久化存储集成专项 (🚧 进行中)'
    content = re.sub(pattern2, replacement2, content)
    if pattern2 != content:
        print("✅ 更新章节: 状态改为 🚧 进行中")
    
    # 更新3: 完成度百分比
    pattern3 = r'(\*\*时间\*\*: 预计 3-4 周 \| \*\*完成度\*\*: )\d+(%)'
    replacement3 = r'\g<1>40\g<2>'
    content = re.sub(pattern3, replacement3, content)
    if pattern3 != content:
        print("✅ 更新完成度: 0% -> 40%")
    
    # 检查是否有变化
    if content == original_content:
        print("⚠️  警告: 未找到匹配内容,可能已经更新或格式变化")
        print("正在尝试模糊匹配...")
        
        # 模糊匹配: 只要包含 Phase 4.3 的行
        lines = content.split('\n')
        updated = False
        for i, line in enumerate(lines):
            # 更新表格行
            if 'Phase 4.3' in line and '持久化存储集成' in line and '35%' in line:
                lines[i] = '| **Phase 4.3** | **持久化存储集成** | 🚧 进行中 | 40% | Week 3-4/4 |'
                print(f"✅ 模糊匹配更新第 {i+1} 行")
                updated = True
            # 更新章节标题
            if 'Phase 4.3: 持久化存储集成专项' in line and '规划中' in line:
                lines[i] = '## 💾 Phase 4.3: 持久化存储集成专项 (🚧 进行中)'
                print(f"✅ 模糊匹配更新第 {i+1} 行")
                updated = True
            # 更新完成度
            if '完成度**: 0%' in line and i > 0 and 'Phase 4.3' in '\n'.join(lines[max(0,i-5):i]):
                lines[i] = re.sub(r'完成度\*\*: 0%', '完成度**: 40%', lines[i])
                print(f"✅ 模糊匹配更新第 {i+1} 行 (完成度)")
                updated = True
        
        if updated:
            content = '\n'.join(lines)
            print("✅ 模糊匹配成功")
        else:
            print("❌ 未能找到任何匹配项")
            return
    
    # 写回文件 (UTF-8 without BOM)
    try:
        with open('ROADMAP.md', 'w', encoding='utf-8', newline='\n') as f:
            f.write(content)
        print("✅ 文件保存成功 (UTF-8 without BOM)")
    except Exception as e:
        print(f"❌ 保存失败: {e}")
        return
    
    # 验证文件编码
    with open('ROADMAP.md', 'rb') as f:
        first_bytes = f.read(3)
        if first_bytes == b'\xef\xbb\xbf':
            print("⚠️  检测到 BOM 标记")
        else:
            print("✅ 确认: 文件为纯 UTF-8 (无 BOM)")
    
    print("\n🎉 ROADMAP.md 更新完成!")
    print("   - 编码: UTF-8 (without BOM)")
    print("   - Phase 4.3 状态: 🚧 进行中")
    print("   - 完成度: 40%")
    print("   - 周次: Week 3-4/4")

if __name__ == '__main__':
    main()
