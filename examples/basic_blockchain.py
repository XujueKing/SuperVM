#!/usr/bin/env python3
"""
Example: Basic blockchain operations
Demonstrates wallet creation, transactions, and mining
"""

from supervm import Blockchain, Transaction, CryptoUtils

def main():
    print("="*60)
    print("SuperVM - Basic Blockchain Example")
    print("="*60)
    
    # Create blockchain with difficulty 2
    blockchain = Blockchain(difficulty=2)
    print(f"\n✓ Blockchain created with difficulty {blockchain.difficulty}")
    
    # Create wallets
    print("\n1. Creating wallets...")
    alice_priv, alice_pub = CryptoUtils.generate_keypair()
    alice_addr = CryptoUtils.derive_address(alice_pub)
    print(f"   Alice: {alice_addr[:16]}...")
    
    bob_priv, bob_pub = CryptoUtils.generate_keypair()
    bob_addr = CryptoUtils.derive_address(bob_pub)
    print(f"   Bob: {bob_addr[:16]}...")
    
    charlie_priv, charlie_pub = CryptoUtils.generate_keypair()
    charlie_addr = CryptoUtils.derive_address(charlie_pub)
    print(f"   Charlie: {charlie_addr[:16]}...")
    
    # Alice mines first block to get some coins
    print("\n2. Alice mines first block...")
    blockchain.mine_pending_transactions(alice_addr)
    print(f"   ✓ Block mined! Alice balance: {blockchain.get_balance(alice_addr)}")
    
    # Alice sends money to Bob
    print("\n3. Alice sends 30 to Bob...")
    tx1 = Transaction(alice_addr, bob_addr, 30.0)
    tx1.sign(alice_priv)
    blockchain.add_transaction(tx1)
    print("   ✓ Transaction added to pending pool")
    
    # Alice sends money to Charlie
    print("\n4. Alice sends 20 to Charlie...")
    tx2 = Transaction(alice_addr, charlie_addr, 20.0)
    tx2.sign(alice_priv)
    blockchain.add_transaction(tx2)
    print("   ✓ Transaction added to pending pool")
    
    # Bob mines the block
    print("\n5. Bob mines the next block...")
    blockchain.mine_pending_transactions(bob_addr)
    print(f"   ✓ Block mined!")
    
    # Check balances
    print("\n6. Final balances:")
    print(f"   Alice: {blockchain.get_balance(alice_addr)}")
    print(f"   Bob: {blockchain.get_balance(bob_addr)}")
    print(f"   Charlie: {blockchain.get_balance(charlie_addr)}")
    
    # Blockchain info
    print("\n7. Blockchain status:")
    print(f"   Total blocks: {len(blockchain.chain)}")
    print(f"   Valid: {blockchain.is_chain_valid()}")
    print(f"   Pending transactions: {len(blockchain.pending_transactions)}")
    
    # Show blocks
    print("\n8. Block details:")
    for block in blockchain.chain:
        print(f"\n   Block #{block.index}")
        print(f"   Hash: {block.hash}")
        print(f"   Previous: {block.previous_hash[:16]}...")
        print(f"   Transactions: {len(block.transactions)}")
        print(f"   Nonce: {block.nonce}")
        
        for i, tx in enumerate(block.transactions):
            print(f"      TX{i}: {tx.sender[:10]}... → {tx.recipient[:10]}... ({tx.amount})")
    
    print("\n" + "="*60)
    print("Example completed successfully!")
    print("="*60)


if __name__ == '__main__':
    main()
