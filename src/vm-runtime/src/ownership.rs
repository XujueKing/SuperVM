// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

// SuperVM 2.0 - Object Ownership Model (Sui-Inspired)
// 
// 架构师: KING XU (CHINA)
// 创建日期: 2025-11-04
//
// 核心功能：
// 1. 独占对象（Owned Objects）- 快速路径，无需共识，200K+ TPS
// 2. 共享对象（Shared Objects）- 共识路径，MVCC，10-20K TPS
// 3. 不可变对象（Immutable Objects）- 超快路径，读取零成本
// 4. 对象版本管理 - 支持对象升级和回滚

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

// ============================================================================
// 核心数据结构
// ============================================================================

/// 对象 ID（全局唯一）
pub type ObjectId = [u8; 32];

/// 对象版本号
pub type Version = u64;

/// 地址类型
pub type Address = [u8; 32];

/// 对象所有权类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnershipType {
    /// 独占对象（单一所有者）
    /// - 快速路径：无需共识
    /// - 性能：200K+ TPS
    /// - 延迟：< 1ms
    Owned(Address),
    
    /// 共享对象（多方可访问）
    /// - 共识路径：需要 MVCC + BFT
    /// - 性能：10-20K TPS
    /// - 延迟：2-5s
    Shared,
    
    /// 不可变对象（只读）
    /// - 超快路径：无锁读取
    /// - 性能：无限 TPS
    /// - 延迟：< 0.1ms
    Immutable,
}

/// 对象元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    /// 对象 ID
    pub id: ObjectId,
    
    /// 对象版本
    pub version: Version,
    
    /// 所有权类型
    pub ownership: OwnershipType,
    
    /// 对象类型（智能合约类型）
    pub object_type: String,
    
    /// 创建时间戳
    pub created_at: u64,
    
    /// 最后修改时间戳
    pub updated_at: u64,
    
    /// 对象大小（字节）
    pub size: usize,
    
    /// 是否已删除
    pub is_deleted: bool,
}

/// 对象完整数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    /// 元数据
    pub metadata: ObjectMetadata,
    
    /// 对象数据（序列化后的二进制）
    pub data: Vec<u8>,
}

/// 对象访问权限
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessType {
    /// 读取访问
    Read,
    
    /// 写入访问（需要所有权）
    Write,
    
    /// 转移所有权
    Transfer,
    
    /// 删除对象
    Delete,
}

// ============================================================================
// 对象所有权管理器
// ============================================================================

/// 对象所有权管理器
/// 
/// 职责：
/// 1. 注册和查询对象所有权
/// 2. 验证操作权限
/// 3. 转移对象所有权
/// 4. 路由交易到快速路径或共识路径
pub struct OwnershipManager {
    /// 对象元数据索引：object_id -> ObjectMetadata
    objects: Arc<RwLock<HashMap<ObjectId, ObjectMetadata>>>,
    
    /// 地址拥有的对象索引：address -> Set<object_id>
    owned_by_address: Arc<RwLock<HashMap<Address, HashSet<ObjectId>>>>,
    
    /// 共享对象集合
    shared_objects: Arc<RwLock<HashSet<ObjectId>>>,
    
    /// 不可变对象集合
    immutable_objects: Arc<RwLock<HashSet<ObjectId>>>,
    
    /// 版本历史：object_id -> Vec<Version>
    version_history: Arc<RwLock<HashMap<ObjectId, Vec<Version>>>>,
    
    /// 统计信息
    stats: Arc<RwLock<OwnershipStats>>,
}

/// 统计信息
#[derive(Debug, Default)]
pub struct OwnershipStats {
    /// 独占对象数量
    pub owned_count: u64,
    
    /// 共享对象数量
    pub shared_count: u64,
    
    /// 不可变对象数量
    pub immutable_count: u64,
    
    /// 快速路径交易数
    pub fast_path_txs: u64,
    
    /// 共识路径交易数
    pub consensus_path_txs: u64,
    
    /// 所有权转移次数
    pub transfer_count: u64,
}

