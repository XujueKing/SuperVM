"""
Test suite for SuperVM state management
"""

import unittest
from supervm.state import StateManager


class TestStateManager(unittest.TestCase):
    """Test state management functionality"""
    
    def test_create_account(self):
        """Test account creation"""
        state = StateManager()
        state.create_account("alice", 100.0)
        
        account = state.get_account("alice")
        self.assertIsNotNone(account)
        self.assertEqual(account['balance'], 100.0)
        self.assertEqual(account['nonce'], 0)
        self.assertFalse(account['is_contract'])
    
    def test_update_balance(self):
        """Test balance updates"""
        state = StateManager()
        state.create_account("alice", 100.0)
        
        state.update_balance("alice", 50.0)
        self.assertEqual(state.get_balance("alice"), 150.0)
        
        state.update_balance("alice", -30.0)
        self.assertEqual(state.get_balance("alice"), 120.0)
    
    def test_nonce_management(self):
        """Test nonce increment"""
        state = StateManager()
        state.create_account("alice")
        
        self.assertEqual(state.get_nonce("alice"), 0)
        
        state.increment_nonce("alice")
        self.assertEqual(state.get_nonce("alice"), 1)
        
        state.increment_nonce("alice")
        self.assertEqual(state.get_nonce("alice"), 2)
    
    def test_contract_deployment(self):
        """Test smart contract deployment"""
        state = StateManager()
        state.deploy_contract("contract1", "deadbeef")
        
        self.assertTrue(state.is_contract("contract1"))
        self.assertEqual(state.get_contract_code("contract1"), "deadbeef")
    
    def test_storage_operations(self):
        """Test contract storage"""
        state = StateManager()
        state.deploy_contract("contract1", "code")
        
        state.set_storage("contract1", "key1", "value1")
        state.set_storage("contract1", "key2", 42)
        
        self.assertEqual(state.get_storage("contract1", "key1"), "value1")
        self.assertEqual(state.get_storage("contract1", "key2"), 42)
        self.assertIsNone(state.get_storage("contract1", "key3"))
    
    def test_state_snapshot(self):
        """Test state save/load"""
        state = StateManager()
        state.create_account("alice", 100.0)
        state.deploy_contract("contract1", "code")
        state.set_storage("contract1", "key", "value")
        
        # Get snapshot
        snapshot = state.get_state()
        
        # Create new state and load
        state2 = StateManager()
        state2.load_state(snapshot)
        
        self.assertEqual(state2.get_balance("alice"), 100.0)
        self.assertEqual(state2.get_contract_code("contract1"), "code")
        self.assertEqual(state2.get_storage("contract1", "key"), "value")


if __name__ == '__main__':
    unittest.main()
