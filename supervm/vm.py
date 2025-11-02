"""
Virtual Machine implementation for SuperVM
Executes bytecode and smart contracts
"""

from typing import Dict, Any, List, Optional
from .state import StateManager
from .blockchain import Transaction


class OpCode:
    """VM Operation Codes"""
    # Stack operations
    PUSH = 0x01
    POP = 0x02
    DUP = 0x03
    SWAP = 0x04
    
    # Arithmetic operations
    ADD = 0x10
    SUB = 0x11
    MUL = 0x12
    DIV = 0x13
    MOD = 0x14
    
    # Comparison operations
    EQ = 0x20
    LT = 0x21
    GT = 0x22
    
    # Logical operations
    AND = 0x30
    OR = 0x31
    NOT = 0x32
    
    # Memory operations
    MLOAD = 0x40
    MSTORE = 0x41
    
    # Storage operations
    SLOAD = 0x50
    SSTORE = 0x51
    
    # Flow control
    JUMP = 0x60
    JUMPI = 0x61
    JUMPDEST = 0x62
    
    # System operations
    CALL = 0x70
    RETURN = 0x71
    REVERT = 0x72
    STOP = 0x73
    
    # Blockchain operations
    BALANCE = 0x80
    TRANSFER = 0x81
    SENDER = 0x82
    VALUE = 0x83
    TIMESTAMP = 0x84
    BLOCKNUMBER = 0x85


