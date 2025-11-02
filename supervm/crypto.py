"""
Cryptographic utilities for SuperVM
Provides hashing, signing, and verification functions
"""

import hashlib
import json
from typing import Any, Dict
from ecdsa import SigningKey, VerifyingKey, SECP256k1


class CryptoUtils:
    """Cryptographic utility functions for blockchain operations"""
    
    @staticmethod
    def hash_data(data: Any) -> str:
        """
        Create SHA-256 hash of data
        
        Args:
            data: Data to hash (will be JSON serialized if not string)
            
        Returns:
            Hexadecimal hash string
        """
        if not isinstance(data, str):
            data = json.dumps(data, sort_keys=True)
        return hashlib.sha256(data.encode()).hexdigest()
    
    @staticmethod
    def hash_block(index: int, timestamp: float, transactions: list, 
                   previous_hash: str, nonce: int = 0) -> str:
        """
        Create hash of a block
        
        Args:
            index: Block index
            timestamp: Block timestamp
            transactions: List of transactions
            previous_hash: Hash of previous block
            nonce: Proof of work nonce
            
        Returns:
            Hexadecimal hash string
        """
        block_data = {
            'index': index,
            'timestamp': timestamp,
            'transactions': transactions,
            'previous_hash': previous_hash,
            'nonce': nonce
        }
        return CryptoUtils.hash_data(block_data)
    
    @staticmethod
    def generate_keypair() -> tuple:
        """
        Generate a new ECDSA keypair
        
        Returns:
            Tuple of (private_key, public_key) as hex strings
        """
        private_key = SigningKey.generate(curve=SECP256k1)
        public_key = private_key.get_verifying_key()
        return (
            private_key.to_string().hex(),
            public_key.to_string().hex()
        )
    
    @staticmethod
    def sign_data(data: str, private_key_hex: str) -> str:
        """
        Sign data with private key
        
        Args:
            data: Data to sign
            private_key_hex: Private key in hex format
            
        Returns:
            Signature as hex string
        """
        private_key = SigningKey.from_string(
            bytes.fromhex(private_key_hex),
            curve=SECP256k1
        )
        signature = private_key.sign(data.encode())
        return signature.hex()
    
    @staticmethod
    def verify_signature(data: str, signature_hex: str, public_key_hex: str) -> bool:
        """
        Verify signature with public key
        
        Args:
            data: Original data
            signature_hex: Signature in hex format
            public_key_hex: Public key in hex format
            
        Returns:
            True if signature is valid, False otherwise
        """
        try:
            public_key = VerifyingKey.from_string(
                bytes.fromhex(public_key_hex),
                curve=SECP256k1
            )
            public_key.verify(bytes.fromhex(signature_hex), data.encode())
            return True
        except Exception:
            return False
    
    @staticmethod
    def derive_address(public_key_hex: str) -> str:
        """
        Derive blockchain address from public key
        
        Args:
            public_key_hex: Public key in hex format
            
        Returns:
            Address as hex string
        """
        # Simple address derivation: hash of public key
        return hashlib.sha256(public_key_hex.encode()).hexdigest()[:40]
