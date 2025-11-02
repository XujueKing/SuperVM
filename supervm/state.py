"""
State management for the virtual machine
Handles account states, storage, and smart contract data
"""

from typing import Dict, Any, Optional
import json


class StateManager:
    """Manages the state of accounts and smart contracts"""
    
    def __init__(self):
        """Initialize state manager"""
        self.accounts: Dict[str, Dict[str, Any]] = {}
        self.storage: Dict[str, Dict[str, Any]] = {}
        self.contract_code: Dict[str, str] = {}
    
    def create_account(self, address: str, balance: float = 0.0):
        """
        Create a new account
        
        Args:
            address: Account address
            balance: Initial balance
        """
        if address not in self.accounts:
            self.accounts[address] = {
                'balance': balance,
                'nonce': 0,
                'is_contract': False
            }
    
    def get_account(self, address: str) -> Optional[Dict[str, Any]]:
        """
        Get account information
        
        Args:
            address: Account address
            
        Returns:
            Account data or None if not found
        """
        return self.accounts.get(address)
    
    def update_balance(self, address: str, amount: float):
        """
        Update account balance
        
        Args:
            address: Account address
            amount: Amount to add (negative to subtract)
        """
        if address not in self.accounts:
            self.create_account(address)
        self.accounts[address]['balance'] += amount
    
    def get_balance(self, address: str) -> float:
        """
        Get account balance
        
        Args:
            address: Account address
            
        Returns:
            Current balance
        """
        if address in self.accounts:
            return self.accounts[address]['balance']
        return 0.0
    
    def increment_nonce(self, address: str):
        """
        Increment account nonce
        
        Args:
            address: Account address
        """
        if address not in self.accounts:
            self.create_account(address)
        self.accounts[address]['nonce'] += 1
    
    def get_nonce(self, address: str) -> int:
        """
        Get account nonce
        
        Args:
            address: Account address
            
        Returns:
            Current nonce
        """
        if address in self.accounts:
            return self.accounts[address]['nonce']
        return 0
    
    def deploy_contract(self, address: str, code: str):
        """
        Deploy a smart contract
        
        Args:
            address: Contract address
            code: Contract bytecode
        """
        self.create_account(address)
        self.accounts[address]['is_contract'] = True
        self.contract_code[address] = code
        self.storage[address] = {}
    
    def get_contract_code(self, address: str) -> Optional[str]:
        """
        Get contract code
        
        Args:
            address: Contract address
            
        Returns:
            Contract code or None
        """
        return self.contract_code.get(address)
    
    def set_storage(self, address: str, key: str, value: Any):
        """
        Set contract storage value
        
        Args:
            address: Contract address
            key: Storage key
            value: Storage value
        """
        if address not in self.storage:
            self.storage[address] = {}
        self.storage[address][key] = value
    
    def get_storage(self, address: str, key: str) -> Optional[Any]:
        """
        Get contract storage value
        
        Args:
            address: Contract address
            key: Storage key
            
        Returns:
            Storage value or None
        """
        if address in self.storage:
            return self.storage[address].get(key)
        return None
    
    def is_contract(self, address: str) -> bool:
        """
        Check if address is a contract
        
        Args:
            address: Address to check
            
        Returns:
            True if address is a contract
        """
        if address in self.accounts:
            return self.accounts[address].get('is_contract', False)
        return False
    
    def get_state(self) -> Dict[str, Any]:
        """
        Get complete state snapshot
        
        Returns:
            State dictionary
        """
        return {
            'accounts': self.accounts,
            'storage': self.storage,
            'contract_code': self.contract_code
        }
    
    def load_state(self, state: Dict[str, Any]):
        """
        Load state from snapshot
        
        Args:
            state: State dictionary
        """
        self.accounts = state.get('accounts', {})
        self.storage = state.get('storage', {})
        self.contract_code = state.get('contract_code', {})
