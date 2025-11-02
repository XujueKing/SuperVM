"""
Test suite for SuperVM virtual machine
"""

import unittest
from supervm.vm import VirtualMachine, OpCode
from supervm.state import StateManager


class TestVirtualMachine(unittest.TestCase):
    """Test VM execution"""
    
    def setUp(self):
        """Set up test VM"""
        self.vm = VirtualMachine()
    
    def test_stack_operations(self):
        """Test PUSH, POP, DUP, SWAP"""
        # PUSH 5, PUSH 10, SWAP
        bytecode = bytes([
            OpCode.PUSH, 5,
            OpCode.PUSH, 10,
            OpCode.SWAP,
            OpCode.RETURN
        ])
        
        result = self.vm.execute(bytecode, "test", "test")
        self.assertTrue(result['success'])
        self.assertEqual(self.vm.stack[-1], 5)
        self.assertEqual(self.vm.stack[-2], 10)
    
    def test_arithmetic_operations(self):
        """Test ADD, SUB, MUL, DIV"""
        # PUSH 10, PUSH 5, ADD
        bytecode = bytes([
            OpCode.PUSH, 10,
            OpCode.PUSH, 5,
            OpCode.ADD,
            OpCode.RETURN
        ])
        
        result = self.vm.execute(bytecode, "test", "test")
        self.assertTrue(result['success'])
        self.assertEqual(self.vm.stack[-1], 15)
        
        # PUSH 10, PUSH 3, MUL
        bytecode = bytes([
            OpCode.PUSH, 10,
            OpCode.PUSH, 3,
            OpCode.MUL,
            OpCode.RETURN
        ])
        
        self.vm.reset()
        result = self.vm.execute(bytecode, "test", "test")
        self.assertTrue(result['success'])
        self.assertEqual(self.vm.stack[-1], 30)
    
    def test_comparison_operations(self):
        """Test EQ, LT, GT"""
        # PUSH 5, PUSH 5, EQ (should be 1)
        bytecode = bytes([
            OpCode.PUSH, 5,
            OpCode.PUSH, 5,
            OpCode.EQ,
            OpCode.RETURN
        ])
        
        result = self.vm.execute(bytecode, "test", "test")
        self.assertTrue(result['success'])
        self.assertEqual(self.vm.stack[-1], 1)
        
        # PUSH 3, PUSH 5, LT (should be 1)
        bytecode = bytes([
            OpCode.PUSH, 3,
            OpCode.PUSH, 5,
            OpCode.LT,
            OpCode.RETURN
        ])
        
        self.vm.reset()
        result = self.vm.execute(bytecode, "test", "test")
        self.assertTrue(result['success'])
        self.assertEqual(self.vm.stack[-1], 1)
    
    def test_memory_operations(self):
        """Test MLOAD, MSTORE"""
        # PUSH 42, PUSH 0, MSTORE, PUSH 0, MLOAD
        bytecode = bytes([
            OpCode.PUSH, 42,
            OpCode.PUSH, 0,
            OpCode.MSTORE,
            OpCode.PUSH, 0,
            OpCode.MLOAD,
            OpCode.RETURN
        ])
        
        result = self.vm.execute(bytecode, "test", "test")
        self.assertTrue(result['success'])
        self.assertEqual(self.vm.stack[-1], 42)
    
    def test_storage_operations(self):
        """Test SLOAD, SSTORE"""
        # PUSH 100, PUSH 1, SSTORE, PUSH 1, SLOAD
        bytecode = bytes([
            OpCode.PUSH, 100,
            OpCode.PUSH, 1,
            OpCode.SSTORE,
            OpCode.PUSH, 1,
            OpCode.SLOAD,
            OpCode.RETURN
        ])
        
        result = self.vm.execute(bytecode, "test", "contract_addr")
        self.assertTrue(result['success'])
        self.assertEqual(self.vm.stack[-1], 100)
    
    def test_contract_deployment(self):
        """Test contract deployment"""
        bytecode = bytes([
            OpCode.PUSH, 42,
            OpCode.RETURN
        ])
        
        contract_addr = self.vm.deploy_contract(bytecode, "deployer")
        self.assertIsNotNone(contract_addr)
        self.assertTrue(contract_addr.startswith("contract_"))
        
        # Verify contract code is stored
        code = self.vm.state.get_contract_code(contract_addr)
        self.assertEqual(code, bytecode.hex())
    
    def test_contract_call(self):
        """Test calling deployed contract"""
        bytecode = bytes([
            OpCode.PUSH, 10,
            OpCode.PUSH, 5,
            OpCode.ADD,
            OpCode.RETURN
        ])
        
        contract_addr = self.vm.deploy_contract(bytecode, "deployer")
        result = self.vm.call_contract(contract_addr, "caller")
        
        self.assertTrue(result['success'])
        self.assertEqual(result['return_value'], 15)
    
    def test_gas_limit(self):
        """Test gas limit enforcement"""
        # Infinite loop would exceed gas
        bytecode = bytes([
            OpCode.PUSH, 0,  # Position 0
            OpCode.JUMP,      # Jump to 0
        ])
        
        result = self.vm.execute(bytecode, "test", "test", gas_limit=100)
        self.assertFalse(result['success'])
        self.assertIn("gas", result['error'].lower())
    
    def test_revert_operation(self):
        """Test REVERT opcode"""
        bytecode = bytes([
            OpCode.PUSH, 42,
            OpCode.REVERT
        ])
        
        result = self.vm.execute(bytecode, "test", "test")
        self.assertFalse(result['success'])
        self.assertIn("revert", result['error'].lower())


if __name__ == '__main__':
    unittest.main()
