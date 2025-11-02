#!/usr/bin/env python3
"""
Integration test - Full system demonstration
Tests all components working together
"""

import sys
sys.path.insert(0, '/home/runner/work/SuperVM/SuperVM')

from supervm import VirtualMachine, Blockchain, Transaction, CryptoUtils, StateManager
from supervm.vm import OpCode

def test_full_system():
    """Test complete system integration"""
    print("="*60)
    print("SuperVM - Full System Integration Test")
    print("="*60)
    
    # 1. Test Cryptography
    print("\n[1/5] Testing Cryptography...")
    priv1, pub1 = CryptoUtils.generate_keypair()
    addr1 = CryptoUtils.derive_address(pub1)
    data = "test message"
    sig = CryptoUtils.sign_data(data, priv1)
    assert CryptoUtils.verify_signature(data, sig, pub1), "Signature verification failed"
    print("✓ Cryptography working correctly")
    
    # 2. Test Blockchain
    print("\n[2/5] Testing Blockchain...")
    bc = Blockchain(difficulty=1)
    assert len(bc.chain) == 1, "Genesis block not created"
    
    tx1 = Transaction(addr1, "recipient1", 50.0)
    bc.add_transaction(tx1)
    bc.mine_pending_transactions("miner")
    
    assert len(bc.chain) == 2, "Block not mined"
    assert bc.is_chain_valid(), "Blockchain invalid"
    print("✓ Blockchain working correctly")
    
    # 3. Test State Management
    print("\n[3/5] Testing State Management...")
    state = StateManager()
    state.create_account("test_account", 100.0)
    assert state.get_balance("test_account") == 100.0, "Balance incorrect"
    
    state.deploy_contract("contract1", "deadbeef")
    assert state.is_contract("contract1"), "Contract not deployed"
    
    state.set_storage("contract1", "key", "value")
    assert state.get_storage("contract1", "key") == "value", "Storage incorrect"
    print("✓ State management working correctly")
    
    # 4. Test Virtual Machine
    print("\n[4/5] Testing Virtual Machine...")
    vm = VirtualMachine()
    
    # Test arithmetic
    bytecode = bytes([OpCode.PUSH, 10, OpCode.PUSH, 5, OpCode.ADD, OpCode.RETURN])
    result = vm.execute(bytecode, "caller", "contract")
    assert result['success'], "VM execution failed"
    assert result['return_value'] == 15, "Arithmetic incorrect"
    
    # Test contract deployment
    contract_addr = vm.deploy_contract(bytecode, "deployer")
    result = vm.call_contract(contract_addr, "caller")
    assert result['success'], "Contract call failed"
    assert result['return_value'] == 15, "Contract result incorrect"
    print("✓ Virtual machine working correctly")
    
    # 5. Test End-to-End Workflow
    print("\n[5/5] Testing End-to-End Workflow...")
    
    # Create wallets
    priv_alice, pub_alice = CryptoUtils.generate_keypair()
    addr_alice = CryptoUtils.derive_address(pub_alice)
    
    priv_bob, pub_bob = CryptoUtils.generate_keypair()
    addr_bob = CryptoUtils.derive_address(pub_bob)
    
    # Create blockchain
    chain = Blockchain(difficulty=1)
    
    # Mine blocks
    chain.mine_pending_transactions(addr_alice)  # Alice gets mining reward
    
    # Create and sign transaction
    tx = Transaction(addr_alice, addr_bob, 30.0)
    tx.sign(priv_alice)
    assert tx.verify(pub_alice), "Transaction signature verification failed"
    
    # Add transaction and mine
    chain.add_transaction(tx)
    chain.mine_pending_transactions(addr_bob)  # Bob mines
    
    # Verify balances
    alice_balance = chain.get_balance(addr_alice)
    bob_balance = chain.get_balance(addr_bob)
    
    assert alice_balance == 70.0, f"Alice balance incorrect: {alice_balance}"
    assert bob_balance == 130.0, f"Bob balance incorrect: {bob_balance}"
    
    # Deploy smart contract
    vm = VirtualMachine()
    contract_bytecode = bytes([
        OpCode.PUSH, 100,
        OpCode.PUSH, 0,
        OpCode.SSTORE,
        OpCode.PUSH, 0,
        OpCode.SLOAD,
        OpCode.RETURN
    ])
    
    contract_addr = vm.deploy_contract(contract_bytecode, addr_alice)
    result = vm.call_contract(contract_addr, addr_bob)
    
    assert result['success'], "Contract execution failed"
    assert result['return_value'] == 100, "Contract storage incorrect"
    
    print("✓ End-to-end workflow working correctly")
    
    # Final verification
    print("\n" + "="*60)
    print("ALL INTEGRATION TESTS PASSED!")
    print("="*60)
    print(f"\nSystem Summary:")
    print(f"  Blockchain blocks: {len(chain.chain)}")
    print(f"  Chain valid: {chain.is_chain_valid()}")
    print(f"  Accounts created: 2")
    print(f"  Transactions processed: 3")
    print(f"  Contracts deployed: 2")
    print(f"  Alice balance: {alice_balance}")
    print(f"  Bob balance: {bob_balance}")
    print("\n✓ SuperVM is fully operational!")
    
    return True


if __name__ == '__main__':
    try:
        success = test_full_system()
        sys.exit(0 if success else 1)
    except Exception as e:
        print(f"\n✗ Integration test failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
