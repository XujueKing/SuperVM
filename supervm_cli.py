#!/usr/bin/env python3
"""
SuperVM Command Line Interface
Interact with the WEB3.0 Blockchain Super Virtual Machine
"""

import argparse
import json
import sys
from supervm import VirtualMachine, Blockchain, Transaction, CryptoUtils, StateManager
from supervm.vm import OpCode


class SuperVMCLI:
    """Command line interface for SuperVM"""
    
    def __init__(self):
        self.blockchain = Blockchain(difficulty=2)
        self.vm = VirtualMachine()
        self.wallets = {}  # Store generated wallets
    
    def create_wallet(self, name: str):
        """Create a new wallet"""
        private_key, public_key = CryptoUtils.generate_keypair()
        address = CryptoUtils.derive_address(public_key)
        
        self.wallets[name] = {
            'private_key': private_key,
            'public_key': public_key,
            'address': address
        }
        
        print(f"✓ Wallet '{name}' created successfully!")
        print(f"  Address: {address}")
        print(f"  Public Key: {public_key[:32]}...")
        print(f"  Private Key: {private_key[:32]}... (keep this secret!)")
        
        return address
    
    def send_transaction(self, from_wallet: str, to_address: str, amount: float):
        """Send a transaction"""
        if from_wallet not in self.wallets:
            print(f"✗ Wallet '{from_wallet}' not found!")
            return
        
        wallet = self.wallets[from_wallet]
        tx = Transaction(wallet['address'], to_address, amount)
        tx.sign(wallet['private_key'])
        
        self.blockchain.add_transaction(tx)
        print(f"✓ Transaction added to pending pool")
        print(f"  From: {wallet['address'][:16]}...")
        print(f"  To: {to_address[:16]}...")
        print(f"  Amount: {amount}")
    
    def mine_block(self, miner_wallet: str):
        """Mine a new block"""
        if miner_wallet not in self.wallets:
            print(f"✗ Wallet '{miner_wallet}' not found!")
            return
        
        miner_address = self.wallets[miner_wallet]['address']
        
        print(f"⛏ Mining new block...")
        print(f"  Pending transactions: {len(self.blockchain.pending_transactions)}")
        
        self.blockchain.mine_pending_transactions(miner_address)
        
        latest_block = self.blockchain.get_latest_block()
        print(f"✓ Block #{latest_block.index} mined!")
        print(f"  Hash: {latest_block.hash}")
        print(f"  Nonce: {latest_block.nonce}")
        print(f"  Transactions: {len(latest_block.transactions)}")
    
    def get_balance(self, wallet_name: str):
        """Get wallet balance"""
        if wallet_name not in self.wallets:
            print(f"✗ Wallet '{wallet_name}' not found!")
            return
        
        address = self.wallets[wallet_name]['address']
        balance = self.blockchain.get_balance(address)
        
        print(f"Balance for '{wallet_name}':")
        print(f"  Address: {address[:16]}...")
        print(f"  Balance: {balance}")
    
    def show_blockchain(self):
        """Display blockchain information"""
        print(f"\n{'='*60}")
        print(f"BLOCKCHAIN STATUS")
        print(f"{'='*60}")
        print(f"Total Blocks: {len(self.blockchain.chain)}")
        print(f"Difficulty: {self.blockchain.difficulty}")
        print(f"Valid: {self.blockchain.is_chain_valid()}")
        print(f"Pending Transactions: {len(self.blockchain.pending_transactions)}")
        print(f"\nRecent Blocks:")
        
        for block in self.blockchain.chain[-5:]:
            print(f"\n  Block #{block.index}")
            print(f"    Hash: {block.hash}")
            print(f"    Previous: {block.previous_hash[:16]}...")
            print(f"    Transactions: {len(block.transactions)}")
            print(f"    Nonce: {block.nonce}")
    
    def deploy_contract(self, wallet_name: str, bytecode_hex: str):
        """Deploy a smart contract"""
        if wallet_name not in self.wallets:
            print(f"✗ Wallet '{wallet_name}' not found!")
            return
        
        deployer = self.wallets[wallet_name]['address']
        bytecode = bytes.fromhex(bytecode_hex)
        
        contract_address = self.vm.deploy_contract(bytecode, deployer)
        
        print(f"✓ Contract deployed successfully!")
        print(f"  Contract Address: {contract_address}")
        print(f"  Deployer: {deployer[:16]}...")
        
        return contract_address
    
    def call_contract(self, wallet_name: str, contract_address: str):
        """Call a smart contract"""
        if wallet_name not in self.wallets:
            print(f"✗ Wallet '{wallet_name}' not found!")
            return
        
        caller = self.wallets[wallet_name]['address']
        
        print(f"Calling contract {contract_address}...")
        result = self.vm.call_contract(contract_address, caller)
        
        print(f"\nExecution Result:")
        print(f"  Success: {result['success']}")
        print(f"  Gas Used: {result['gas_used']}")
        if result['return_value'] is not None:
            print(f"  Return Value: {result['return_value']}")
        if result['error']:
            print(f"  Error: {result['error']}")
    
    def execute_bytecode(self, wallet_name: str, bytecode_hex: str):
        """Execute bytecode directly"""
        if wallet_name not in self.wallets:
            print(f"✗ Wallet '{wallet_name}' not found!")
            return
        
        caller = self.wallets[wallet_name]['address']
        bytecode = bytes.fromhex(bytecode_hex)
        
        print(f"Executing bytecode...")
        result = self.vm.execute(bytecode, caller, "direct_execution")
        
        print(f"\nExecution Result:")
        print(f"  Success: {result['success']}")
        print(f"  Gas Used: {result['gas_used']}")
        if result['return_value'] is not None:
            print(f"  Return Value: {result['return_value']}")
        if result['error']:
            print(f"  Error: {result['error']}")


