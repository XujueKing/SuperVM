#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
修复UTF-8编码被错误解释为GBK后又存为UTF-8的文件
"""

import os
import sys

def fix_double_encoded_file(input_file):
    """
    修复双重编码问题:
    UTF-8的中文字节被误读为GBK,显示为乱码,然后这些乱码又被保存成UTF-8
    
    解决方法:
    1. 读取文件为UTF-8(得到乱码字符串)
    2. 将乱码字符串编码为GBK字节(还原原始UTF-8字节)
    3. 用UTF-8正确解码这些字节
    """
    try:
        print(f"正在处理 {input_file} ...")
        
        # 读取文件(UTF-8编码)
        with open(input_file, 'r', encoding='utf-8', errors='ignore') as f:
            wrong_text = f.read()
        
        # 将乱码文本编码为GBK字节(这会还原原始的UTF-8字节)
        # 然后用UTF-8正确解码
        try:
            # 方法1: 通过GBK编码还原UTF-8字节
            utf8_bytes = wrong_text.encode('gbk', errors='ignore')
            correct_text = utf8_bytes.decode('utf-8', errors='ignore')
        except Exception:
            # 方法2: 如果方法1失败,尝试直接使用latin-1
            utf8_bytes = wrong_text.encode('latin-1', errors='ignore')
            correct_text = utf8_bytes.decode('utf-8', errors='ignore')
        
        # 写回文件
        with open(input_file, 'w', encoding='utf-8', newline='\n') as f:
            f.write(correct_text)
        
        print(f"✓ 已修复 {input_file}")
        return True
        
    except Exception as e:
        print(f"✗ 修复 {input_file} 失败: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == '__main__':
    files = [
        'docs/gc-observability.md',
        'docs/stress-testing-guide.md'
    ]
    
    print("="*60)
    print("开始修复文件编码...")
    print("问题: UTF-8字节被误读为GBK,然后又存为UTF-8")
    print("="*60)
    print()
    
    success_count = 0
    
    for file in files:
        if os.path.exists(file):
            if fix_double_encoded_file(file):
                success_count += 1
            print()
        else:
            print(f"✗ 文件不存在: {file}")
            print()
    
    print("="*60)
    print(f"完成! 成功修复 {success_count}/{len(files)} 个文件")
    print("="*60)
    
    sys.exit(0 if success_count == len(files) else 1)
