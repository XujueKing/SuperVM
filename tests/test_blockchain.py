"""
Test suite for SuperVM blockchain
"""

import unittest
from supervm.blockchain import Transaction, Block, Blockchain


class TestTransaction(unittest.TestCase):
    """Test transaction functionality"""
    
    def test_create_transaction(self):
        """Test transaction creation"""
        tx = Transaction("alice", "bob", 50.0)
        
        self.assertEqual(tx.sender, "alice")
        self.assertEqual(tx.recipient, "bob")
        self.assertEqual(tx.amount, 50.0)
        self.assertIsNotNone(tx.timestamp)
    
    def test_transaction_to_dict(self):
        """Test transaction serialization"""
        tx = Transaction("alice", "bob", 50.0, {"memo": "payment"})
        tx_dict = tx.to_dict()
        
        self.assertIsInstance(tx_dict, dict)
        self.assertEqual(tx_dict['sender'], "alice")
        self.assertEqual(tx_dict['recipient'], "bob")
        self.assertEqual(tx_dict['amount'], 50.0)
        self.assertEqual(tx_dict['data']['memo'], "payment")


class TestBlock(unittest.TestCase):
    """Test block functionality"""
    
    def test_create_block(self):
        """Test block creation"""
        tx = Transaction("alice", "bob", 50.0)
        block = Block(1, [tx], "0" * 64)
        
        self.assertEqual(block.index, 1)
        self.assertEqual(len(block.transactions), 1)
        self.assertEqual(block.previous_hash, "0" * 64)
        self.assertIsNotNone(block.hash)
    
    def test_mine_block(self):
        """Test block mining"""
        tx = Transaction("alice", "bob", 50.0)
        block = Block(1, [tx], "0" * 64)
        
        difficulty = 2
        block.mine_block(difficulty)
        
        # Hash should start with required zeros
        self.assertTrue(block.hash.startswith('0' * difficulty))
        self.assertGreater(block.nonce, 0)
    
    def test_calculate_hash(self):
        """Test hash calculation"""
        tx = Transaction("alice", "bob", 50.0)
        block = Block(1, [tx], "0" * 64)
        
        hash1 = block.calculate_hash()
        hash2 = block.calculate_hash()
        
        # Same block should produce same hash
        self.assertEqual(hash1, hash2)


class TestBlockchain(unittest.TestCase):
    """Test blockchain functionality"""
    
    def test_create_blockchain(self):
        """Test blockchain creation"""
        bc = Blockchain(difficulty=2)
        
        self.assertEqual(len(bc.chain), 1)  # Genesis block
        self.assertEqual(bc.chain[0].index, 0)
        self.assertEqual(bc.chain[0].previous_hash, "0")
    
    def test_add_transaction(self):
        """Test adding transactions"""
        bc = Blockchain(difficulty=2)
        tx = Transaction("alice", "bob", 50.0)
        
        bc.add_transaction(tx)
        
        self.assertEqual(len(bc.pending_transactions), 1)
        self.assertEqual(bc.pending_transactions[0].sender, "alice")
    
    def test_mine_block(self):
        """Test mining blocks"""
        bc = Blockchain(difficulty=2)
        tx = Transaction("alice", "bob", 50.0)
        bc.add_transaction(tx)
        
        initial_length = len(bc.chain)
        bc.mine_pending_transactions("miner")
        
        self.assertEqual(len(bc.chain), initial_length + 1)
        self.assertEqual(len(bc.pending_transactions), 0)
    
    def test_get_balance(self):
        """Test balance calculation"""
        bc = Blockchain(difficulty=2)
        
        # Mine a block to give miner rewards
        bc.mine_pending_transactions("alice")
        
        balance = bc.get_balance("alice")
        self.assertEqual(balance, bc.mining_reward)
    
    def test_chain_validation(self):
        """Test blockchain validation"""
        bc = Blockchain(difficulty=2)
        tx = Transaction("alice", "bob", 50.0)
        bc.add_transaction(tx)
        bc.mine_pending_transactions("miner")
        
        # Valid chain
        self.assertTrue(bc.is_chain_valid())
        
        # Tamper with chain
        bc.chain[1].transactions[0].amount = 1000.0
        
        # Chain should be invalid now
        self.assertFalse(bc.is_chain_valid())


if __name__ == '__main__':
    unittest.main()
