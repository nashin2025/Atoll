# Atoll Protocol Specification v1.0

## Overview

Atoll is a research-first cryptocurrency protocol. Every feature undergoes:
**Research → Implement → Test → PR**

## Core Components

### Consensus: Penta-Consensus
Five simultaneous consensus layers with HotStuff BFT finality:
1. Proof of Useful Work (PoUW)
2. Proof of Stake with VRF (PoS-VRF)
3. Proof of Spacetime (PoST)
4. Proof of Reputation (PoR)
5. Proof of Humanness (PoH)

### Hash: Morph-Hash
- 32 rotating cryptographic algorithms
- AI-evolved S-boxes every 10,000 blocks
- 4GB memory-hard component
- Post-quantum lattice-based finalization

### Architecture: Sharded DAG-Blockchain Hybrid
- Beacon chain + 64 shards (initial)
- GHOSTDAG fork choice rule
- Sub-second finality target
- 100,000+ TPS target

### Smart Contracts: 6 Virtual Machines
EVM, WASM, Move, RISC-V, AI VM, ZK VM

### Privacy: Full Stack
- zk-STARKs (transparent, post-quantum)
- Homomorphic encryption (BFV)
- Selective disclosure
- Encrypted mempool (MEV-resistant)

### Security: Post-Quantum
- Dilithium signatures (Level 5)
- SPHINCS+ backup signatures
- Kyber key encapsulation
- Formal verification in Coq/TLA+/Isabelle

### Governance: AI-Advised Hybrid DAO
- Quadratic voting
- Futarchy prediction markets
- On-chain constitution
- Time-locked execution

### Tokens: Triple-Token System
- ATOLL (CHM) - value store and governance
- MIND - gas/computation token
- PROOF - soulbound reputation (non-transferable)

## Network Parameters

| Parameter | Value |
|-----------|-------|
| Chain ID | 777777 |
| Block time | 1 second |
| Epoch length | 32 slots |
| Finality | ~3 seconds |
| Max block size | 2 MB |
| Max tx per block | 10,000 |
| Genesis timestamp | 2027-01-01T00:00:00Z |

## Token Parameters

| Parameter | Value |
|-----------|-------|
| ATOLL total supply | 21,000,000 |
| ATOLL decimals | 6 |
| ATOLL halving interval | 210,000 blocks |
| ATOLL initial block reward | 50 ATOLL |
| MIND burn rate | 70% |
| PROOF decay rate | 10%/year |

## Development Workflow

1. **Research** - Thorough research on the feature
2. **Implement** - Write the code
3. **Test** - Comprehensive testing (unit, integration, stress, adversarial, fuzzing)
4. **PR** - Pull request after successful testing and review

## License

Apache 2.0 + MIT dual license.
