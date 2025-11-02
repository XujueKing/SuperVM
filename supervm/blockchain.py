"""
Blockchain data structures and implementation
"""

import time
from typing import List, Dict, Any, Optional
from .crypto import CryptoUtils


class Transaction:
    """Represents a transaction in the blockchain"""
    
    def __init__(self, sender: str, recipient: str, amount: float, 
                 data: Optional[Dict[str, Any]] = None, signature: Optional[str] = None):
        """
        Initialize a transaction
        
        Args:
            sender: Sender address
            recipient: Recipient address
            amount: Transaction amount
            data: Optional transaction data (for smart contracts)
            signature: Optional transaction signature
        """
        self.sender = sender
        self.recipient = recipient
        self.amount = amount
        self.data = data or {}
        self.signature = signature
        self.timestamp = time.time()
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert transaction to dictionary"""
        return {
            'sender': self.sender,
            'recipient': self.recipient,
            'amount': self.amount,
            'data': self.data,
            'timestamp': self.timestamp,
            'signature': self.signature
        }
    
    def sign(self, private_key: str):
        """Sign the transaction with private key"""
        tx_data = f"{self.sender}{self.recipient}{self.amount}{self.timestamp}"
        self.signature = CryptoUtils.sign_data(tx_data, private_key)
    
    def verify(self, public_key: str) -> bool:
        """Verify transaction signature"""
        if not self.signature:
            return False
        tx_data = f"{self.sender}{self.recipient}{self.amount}{self.timestamp}"
        return CryptoUtils.verify_signature(tx_data, self.signature, public_key)


class Block:
    """Represents a block in the blockchain"""
    
    def __init__(self, index: int, transactions: List[Transaction], 
                 previous_hash: str, timestamp: Optional[float] = None):
        """
        Initialize a block
        
        Args:
            index: Block index
            transactions: List of transactions
            previous_hash: Hash of previous block
            timestamp: Optional timestamp (defaults to current time)
        """
        self.index = index
        self.timestamp = timestamp or time.time()
        self.transactions = transactions
        self.previous_hash = previous_hash
        self.nonce = 0
        self.hash = self.calculate_hash()
    
    def calculate_hash(self) -> str:
        """Calculate block hash"""
        tx_data = [tx.to_dict() for tx in self.transactions]
        return CryptoUtils.hash_block(
            self.index,
            self.timestamp,
            tx_data,
            self.previous_hash,
            self.nonce
        )
    
    def mine_block(self, difficulty: int):
        """
        Mine block using Proof of Work
        
        Args:
            difficulty: Mining difficulty (number of leading zeros)
        """
        target = '0' * difficulty
        while self.hash[:difficulty] != target:
            self.nonce += 1
            self.hash = self.calculate_hash()
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert block to dictionary"""
        return {
            'index': self.index,
            'timestamp': self.timestamp,
            'transactions': [tx.to_dict() for tx in self.transactions],
            'previous_hash': self.previous_hash,
            'nonce': self.nonce,
            'hash': self.hash
        }


class Blockchain:
    """Main blockchain implementation"""
    
    def __init__(self, difficulty: int = 2):
        """
        Initialize blockchain
        
        Args:
            difficulty: Mining difficulty
        """
        self.chain: List[Block] = []
        self.difficulty = difficulty
        self.pending_transactions: List[Transaction] = []
        self.mining_reward = 100
        self.create_genesis_block()
    
    def create_genesis_block(self):
        """Create the first block in the chain"""
        genesis_block = Block(0, [], "0")
        genesis_block.mine_block(self.difficulty)
        self.chain.append(genesis_block)
    
    def get_latest_block(self) -> Block:
        """Get the most recent block"""
        return self.chain[-1]
    
    def add_transaction(self, transaction: Transaction):
        """Add a transaction to pending transactions"""
        self.pending_transactions.append(transaction)
    
    def mine_pending_transactions(self, miner_address: str):
        """
        Mine pending transactions and create new block
        
        Args:
            miner_address: Address to receive mining reward
        """
        # Create mining reward transaction
        reward_tx = Transaction("SYSTEM", miner_address, self.mining_reward)
        self.pending_transactions.append(reward_tx)
        
        # Create new block
        block = Block(
            len(self.chain),
            self.pending_transactions,
            self.get_latest_block().hash
        )
        block.mine_block(self.difficulty)
        
        # Add block to chain
        self.chain.append(block)
        
        # Reset pending transactions
        self.pending_transactions = []
    
    def get_balance(self, address: str) -> float:
        """
        Get balance for an address
        
        Args:
            address: Address to check
            
        Returns:
            Current balance
        """
        balance = 0.0
        
        for block in self.chain:
            for tx in block.transactions:
                if tx.sender == address:
                    balance -= tx.amount
                if tx.recipient == address:
                    balance += tx.amount
        
        return balance
    
    def is_chain_valid(self) -> bool:
        """
        Validate the entire blockchain
        
        Returns:
            True if chain is valid, False otherwise
        """
        for i in range(1, len(self.chain)):
            current_block = self.chain[i]
            previous_block = self.chain[i - 1]
            
            # Check if hash is correct
            if current_block.hash != current_block.calculate_hash():
                return False
            
            # Check if previous hash matches
            if current_block.previous_hash != previous_block.hash:
                return False
            
            # Check proof of work
            if not current_block.hash.startswith('0' * self.difficulty):
                return False
        
        return True
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert blockchain to dictionary"""
        return {
            'chain': [block.to_dict() for block in self.chain],
            'difficulty': self.difficulty,
            'pending_transactions': [tx.to_dict() for tx in self.pending_transactions]
        }
