# SuperVM

**WEB3.0 Blockchain Super Virtual Machine**

A comprehensive blockchain virtual machine implementation with smart contract support, proof-of-work consensus, and a complete execution environment.

## Features

- 🔗 **Full Blockchain Implementation**: Complete blockchain with proof-of-work consensus
- 💻 **Virtual Machine**: Stack-based VM with custom instruction set
- 📜 **Smart Contracts**: Deploy and execute smart contracts with bytecode
- 🔐 **Cryptography**: ECDSA signatures, SHA-256 hashing, secure key management
- 💰 **Wallet System**: Create wallets, manage keys, and track balances
- ⛏️ **Mining**: Proof-of-work mining with configurable difficulty
- 🔄 **State Management**: Persistent state for accounts and contract storage
- 🖥️ **CLI Interface**: Interactive command-line interface

## Architecture

### Core Components

1. **Blockchain** (`blockchain.py`)
   - Block structure and chain management
   - Transaction handling
   - Proof-of-work consensus
   - Chain validation

2. **Virtual Machine** (`vm.py`)
   - Stack-based execution engine
   - 30+ opcodes (arithmetic, logic, memory, storage, etc.)
   - Gas metering for resource management
   - Smart contract deployment and execution

3. **State Manager** (`state.py`)
   - Account state tracking
   - Contract storage
   - Balance management
   - State snapshots

4. **Cryptography** (`crypto.py`)
   - ECDSA key generation and signing
   - SHA-256 hashing
   - Signature verification
   - Address derivation

## Installation

```bash
# Clone the repository
git clone https://github.com/XujueKing/SuperVM.git
cd SuperVM

# Install dependencies
pip install -r requirements.txt
```

## Quick Start

### Using the CLI

```bash
# Create wallets
python supervm_cli.py wallet create alice
python supervm_cli.py wallet create bob

# Check balance
python supervm_cli.py wallet balance alice

# Send transaction
python supervm_cli.py send alice <bob_address> 50

# Mine a block
python supervm_cli.py mine alice

# View blockchain status
python supervm_cli.py status

# Interactive mode
python supervm_cli.py interactive
```

### Using as a Library

```python
from supervm import VirtualMachine, Blockchain, Transaction, CryptoUtils

# Create blockchain
blockchain = Blockchain(difficulty=2)

# Create transaction
tx = Transaction("alice", "bob", 100.0)
blockchain.add_transaction(tx)

# Mine block
blockchain.mine_pending_transactions("miner")

# Check balance
balance = blockchain.get_balance("alice")

# Deploy smart contract
vm = VirtualMachine()
bytecode = bytes([0x01, 0x0A, 0x01, 0x05, 0x10, 0x71])  # PUSH 10, PUSH 5, ADD, RETURN
contract_addr = vm.deploy_contract(bytecode, "deployer")

# Call contract
result = vm.call_contract(contract_addr, "caller")
print(f"Result: {result['return_value']}")
```

## VM Instruction Set

### Stack Operations
- `PUSH` (0x01): Push value onto stack
- `POP` (0x02): Pop value from stack
- `DUP` (0x03): Duplicate top stack item
- `SWAP` (0x04): Swap top two stack items

### Arithmetic Operations
- `ADD` (0x10): Addition
- `SUB` (0x11): Subtraction
- `MUL` (0x12): Multiplication
- `DIV` (0x13): Division
- `MOD` (0x14): Modulo

### Comparison Operations
- `EQ` (0x20): Equal
- `LT` (0x21): Less than
- `GT` (0x22): Greater than

### Logical Operations
- `AND` (0x30): Bitwise AND
- `OR` (0x31): Bitwise OR
- `NOT` (0x32): Bitwise NOT

### Memory Operations
- `MLOAD` (0x40): Load from memory
- `MSTORE` (0x41): Store to memory

### Storage Operations
- `SLOAD` (0x50): Load from storage
- `SSTORE` (0x51): Store to storage

### Flow Control
- `JUMP` (0x60): Unconditional jump
- `JUMPI` (0x61): Conditional jump
- `JUMPDEST` (0x62): Jump destination

### System Operations
- `CALL` (0x70): Call contract
- `RETURN` (0x71): Return from execution
- `REVERT` (0x72): Revert execution
- `STOP` (0x73): Stop execution

### Blockchain Operations
- `BALANCE` (0x80): Get account balance
- `TRANSFER` (0x81): Transfer value
- `SENDER` (0x82): Get caller address
- `VALUE` (0x83): Get transaction value
- `TIMESTAMP` (0x84): Get block timestamp
- `BLOCKNUMBER` (0x85): Get block number

## Smart Contract Example

```python
from supervm import VirtualMachine
from supervm.vm import OpCode

# Simple contract: Store and retrieve a value
# PUSH 42, PUSH 0, SSTORE (store 42 at key 0)
# PUSH 0, SLOAD (load from key 0)
# RETURN

bytecode = bytes([
    OpCode.PUSH, 42,
    OpCode.PUSH, 0,
    OpCode.SSTORE,
    OpCode.PUSH, 0,
    OpCode.SLOAD,
    OpCode.RETURN
])

vm = VirtualMachine()
contract_addr = vm.deploy_contract(bytecode, "deployer")
result = vm.call_contract(contract_addr, "caller")

print(f"Stored value: {result['return_value']}")  # Output: 42
```

## Running Tests

```bash
# Run all tests
python -m unittest discover tests -v

# Run specific test file
python -m unittest tests.test_vm -v
```

## Project Structure

```
SuperVM/
├── supervm/
│   ├── __init__.py       # Package initialization
│   ├── blockchain.py     # Blockchain implementation
│   ├── vm.py            # Virtual machine
│   ├── state.py         # State management
│   └── crypto.py        # Cryptographic utilities
├── tests/
│   ├── test_blockchain.py
│   ├── test_vm.py
│   ├── test_state.py
│   └── test_crypto.py
├── supervm_cli.py       # Command-line interface
├── requirements.txt     # Python dependencies
└── README.md           # This file
```

## Development

### Adding New Opcodes

1. Add opcode constant to `OpCode` class in `vm.py`
2. Implement opcode logic in `execute_opcode()` method
3. Add tests in `tests/test_vm.py`

### Contributing

Contributions are welcome! Please ensure:
- All tests pass
- Code follows existing style
- New features include tests
- Documentation is updated

## License

MIT License - feel free to use this project for learning and development.

## Roadmap

- [ ] Network layer for P2P communication
- [ ] Merkle tree implementation for efficient verification
- [ ] Advanced consensus mechanisms (PoS, BFT)
- [ ] Contract ABI and high-level language compiler
- [ ] Web interface for blockchain explorer
- [ ] Performance optimizations
- [ ] Additional cryptographic primitives

## Technical Details

### Gas System
The VM implements a simple gas metering system to prevent infinite loops and resource exhaustion. Each operation consumes 1 gas unit, with a configurable gas limit per execution.

### State Model
The state manager maintains:
- Account balances and nonces
- Smart contract code
- Contract storage (key-value pairs)
- State snapshots for rollback capability

### Mining Algorithm
Proof-of-work mining with SHA-256 hash function. Difficulty is adjustable based on required leading zeros in block hash.

### Security Features
- ECDSA signatures for transaction authentication
- Merkle-tree compatible block structure
- Gas limits to prevent DoS
- Revert capability for failed transactions

---

**Built with ❤️ for Web3 developers**
