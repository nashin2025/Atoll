# Atoll Protocol

A cryptocurrency protocol built on research-first principles. Every feature goes through:
**Research → Implement → Test → PR**

## Architecture Overview

- **Consensus**: Penta-Consensus (5 simultaneous layers + HotStuff BFT finality)
- **Hash**: Morph-Hash (32 rotating algorithms, AI-evolved S-boxes, memory-hard)
- **Architecture**: Sharded DAG-Blockchain hybrid with beacon chain
- **Smart Contracts**: 6 Virtual Machines (EVM, WASM, Move, RISC-V, AI VM, ZK VM)
- **Privacy**: zk-STARKs, homomorphic encryption, selective disclosure, encrypted mempool
- **Security**: Post-quantum signatures (Dilithium, SPHINCS+), formal verification
- **Governance**: AI-advised hybrid DAO with quadratic voting and futarchy
- **Tokens**: Triple-token system (ATOLL value, MIND gas, PROOF soulbound reputation)

## Development Workflow

1. **Research** - Phase 0: Foundation & Research
2. **Implement** - Build the feature
3. **Test** - Comprehensive testing before PR
4. **PR** - Pull request after successful testing

## Repository Structure

```
atoll-protocol/
├── specification/
├── core/
├── wallet/
├── explorer/
├── tools/
├── bridges/
├── docs/
├── scripts/
├── audits/
└── tests/
```

## License

Apache 2.0 + MIT dual license.