impl OwnershipManager {
    /// 创建新的所有权管理器
    pub fn new() -> Self {
        Self {
            objects: Arc::new(RwLock::new(HashMap::new())),
            owned_by_address: Arc::new(RwLock::new(HashMap::new())),
            shared_objects: Arc::new(RwLock::new(HashSet::new())),
            immutable_objects: Arc::new(RwLock::new(HashSet::new())),
            version_history: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(OwnershipStats::default())),
        }
    }
    
    // ========================================================================
    // 对象注册
    // ========================================================================
    
    /// 注册新对象
    pub fn register_object(&self, mut metadata: ObjectMetadata) -> Result<(), String> {
        let object_id = metadata.id;
        
        // 检查对象是否已存在
        {
            let objects = self.objects.read().unwrap();
            if objects.contains_key(&object_id) {
                return Err(format!("Object {:?} already exists", object_id));
            }
        }
        
        // 设置初始版本
        metadata.version = 1;
        metadata.created_at = Self::current_timestamp();
        metadata.updated_at = metadata.created_at;
        
        // 注册到对应的索引
        match &metadata.ownership {
            OwnershipType::Owned(owner) => {
                // 独占对象
                self.owned_by_address
                    .write()
                    .unwrap()
                    .entry(*owner)
                    .or_default()
                    .insert(object_id);
                
                self.stats.write().unwrap().owned_count += 1;
            }
            OwnershipType::Shared => {
                // 共享对象
                self.shared_objects.write().unwrap().insert(object_id);
                self.stats.write().unwrap().shared_count += 1;
            }
            OwnershipType::Immutable => {
                // 不可变对象
                self.immutable_objects.write().unwrap().insert(object_id);
                self.stats.write().unwrap().immutable_count += 1;
            }
        }
        
        // 保存元数据
        self.objects.write().unwrap().insert(object_id, metadata.clone());
        
        // 记录版本历史
        self.version_history
            .write()
            .unwrap()
            .insert(object_id, vec![1]);
        
        Ok(())
    }
    
    // ========================================================================
    // 对象查询
    // ========================================================================
    
    /// 获取对象元数据
    pub fn get_metadata(&self, object_id: &ObjectId) -> Option<ObjectMetadata> {
        self.objects.read().unwrap().get(object_id).cloned()
    }
    
    /// 获取对象所有权类型
    pub fn get_ownership_type(&self, object_id: &ObjectId) -> Option<OwnershipType> {
        self.objects
            .read()
            .unwrap()
            .get(object_id)
            .map(|m| m.ownership.clone())
    }
    
    /// 检查地址是否拥有对象
    pub fn is_owned_by(&self, object_id: &ObjectId, address: &Address) -> bool {
        self.owned_by_address
            .read()
            .unwrap()
            .get(address)
            .map(|set| set.contains(object_id))
            .unwrap_or(false)
    }
    
    /// 获取地址拥有的所有对象
    pub fn get_owned_objects(&self, address: &Address) -> Vec<ObjectId> {
        self.owned_by_address
            .read()
            .unwrap()
            .get(address)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }
    
    /// 检查是否为共享对象
    pub fn is_shared(&self, object_id: &ObjectId) -> bool {
        self.shared_objects.read().unwrap().contains(object_id)
    }
    
    /// 检查是否为不可变对象
    pub fn is_immutable(&self, object_id: &ObjectId) -> bool {
        self.immutable_objects.read().unwrap().contains(object_id)
    }
    
    // ========================================================================
    // 权限验证
    // ========================================================================
    
    /// 验证访问权限
    pub fn verify_access(
        &self,
        object_id: &ObjectId,
        address: &Address,
        access_type: AccessType,
    ) -> Result<(), String> {
        let metadata = self.get_metadata(object_id)
            .ok_or_else(|| format!("Object {:?} not found", object_id))?;
        
        // 检查对象是否已删除
        if metadata.is_deleted {
            return Err(format!("Object {:?} has been deleted", object_id));
        }
        
        match metadata.ownership {
            OwnershipType::Owned(owner) => {
                // 独占对象：只有所有者可以写入/转移/删除
                match access_type {
                    AccessType::Read => Ok(()), // 任何人都可以读取
                    AccessType::Write | AccessType::Transfer | AccessType::Delete => {
                        if owner == *address {
                            Ok(())
                        } else {
                            Err(format!(
                                "Access denied: {:?} is not owned by {:?}",
                                object_id, address
                            ))
                        }
                    }
                }
            }
            OwnershipType::Shared => {
                // 共享对象：任何人都可以读写（通过共识）
                match access_type {
                    AccessType::Read | AccessType::Write => Ok(()),
                    AccessType::Transfer => {
                        Err("Cannot transfer shared object".to_string())
                    }
                    AccessType::Delete => {
                        Err("Cannot delete shared object directly".to_string())
                    }
                }
            }
            OwnershipType::Immutable => {
                // 不可变对象：只能读取
                match access_type {
                    AccessType::Read => Ok(()),
                    _ => Err(format!("Object {:?} is immutable", object_id)),
                }
            }
        }
    }
    
    // ========================================================================
    // 所有权转移
    // ========================================================================
    
    /// 转移对象所有权
    pub fn transfer_ownership(
        &self,
        object_id: &ObjectId,
        from: &Address,
        to: &Address,
    ) -> Result<(), String> {
        // 验证权限
        self.verify_access(object_id, from, AccessType::Transfer)?;
        
        // 获取当前元数据
        let mut metadata = self.get_metadata(object_id)
            .ok_or_else(|| format!("Object {:?} not found", object_id))?;
        
        // 检查当前是否为独占对象
        if let OwnershipType::Owned(owner) = metadata.ownership {
            if owner != *from {
                return Err(format!(
                    "Transfer denied: {:?} is not owned by {:?}",
                    object_id, from
                ));
            }
        } else {
            return Err(format!("Cannot transfer non-owned object {:?}", object_id));
        }
        
        // 从旧所有者移除
        self.owned_by_address
            .write()
            .unwrap()
            .get_mut(from)
            .map(|set| set.remove(object_id));
        
        // 添加到新所有者
        self.owned_by_address
            .write()
            .unwrap()
            .entry(*to)
            .or_default()
            .insert(*object_id);
        
        // 更新元数据
        metadata.ownership = OwnershipType::Owned(*to);
        metadata.version += 1;
        metadata.updated_at = Self::current_timestamp();
        
        self.objects.write().unwrap().insert(*object_id, metadata.clone());
        
        // 记录版本历史
        if let Some(versions) = self.version_history
            .write()
            .unwrap()
            .get_mut(object_id) { versions.push(metadata.version) }
        
        // 更新统计
        self.stats.write().unwrap().transfer_count += 1;
        
        Ok(())
    }
    
    /// 转换为共享对象
    pub fn make_shared(&self, object_id: &ObjectId, owner: &Address) -> Result<(), String> {
        // 验证权限
        self.verify_access(object_id, owner, AccessType::Write)?;
        
        // 获取当前元数据
        let mut metadata = self.get_metadata(object_id)
            .ok_or_else(|| format!("Object {:?} not found", object_id))?;
        
        // 检查当前是否为独占对象
        if let OwnershipType::Owned(current_owner) = metadata.ownership {
            if current_owner != *owner {
                return Err(format!(
                    "Make shared denied: {:?} is not owned by {:?}",
                    object_id, owner
                ));
            }
        } else {
            return Err(format!("Object {:?} is not an owned object", object_id));
        }
        
        // 从独占对象索引移除
        self.owned_by_address
            .write()
            .unwrap()
            .get_mut(owner)
            .map(|set| set.remove(object_id));
        
        // 添加到共享对象集合
        self.shared_objects.write().unwrap().insert(*object_id);
        
        // 更新元数据
        metadata.ownership = OwnershipType::Shared;
        metadata.version += 1;
        metadata.updated_at = Self::current_timestamp();
        
        self.objects.write().unwrap().insert(*object_id, metadata.clone());
        
        // 更新统计
        let mut stats = self.stats.write().unwrap();
        stats.owned_count -= 1;
        stats.shared_count += 1;
        
        Ok(())
    }
    
    /// 冻结为不可变对象
    pub fn make_immutable(&self, object_id: &ObjectId, owner: &Address) -> Result<(), String> {
        // 验证权限
        self.verify_access(object_id, owner, AccessType::Write)?;
        
        // 获取当前元数据
        let mut metadata = self.get_metadata(object_id)
            .ok_or_else(|| format!("Object {:?} not found", object_id))?;
        
        // 检查当前是否为独占对象
        if let OwnershipType::Owned(current_owner) = metadata.ownership {
            if current_owner != *owner {
                return Err(format!(
                    "Make immutable denied: {:?} is not owned by {:?}",
                    object_id, owner
                ));
            }
        } else {
            return Err("Can only freeze owned objects".to_string());
        }
        
        // 从独占对象索引移除
        self.owned_by_address
            .write()
            .unwrap()
            .get_mut(owner)
            .map(|set| set.remove(object_id));
        
        // 添加到不可变对象集合
        self.immutable_objects.write().unwrap().insert(*object_id);
        
        // 更新元数据
        metadata.ownership = OwnershipType::Immutable;
        metadata.version += 1;
        metadata.updated_at = Self::current_timestamp();
        
        self.objects.write().unwrap().insert(*object_id, metadata.clone());
        
        // 更新统计
        let mut stats = self.stats.write().unwrap();
        stats.owned_count -= 1;
        stats.immutable_count += 1;
        
        Ok(())
    }
    
    // ========================================================================
    // 路径路由
    // ========================================================================
    
    /// 判断交易应该走哪个路径
    /// 
    /// 返回：
    /// - true: 快速路径（独占对象或不可变对象）
    /// - false: 共识路径（共享对象）
    pub fn should_use_fast_path(&self, object_ids: &[ObjectId]) -> bool {
        for object_id in object_ids {
            if self.is_shared(object_id) {
                // 任何一个共享对象都需要走共识路径
                return false;
            }
        }
        
        // 全是独占对象或不可变对象，走快速路径
        true
    }
    
    /// 记录交易路径统计
    pub fn record_transaction_path(&self, is_fast_path: bool) {
        let mut stats = self.stats.write().unwrap();
        if is_fast_path {
            stats.fast_path_txs += 1;
        } else {
            stats.consensus_path_txs += 1;
        }
    }
    
    // ========================================================================
    // 对象更新
    // ========================================================================
    
    /// 更新对象版本（在对象修改后调用）
    pub fn update_version(&self, object_id: &ObjectId) -> Result<Version, String> {
        let mut objects = self.objects.write().unwrap();
        let metadata = objects
            .get_mut(object_id)
            .ok_or_else(|| format!("Object {:?} not found", object_id))?;
        
        metadata.version += 1;
        metadata.updated_at = Self::current_timestamp();
        let new_version = metadata.version;
        
        // 记录版本历史
        if let Some(versions) = self.version_history
            .write()
            .unwrap()
            .get_mut(object_id) { versions.push(new_version) }
        
        Ok(new_version)
    }
    
    /// 删除对象（软删除）
    pub fn delete_object(&self, object_id: &ObjectId, address: &Address) -> Result<(), String> {
        // 验证权限
        self.verify_access(object_id, address, AccessType::Delete)?;
        
        let mut objects = self.objects.write().unwrap();
        let metadata = objects
            .get_mut(object_id)
            .ok_or_else(|| format!("Object {:?} not found", object_id))?;
        
        metadata.is_deleted = true;
        metadata.updated_at = Self::current_timestamp();
        
        // 从索引中移除（但保留元数据用于审计）
        if let OwnershipType::Owned(owner) = metadata.ownership {
            self.owned_by_address
                .write()
                .unwrap()
                .get_mut(&owner)
                .map(|set| set.remove(object_id));
        }
        
        Ok(())
    }
    
    // ========================================================================
    // 统计信息
    // ========================================================================
    
    /// 获取统计信息
    pub fn get_stats(&self) -> OwnershipStats {
        self.stats.read().unwrap().clone()
    }
    
    /// 获取快速路径占比
    pub fn get_fast_path_ratio(&self) -> f64 {
        let stats = self.stats.read().unwrap();
        let total = stats.fast_path_txs + stats.consensus_path_txs;
        if total == 0 {
            return 0.0;
        }
        stats.fast_path_txs as f64 / total as f64
    }
    
    // ========================================================================
    // 工具函数
    // ========================================================================
    
    /// 获取当前时间戳（Unix 毫秒）
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