def main():
    """Main CLI entry point"""
    cli = SuperVMCLI()
    
    print("="*60)
    print("SuperVM - WEB3.0 Blockchain Super Virtual Machine")
    print("="*60)
    print()
    
    parser = argparse.ArgumentParser(description='SuperVM CLI')
    subparsers = parser.add_subparsers(dest='command', help='Commands')
    
    # Wallet commands
    wallet_parser = subparsers.add_parser('wallet', help='Wallet operations')
    wallet_parser.add_argument('action', choices=['create', 'balance'], help='Wallet action')
    wallet_parser.add_argument('name', help='Wallet name')
    
    # Transaction commands
    tx_parser = subparsers.add_parser('send', help='Send transaction')
    tx_parser.add_argument('from_wallet', help='From wallet name')
    tx_parser.add_argument('to_address', help='To address')
    tx_parser.add_argument('amount', type=float, help='Amount to send')
    
    # Mining commands
    mine_parser = subparsers.add_parser('mine', help='Mine a block')
    mine_parser.add_argument('wallet', help='Miner wallet name')
    
    # Blockchain commands
    subparsers.add_parser('status', help='Show blockchain status')
    
    # Contract commands
    deploy_parser = subparsers.add_parser('deploy', help='Deploy smart contract')
    deploy_parser.add_argument('wallet', help='Deployer wallet name')
    deploy_parser.add_argument('bytecode', help='Contract bytecode (hex)')
    
    call_parser = subparsers.add_parser('call', help='Call smart contract')
    call_parser.add_argument('wallet', help='Caller wallet name')
    call_parser.add_argument('contract', help='Contract address')
    
    exec_parser = subparsers.add_parser('exec', help='Execute bytecode')
    exec_parser.add_argument('wallet', help='Executor wallet name')
    exec_parser.add_argument('bytecode', help='Bytecode to execute (hex)')
    
    # Interactive mode
    subparsers.add_parser('interactive', help='Start interactive mode')
    
    args = parser.parse_args()
    
    if args.command == 'wallet':
        if args.action == 'create':
            cli.create_wallet(args.name)
        elif args.action == 'balance':
            cli.get_balance(args.name)
    
    elif args.command == 'send':
        cli.send_transaction(args.from_wallet, args.to_address, args.amount)
    
    elif args.command == 'mine':
        cli.mine_block(args.wallet)
    
    elif args.command == 'status':
        cli.show_blockchain()
    
    elif args.command == 'deploy':
        cli.deploy_contract(args.wallet, args.bytecode)
    
    elif args.command == 'call':
        cli.call_contract(args.wallet, args.contract)
    
    elif args.command == 'exec':
        cli.execute_bytecode(args.wallet, args.bytecode)
    
    elif args.command == 'interactive':
        print("\nInteractive mode - type 'help' for commands, 'exit' to quit")
        interactive_mode(cli)
    
    else:
        parser.print_help()


def interactive_mode(cli):
    """Interactive REPL mode"""
    print("\nAvailable commands:")
    print("  create <wallet_name>           - Create a new wallet")
    print("  balance <wallet_name>          - Check wallet balance")
    print("  send <from> <to> <amount>      - Send transaction")
    print("  mine <wallet_name>             - Mine pending transactions")
    print("  status                         - Show blockchain status")
    print("  deploy <wallet> <bytecode>     - Deploy smart contract")
    print("  exec <wallet> <bytecode>       - Execute bytecode")
    print("  help                           - Show this help")
    print("  exit                           - Exit interactive mode")
    print()
    
    while True:
        try:
            cmd = input("supervm> ").strip().split()
            if not cmd:
                continue
            
            if cmd[0] == 'exit':
                break
            elif cmd[0] == 'help':
                print("Commands: create, balance, send, mine, status, deploy, exec, exit")
            elif cmd[0] == 'create' and len(cmd) == 2:
                cli.create_wallet(cmd[1])
            elif cmd[0] == 'balance' and len(cmd) == 2:
                cli.get_balance(cmd[1])
            elif cmd[0] == 'send' and len(cmd) == 4:
                cli.send_transaction(cmd[1], cmd[2], float(cmd[3]))
            elif cmd[0] == 'mine' and len(cmd) == 2:
                cli.mine_block(cmd[1])
            elif cmd[0] == 'status':
                cli.show_blockchain()
            elif cmd[0] == 'deploy' and len(cmd) == 3:
                cli.deploy_contract(cmd[1], cmd[2])
            elif cmd[0] == 'exec' and len(cmd) == 3:
                cli.execute_bytecode(cmd[1], cmd[2])
            else:
                print("Invalid command. Type 'help' for available commands.")
        
        except KeyboardInterrupt:
            print("\nUse 'exit' to quit")
        except Exception as e:
            print(f"Error: {e}")


if __name__ == '__main__':
    main()
