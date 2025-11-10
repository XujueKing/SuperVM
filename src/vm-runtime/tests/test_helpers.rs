// SPDX-License-Identifier: GPL-3.0-or-later
// Test helpers for SuperVM routing & ownership setup
use vm_runtime::{OwnershipManager, OwnershipType, ObjectMetadata};
use vm_runtime::supervm::{SuperVM, Transaction, Privacy};
use vm_runtime::parallel_mvcc::MvccScheduler;
use vm_runtime::mvcc::Txn;

pub type Address = [u8;32];
pub type ObjectId = [u8;32];

pub fn addr(id: u8) -> Address { let mut a = [0u8;32]; a[0]=id; a }
pub fn obj(id: u8) -> ObjectId { let mut o = [0u8;32]; o[0]=id; o }

pub struct VmTestContext {
    pub ownership: OwnershipManager,
    pub scheduler: MvccScheduler,
}

impl VmTestContext {
    pub fn new() -> Self {
        Self { ownership: OwnershipManager::new(), scheduler: MvccScheduler::new() }
    }

    pub fn register_owned(&self, object_id: ObjectId, owner: Address) {
        let meta = ObjectMetadata {
            id: object_id,
            version: 0,
            ownership: OwnershipType::Owned(owner),
            object_type: "Test".into(),
            created_at: 0,
            updated_at: 0,
            size: 0,
            is_deleted: false,
        };
        self.ownership.register_object(meta).expect("register owned object")
    }

    pub fn register_shared(&self, object_id: ObjectId) {
        let meta = ObjectMetadata {
            id: object_id,
            version: 0,
            ownership: OwnershipType::Shared,
            object_type: "Test".into(),
            created_at: 0,
            updated_at: 0,
            size: 0,
            is_deleted: false,
        };
        self.ownership.register_object(meta).expect("register shared object")
    }

    pub fn register_immutable(&self, object_id: ObjectId) {
        let meta = ObjectMetadata {
            id: object_id,
            version: 0,
            ownership: OwnershipType::Immutable,
            object_type: "Test".into(),
            created_at: 0,
            updated_at: 0,
            size: 0,
            is_deleted: false,
        };
        self.ownership.register_object(meta).expect("register immutable object")
    }

    pub fn build_vm(&self) -> SuperVM<'_> {
        SuperVM::new(&self.ownership).with_scheduler(&self.scheduler)
    }
}

pub fn tx(from: Address, objects: Vec<ObjectId>, privacy: Privacy) -> Transaction {
    Transaction { from, objects, privacy }
}

// Simple consensus op for smoke tests
pub fn write_key(txn: &mut Txn, key: &[u8], val: &[u8]) -> anyhow::Result<i32> {
    txn.write(key.to_vec(), val.to_vec());
    Ok(val.len() as i32)
}