impl Default for OwnershipManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for OwnershipStats {
    fn clone(&self) -> Self {
        Self {
            owned_count: self.owned_count,
            shared_count: self.shared_count,
            immutable_count: self.immutable_count,
            fast_path_txs: self.fast_path_txs,
            consensus_path_txs: self.consensus_path_txs,
            transfer_count: self.transfer_count,
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_object(id: u8, _owner: Address, ownership: OwnershipType) -> ObjectMetadata {
        let mut object_id = [0u8; 32];
        object_id[0] = id;
        
        ObjectMetadata {
            id: object_id,
            version: 0,
            ownership,
            object_type: "TestObject".to_string(),
            created_at: 0,
            updated_at: 0,
            size: 100,
            is_deleted: false,
        }
    }
    
    #[test]
    fn test_register_owned_object() {
        let manager = OwnershipManager::new();
        let owner = [1u8; 32];
        let metadata = create_test_object(1, owner, OwnershipType::Owned(owner));
        
        assert!(manager.register_object(metadata.clone()).is_ok());
        
        // 验证注册成功
        assert_eq!(
            manager.get_ownership_type(&metadata.id),
            Some(OwnershipType::Owned(owner))
        );
        assert!(manager.is_owned_by(&metadata.id, &owner));
        
        // 统计信息
        let stats = manager.get_stats();
        assert_eq!(stats.owned_count, 1);
    }
    
    #[test]
    fn test_register_shared_object() {
        let manager = OwnershipManager::new();
        let metadata = create_test_object(2, [0u8; 32], OwnershipType::Shared);
        
        assert!(manager.register_object(metadata.clone()).is_ok());
        
        // 验证注册成功
        assert_eq!(
            manager.get_ownership_type(&metadata.id),
            Some(OwnershipType::Shared)
        );
        assert!(manager.is_shared(&metadata.id));
        
        // 统计信息
        let stats = manager.get_stats();
        assert_eq!(stats.shared_count, 1);
    }
    
    #[test]
    fn test_transfer_ownership() {
        let manager = OwnershipManager::new();
        let owner1 = [1u8; 32];
        let owner2 = [2u8; 32];
        let metadata = create_test_object(3, owner1, OwnershipType::Owned(owner1));
        
        manager.register_object(metadata.clone()).unwrap();
        
        // 转移所有权
        assert!(manager.transfer_ownership(&metadata.id, &owner1, &owner2).is_ok());
        
        // 验证转移成功
        assert!(!manager.is_owned_by(&metadata.id, &owner1));
        assert!(manager.is_owned_by(&metadata.id, &owner2));
        
        // 统计信息
        let stats = manager.get_stats();
        assert_eq!(stats.transfer_count, 1);
    }
    
    #[test]
    fn test_make_shared() {
        let manager = OwnershipManager::new();
        let owner = [1u8; 32];
        let metadata = create_test_object(4, owner, OwnershipType::Owned(owner));
        
        manager.register_object(metadata.clone()).unwrap();
        
        // 转换为共享对象
        assert!(manager.make_shared(&metadata.id, &owner).is_ok());
        
        // 验证转换成功
        assert!(manager.is_shared(&metadata.id));
        assert!(!manager.is_owned_by(&metadata.id, &owner));
        
        // 统计信息
        let stats = manager.get_stats();
        assert_eq!(stats.owned_count, 0);
        assert_eq!(stats.shared_count, 1);
    }
    
    #[test]
    fn test_make_immutable() {
        let manager = OwnershipManager::new();
        let owner = [1u8; 32];
        let metadata = create_test_object(5, owner, OwnershipType::Owned(owner));
        
        manager.register_object(metadata.clone()).unwrap();
        
        // 冻结为不可变对象
        assert!(manager.make_immutable(&metadata.id, &owner).is_ok());
        
        // 验证冻结成功
        assert!(manager.is_immutable(&metadata.id));
        
        // 统计信息
        let stats = manager.get_stats();
        assert_eq!(stats.owned_count, 0);
        assert_eq!(stats.immutable_count, 1);
    }
    
    #[test]
    fn test_fast_path_routing() {
        let manager = OwnershipManager::new();
        let owner = [1u8; 32];
        
        // 创建独占对象和共享对象
        let owned_obj = create_test_object(6, owner, OwnershipType::Owned(owner));
        let shared_obj = create_test_object(7, [0u8; 32], OwnershipType::Shared);
        
        manager.register_object(owned_obj.clone()).unwrap();
        manager.register_object(shared_obj.clone()).unwrap();
        
        // 只有独占对象：走快速路径
        assert!(manager.should_use_fast_path(&[owned_obj.id]));
        
        // 包含共享对象：走共识路径
        assert!(!manager.should_use_fast_path(&[owned_obj.id, shared_obj.id]));
    }
    
    #[test]
    fn test_access_control() {
        let manager = OwnershipManager::new();
        let owner = [1u8; 32];
        let other = [2u8; 32];
        let metadata = create_test_object(8, owner, OwnershipType::Owned(owner));
        
        manager.register_object(metadata.clone()).unwrap();
        
        // 所有者可以写入
        assert!(manager.verify_access(&metadata.id, &owner, AccessType::Write).is_ok());
        
        // 非所有者不能写入
        assert!(manager.verify_access(&metadata.id, &other, AccessType::Write).is_err());
        
        // 任何人都可以读取
        assert!(manager.verify_access(&metadata.id, &other, AccessType::Read).is_ok());
    }
}