class VirtualMachine:
    """
    SuperVM - Blockchain Virtual Machine
    Executes bytecode instructions and manages smart contract execution
    """
    
    def __init__(self, state_manager: Optional[StateManager] = None):
        """
        Initialize VM
        
        Args:
            state_manager: Optional state manager instance
        """
        self.state = state_manager or StateManager()
        self.stack: List[Any] = []
        self.memory: Dict[int, Any] = {}
        self.pc = 0  # Program counter
        self.gas = 0
        self.gas_limit = 1000000
        self.stopped = False
        
        # Execution context
        self.caller = ""
        self.contract_address = ""
        self.value = 0
        self.block_number = 0
        self.timestamp = 0
    
    def reset(self):
        """Reset VM state for new execution"""
        self.stack = []
        self.memory = {}
        self.pc = 0
        self.gas = 0
        self.stopped = False
    
    def execute(self, bytecode: bytes, caller: str, contract_address: str,
                value: float = 0, gas_limit: int = 1000000) -> Dict[str, Any]:
        """
        Execute bytecode
        
        Args:
            bytecode: Bytecode to execute
            caller: Address calling the contract
            contract_address: Contract being executed
            value: Value sent with transaction
            gas_limit: Maximum gas for execution
            
        Returns:
            Execution result dictionary
        """
        self.reset()
        self.caller = caller
        self.contract_address = contract_address
        self.value = value
        self.gas_limit = gas_limit
        
        result = {
            'success': True,
            'gas_used': 0,
            'return_value': None,
            'error': None
        }
        
        try:
            while self.pc < len(bytecode) and not self.stopped:
                if self.gas >= self.gas_limit:
                    raise Exception("Out of gas")
                
                opcode = bytecode[self.pc]
                self.execute_opcode(opcode, bytecode)
                self.pc += 1
                self.gas += 1
            
            result['gas_used'] = self.gas
            if self.stack:
                result['return_value'] = self.stack[-1]
                
        except Exception as e:
            result['success'] = False
            result['error'] = str(e)
            result['gas_used'] = self.gas
        
        return result
    
    def execute_opcode(self, opcode: int, bytecode: bytes):
        """
        Execute a single opcode
        
        Args:
            opcode: Operation code to execute
            bytecode: Complete bytecode for context
        """
        # Stack operations
        if opcode == OpCode.PUSH:
            self.pc += 1
            if self.pc < len(bytecode):
                value = bytecode[self.pc]
                self.stack.append(value)
        
        elif opcode == OpCode.POP:
            if self.stack:
                self.stack.pop()
        
        elif opcode == OpCode.DUP:
            if self.stack:
                self.stack.append(self.stack[-1])
        
        elif opcode == OpCode.SWAP:
            if len(self.stack) >= 2:
                self.stack[-1], self.stack[-2] = self.stack[-2], self.stack[-1]
        
        # Arithmetic operations
        elif opcode == OpCode.ADD:
            if len(self.stack) >= 2:
                b = self.stack.pop()
                a = self.stack.pop()
                self.stack.append(a + b)
        
        elif opcode == OpCode.SUB:
            if len(self.stack) >= 2:
                b = self.stack.pop()
                a = self.stack.pop()
                self.stack.append(a - b)
        
        elif opcode == OpCode.MUL:
            if len(self.stack) >= 2:
                b = self.stack.pop()
                a = self.stack.pop()
                self.stack.append(a * b)
        
        elif opcode == OpCode.DIV:
            if len(self.stack) >= 2:
                b = self.stack.pop()
                a = self.stack.pop()
                if b != 0:
                    self.stack.append(a // b)
                else:
                    self.stack.append(0)
        
        elif opcode == OpCode.MOD:
            if len(self.stack) >= 2:
                b = self.stack.pop()
                a = self.stack.pop()
                if b != 0:
                    self.stack.append(a % b)
                else:
                    self.stack.append(0)
        
        # Comparison operations
        elif opcode == OpCode.EQ:
            if len(self.stack) >= 2:
                b = self.stack.pop()
                a = self.stack.pop()
                self.stack.append(1 if a == b else 0)
        
        elif opcode == OpCode.LT:
            if len(self.stack) >= 2:
                b = self.stack.pop()
                a = self.stack.pop()
                self.stack.append(1 if a < b else 0)
        
        elif opcode == OpCode.GT:
            if len(self.stack) >= 2:
                b = self.stack.pop()
                a = self.stack.pop()
                self.stack.append(1 if a > b else 0)
        
        # Logical operations
        elif opcode == OpCode.AND:
            if len(self.stack) >= 2:
                b = self.stack.pop()
                a = self.stack.pop()
                self.stack.append(a & b)
        
        elif opcode == OpCode.OR:
            if len(self.stack) >= 2:
                b = self.stack.pop()
                a = self.stack.pop()
                self.stack.append(a | b)
        
        elif opcode == OpCode.NOT:
            if self.stack:
                a = self.stack.pop()
                self.stack.append(~a)
        
        # Memory operations
        elif opcode == OpCode.MLOAD:
            if self.stack:
                addr = self.stack.pop()
                value = self.memory.get(addr, 0)
                self.stack.append(value)
        
        elif opcode == OpCode.MSTORE:
            if len(self.stack) >= 2:
                addr = self.stack.pop()
                value = self.stack.pop()
                self.memory[addr] = value
        
        # Storage operations
        elif opcode == OpCode.SLOAD:
            if self.stack:
                key = str(self.stack.pop())
                value = self.state.get_storage(self.contract_address, key) or 0
                self.stack.append(value)
        
        elif opcode == OpCode.SSTORE:
            if len(self.stack) >= 2:
                key = str(self.stack.pop())
                value = self.stack.pop()
                self.state.set_storage(self.contract_address, key, value)
        
        # Flow control
        elif opcode == OpCode.JUMP:
            if self.stack:
                dest = self.stack.pop()
                self.pc = dest - 1  # -1 because pc will be incremented
        
        elif opcode == OpCode.JUMPI:
            if len(self.stack) >= 2:
                dest = self.stack.pop()
                condition = self.stack.pop()
                if condition:
                    self.pc = dest - 1
        
        # System operations
        elif opcode == OpCode.RETURN:
            self.stopped = True
        
        elif opcode == OpCode.REVERT:
            self.stopped = True
            raise Exception("Execution reverted")
        
        elif opcode == OpCode.STOP:
            self.stopped = True
        
        # Blockchain operations
        elif opcode == OpCode.BALANCE:
            if self.stack:
                addr = str(self.stack.pop())
                balance = self.state.get_balance(addr)
                self.stack.append(balance)
        
        elif opcode == OpCode.TRANSFER:
            if len(self.stack) >= 2:
                recipient = str(self.stack.pop())
                amount = self.stack.pop()
                # Transfer value
                self.state.update_balance(self.caller, -amount)
                self.state.update_balance(recipient, amount)
        
        elif opcode == OpCode.SENDER:
            # Push caller address (as hash for simplicity)
            self.stack.append(hash(self.caller) & 0xFFFFFFFF)
        
        elif opcode == OpCode.VALUE:
            self.stack.append(self.value)
        
        elif opcode == OpCode.TIMESTAMP:
            self.stack.append(self.timestamp)
        
        elif opcode == OpCode.BLOCKNUMBER:
            self.stack.append(self.block_number)
    
    def deploy_contract(self, bytecode: bytes, deployer: str) -> str:
        """
        Deploy a smart contract
        
        Args:
            bytecode: Contract bytecode
            deployer: Address deploying the contract
            
        Returns:
            Contract address
        """
        # Generate contract address from deployer and nonce
        nonce = self.state.get_nonce(deployer)
        contract_address = f"contract_{deployer}_{nonce}"
        
        # Deploy contract
        self.state.deploy_contract(contract_address, bytecode.hex())
        self.state.increment_nonce(deployer)
        
        return contract_address
    
    def call_contract(self, contract_address: str, caller: str,
                     value: float = 0, gas_limit: int = 1000000) -> Dict[str, Any]:
        """
        Call a deployed contract
        
        Args:
            contract_address: Address of contract to call
            caller: Address calling the contract
            value: Value sent with call
            gas_limit: Maximum gas for execution
            
        Returns:
            Execution result
        """
        code_hex = self.state.get_contract_code(contract_address)
        if not code_hex:
            return {
                'success': False,
                'error': 'Contract not found',
                'gas_used': 0
            }
        
        bytecode = bytes.fromhex(code_hex)
        return self.execute(bytecode, caller, contract_address, value, gas_limit)
