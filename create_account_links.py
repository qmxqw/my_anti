#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
账户硬链接管理脚本
功能:
1. 删除 acc 目录下所有 .json 文件
2. 读取 accounts.json
3. 为每个账户在 acc 目录下创建硬链接(使用 email @ 之前的部分作为文件名)
"""

import os
import json
import glob
from pathlib import Path


def delete_acc_json_files(acc_dir="acc"):
    """删除 acc 目录下所有 .json 文件"""
    if not os.path.exists(acc_dir):
        print(f"目录 {acc_dir} 不存在,跳过删除操作")
        return
    
    json_files = glob.glob(os.path.join(acc_dir, "*.json"))
    deleted_count = 0
    
    for file_path in json_files:
        try:
            os.remove(file_path)
            print(f"已删除: {file_path}")
            deleted_count += 1
        except Exception as e:
            print(f"删除失败 {file_path}: {e}")
    
    print(f"共删除 {deleted_count} 个文件")


def create_account_links(accounts_json="accounts.json", acc_dir="acc", accounts_dir="accounts"):
    """读取 accounts.json 并创建硬链接"""
    # 确保 acc 目录存在
    if not os.path.exists(acc_dir):
        os.makedirs(acc_dir, exist_ok=True)
        print(f"创建目录: {acc_dir}")
    
    # 读取 accounts.json
    if not os.path.exists(accounts_json):
        print(f"错误: {accounts_json} 文件不存在")
        return
    
    try:
        with open(accounts_json, 'r', encoding='utf-8') as f:
            data = json.load(f)
    except Exception as e:
        print(f"读取 {accounts_json} 失败: {e}")
        return
    
    accounts = data.get("accounts", [])
    if not accounts:
        print("警告: accounts.json 中没有账户数据")
        return
    
    print(f"\n找到 {len(accounts)} 个账户,开始创建硬链接...")
    
    success_count = 0
    for account in accounts:
        account_id = account.get("id")
        email = account.get("email")
        
        if not account_id or not email:
            print(f"跳过无效账户: {account}")
            continue
        
        # 提取 email @ 之前的部分
        email_prefix = email.split('@')[0] if '@' in email else email
        
        # 源文件路径
        source_file = os.path.join(accounts_dir, f"{account_id}.json")
        # 硬链接路径
        link_file = os.path.join(acc_dir, f"{email_prefix}.json")
        
        # 检查源文件是否存在
        if not os.path.exists(source_file):
            print(f"警告: 源文件不存在 {source_file}")
            continue
        
        try:
            # 创建硬链接
            os.link(source_file, link_file)
            print(f"✓ 创建硬链接: {link_file} -> {source_file}")
            success_count += 1
        except FileExistsError:
            print(f"跳过: {link_file} 已存在")
        except Exception as e:
            print(f"创建硬链接失败 {link_file}: {e}")
    
    print(f"\n完成! 成功创建 {success_count} 个硬链接")


def main():
    print("=" * 60)
    print("账户硬链接管理脚本")
    print("=" * 60)
    
    # 步骤 1: 删除 acc 目录下所有 .json 文件
    print("\n[步骤 1] 删除 acc 目录下所有 .json 文件")
    delete_acc_json_files()
    
    # 步骤 2: 创建硬链接
    print("\n[步骤 2] 创建账户硬链接")
    create_account_links()
    
    print("\n" + "=" * 60)
    print("操作完成!")
    print("=" * 60)


if __name__ == "__main__":
    main()
