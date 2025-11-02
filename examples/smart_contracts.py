#!/usr/bin/env python3
"""
Example: Smart contract deployment and execution
Demonstrates VM bytecode execution and contract interaction
"""

from supervm import VirtualMachine
from supervm.vm import OpCode

def main():
    print("="*60)
    print("SuperVM - Smart Contract Example")
    print("="*60)
    
    # Create VM
    vm = VirtualMachine()
    
    # Example 1: Simple arithmetic contract
    print("\n1. Simple Arithmetic Contract")
    print("   Contract: Add 10 + 5")
    
    bytecode1 = bytes([
        OpCode.PUSH, 10,
        OpCode.PUSH, 5,
        OpCode.ADD,
        OpCode.RETURN
    ])
    
    contract1_addr = vm.deploy_contract(bytecode1, "deployer")
    print(f"   ✓ Contract deployed at: {contract1_addr}")
    
    result1 = vm.call_contract(contract1_addr, "caller")
    print(f"   Result: {result1['return_value']}")
    print(f"   Gas used: {result1['gas_used']}")
    
    # Example 2: Storage contract
    print("\n2. Storage Contract")
    print("   Contract: Store value 42 and retrieve it")
    
    bytecode2 = bytes([
        OpCode.PUSH, 42,     # Push value
        OpCode.PUSH, 0,      # Push storage key
        OpCode.SSTORE,       # Store value at key 0
        OpCode.PUSH, 0,      # Push storage key
        OpCode.SLOAD,        # Load from key 0
        OpCode.RETURN
    ])
    
    contract2_addr = vm.deploy_contract(bytecode2, "deployer")
    print(f"   ✓ Contract deployed at: {contract2_addr}")
    
    result2 = vm.call_contract(contract2_addr, "caller")
    print(f"   Stored and retrieved: {result2['return_value']}")
    print(f"   Gas used: {result2['gas_used']}")
    
    # Example 3: Counter contract
    print("\n3. Counter Contract")
    print("   Contract: Increment counter in storage")
    
    # First call: Initialize counter to 0
    bytecode3a = bytes([
        OpCode.PUSH, 0,      # Initial value
        OpCode.PUSH, 1,      # Storage key for counter
        OpCode.SSTORE,       # Store
        OpCode.RETURN
    ])
    
    contract3_addr = vm.deploy_contract(bytecode3a, "deployer")
    print(f"   ✓ Contract deployed at: {contract3_addr}")
    vm.call_contract(contract3_addr, "caller")
    print("   ✓ Counter initialized to 0")
    
    # Increment counter multiple times
    bytecode3b = bytes([
        OpCode.PUSH, 1,      # Storage key
        OpCode.SLOAD,        # Load current value
        OpCode.PUSH, 1,      # Push increment
        OpCode.ADD,          # Add
        OpCode.PUSH, 1,      # Storage key
        OpCode.SSTORE,       # Store new value
        OpCode.PUSH, 1,      # Load key again
        OpCode.SLOAD,        # Load for return
        OpCode.RETURN
    ])
    
    # Update contract code for incrementing
    vm.state.contract_code[contract3_addr] = bytecode3b.hex()
    
    for i in range(5):
        result = vm.call_contract(contract3_addr, "caller")
        print(f"   Increment {i+1}: Counter = {result['return_value']}")
    
    # Example 4: Comparison contract
    print("\n4. Comparison Contract")
    print("   Contract: Compare two numbers")
    
    bytecode4 = bytes([
        OpCode.PUSH, 10,     # First number
        OpCode.PUSH, 5,      # Second number
        OpCode.GT,           # Greater than comparison
        OpCode.RETURN
    ])
    
    contract4_addr = vm.deploy_contract(bytecode4, "deployer")
    print(f"   ✓ Contract deployed at: {contract4_addr}")
    
    result4 = vm.call_contract(contract4_addr, "caller")
    print(f"   Is 10 > 5? {result4['return_value']} (1=True, 0=False)")
    print(f"   Gas used: {result4['gas_used']}")
    
    # Example 5: Memory operations
    print("\n5. Memory Contract")
    print("   Contract: Use memory to store temporary values")
    
    bytecode5 = bytes([
        OpCode.PUSH, 100,    # Value to store
        OpCode.PUSH, 0,      # Memory address
        OpCode.MSTORE,       # Store in memory
        OpCode.PUSH, 0,      # Memory address
        OpCode.MLOAD,        # Load from memory
        OpCode.PUSH, 50,     # Value to add
        OpCode.ADD,          # Add values
        OpCode.RETURN
    ])
    
    contract5_addr = vm.deploy_contract(bytecode5, "deployer")
    print(f"   ✓ Contract deployed at: {contract5_addr}")
    
    result5 = vm.call_contract(contract5_addr, "caller")
    print(f"   Memory operation result: {result5['return_value']}")
    print(f"   Gas used: {result5['gas_used']}")
    
    # Example 6: Complex calculation
    print("\n6. Complex Calculation Contract")
    print("   Contract: (10 + 5) * 3")
    
    bytecode6 = bytes([
        OpCode.PUSH, 10,
        OpCode.PUSH, 5,
        OpCode.ADD,          # 10 + 5 = 15
        OpCode.PUSH, 3,
        OpCode.MUL,          # 15 * 3 = 45
        OpCode.RETURN
    ])
    
    contract6_addr = vm.deploy_contract(bytecode6, "deployer")
    print(f"   ✓ Contract deployed at: {contract6_addr}")
    
    result6 = vm.call_contract(contract6_addr, "caller")
    print(f"   Calculation result: {result6['return_value']}")
    print(f"   Gas used: {result6['gas_used']}")
    
    print("\n" + "="*60)
    print("All contract examples completed successfully!")
    print("="*60)


if __name__ == '__main__':
    main()
