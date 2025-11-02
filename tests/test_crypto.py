"""
Test suite for SuperVM crypto utilities
"""

import unittest
from supervm.crypto import CryptoUtils


class TestCryptoUtils(unittest.TestCase):
    """Test cryptographic utilities"""
    
    def test_hash_data(self):
        """Test data hashing"""
        data1 = "test data"
        data2 = {"key": "value"}
        
        hash1 = CryptoUtils.hash_data(data1)
        hash2 = CryptoUtils.hash_data(data2)
        
        self.assertEqual(len(hash1), 64)  # SHA-256 produces 64 hex chars
        self.assertEqual(len(hash2), 64)
        self.assertNotEqual(hash1, hash2)
        
        # Same data should produce same hash
        hash3 = CryptoUtils.hash_data(data1)
        self.assertEqual(hash1, hash3)
    
    def test_generate_keypair(self):
        """Test keypair generation"""
        private_key, public_key = CryptoUtils.generate_keypair()
        
        self.assertIsInstance(private_key, str)
        self.assertIsInstance(public_key, str)
        self.assertGreater(len(private_key), 0)
        self.assertGreater(len(public_key), 0)
        
        # Different calls should generate different keys
        private_key2, public_key2 = CryptoUtils.generate_keypair()
        self.assertNotEqual(private_key, private_key2)
        self.assertNotEqual(public_key, public_key2)
    
    def test_sign_and_verify(self):
        """Test signing and verification"""
        private_key, public_key = CryptoUtils.generate_keypair()
        data = "test message"
        
        signature = CryptoUtils.sign_data(data, private_key)
        self.assertIsInstance(signature, str)
        self.assertGreater(len(signature), 0)
        
        # Valid signature should verify
        is_valid = CryptoUtils.verify_signature(data, signature, public_key)
        self.assertTrue(is_valid)
        
        # Invalid signature should not verify
        is_valid = CryptoUtils.verify_signature("wrong data", signature, public_key)
        self.assertFalse(is_valid)
    
    def test_derive_address(self):
        """Test address derivation"""
        _, public_key = CryptoUtils.generate_keypair()
        address = CryptoUtils.derive_address(public_key)
        
        self.assertIsInstance(address, str)
        self.assertEqual(len(address), 40)
        
        # Same public key should produce same address
        address2 = CryptoUtils.derive_address(public_key)
        self.assertEqual(address, address2)


if __name__ == '__main__':
    unittest.main()
