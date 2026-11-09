# Atoll Protocol — Litepaper

**Version:** 1.0  
**Date:** November 2026  
**Status:** Draft  
**Chain ID:** 777777

---

## Executive Summary

Atoll Protocol is a research-first, post-quantum Layer 1 blockchain designed for long-term security, scalability, and sovereignty. Atoll combines post-quantum cryptography (Dilithium, SPHINCS+, Kyber), a novel Penta-Consensus mechanism, 64-shard architecture, a triple-token economic model (ATOLL/MIND/PROOF), and six native virtual machines (EVM, WASM, Move, RISC-V, AI VM, ZK VM) into a single protocol stack.

This litepaper describes the Atoll Protocol architecture, token economics, governance model, and security posture. It is intended for developers, validators, researchers, and potential contributors.

---

## 1. Introduction

### 1.1 The Problem

Existing Layer 1 blockchains face three critical challenges:

1. **Quantum vulnerability**: Zero of the top 20 Layer 1 blockchains (holding approximately $2.25T combined) have deployed post-quantum cryptography on mainnet. A cryptographically relevant quantum computer could compromise user funds and chain integrity.
2. **Single points of failure**: Most L1s rely on a single consensus algorithm and single VM, limiting flexibility and resilience.
3. **Economic misalignment**: Single-token models conflate value storage, gas payment, and governance, creating misaligned incentives.

### 1.2 The Atoll Solution

Atoll addresses these challenges through:

| Challenge | Atoll Solution |
|-----------|---------------|
| Quantum vulnerability | Post-quantum from genesis (Dilithium signatures, SPHINCS+, Kyber KEM) |
| Consensus monoculture | Penta-Consensus (5-layer hybrid) |
| VM lock-in | Six native VMs (EVM, WASM, Move, RISC-V, AI VM, ZK VM) |
| Token conflation | Triple-token system (ATOLL/MIND/PROOF) |
| Sharding complexity | 64-shard architecture from genesis |

---

## 2. Architecture

### 2.1 Morph-Hash

Atoll's custom hash function, Morph-Hash, is designed for quantum resistance and efficient verification. It combines multiple hash rounds with post-quantum primitives to resist both classical and quantum preimage attacks.

### 2.2 Penta-Consensus

Penta-Consensus is a five-layer consensus architecture:

| Layer | Function |
|-------|----------|
| Layer 1 | Block proposal |
| Layer 2 | Block validation |
| Layer 3 | Cross-shard ordering |
| Layer 4 | Finality gadget |
| Layer 5 | Emergency circuit breaker |

### 2.3 Sharding

Atoll deploys 64 shards from genesis, enabling horizontal transaction throughput. Each shard maintains independent state with cross-shard communication via receipt-based messaging.

### 2.4 Privacy

Atoll implements encrypted mempool (threshold encryption), zk-STARKs for privacy-preserving proofs, and homomorphic encryption for private computation.

---

## 3. Triple-Token Economics

Atoll uses three distinct tokens, each with a specific purpose:

| Token | Symbol | Purpose | Transferable |
|-------|--------|---------|-------------|
| Atoll Token | ATOLL | Value store, governance, block rewards | Yes |
| Mind Token | MIND | Gas/computation payments, 70% fee burn | Yes |
| Proof Token | PROOF | Soulbound reputation, quadratic voting weight | No (soulbound) |

### 3.1 ATOLL Emission

- **Maximum supply**: 21,000,000 ATOLL
- **Initial block reward**: 50 ATOLL per block
- **Halving interval**: Every 210,000 blocks (~1 year at 1-second blocks)
- **Genesis allocation**: 2.1M ATOLL (10% of supply) to treasury, team, research, bounty, airdrop
- **Remaining 90%**: Earned through block rewards over time

### 3.2 Block Reward Distribution

| Recipient | Share |
|-----------|-------|
| Block Proposer | 30% |
| PoUW Miner | 25% |
| PoST Farmers | 15% |
| Treasury | 15% |
| Staking Rewards | 10% |
| Public Goods | 5% |

### 3.3 MIND Token

MIND is minted per block (100 MIND) and used for gas payments. 70% of transaction fees are burned, creating deflationary pressure.

### 3.4 PROOF Token

PROOF is a soulbound (non-transferable) reputation token awarded for:
- Valid block production: 10 PROOF
- Useful work solved: 50 PROOF
- Governance votes: 5 PROOF
- Storage contribution (per block): 3 PROOF
- Humanity verification: 100 PROOF (one-time)
- Bug bounty: 1,000 PROOF
- Community contribution: 20 PROOF

PROOF decays at 10% per year (continuously compounded) and determines quadratic voting weight: `weight = sqrt(balance)`.

---

## 4. Smart Contract Virtual Machines

Atoll natively supports six virtual machines:

| VM | Language | Use Case |
|----|----------|----------|
| EVM | Solidity | Ethereum-compatible dApps |
| WASM | Rust, C++, Go | General-purpose computation |
| Move | Move | Resource-oriented programming |
| RISC-V | Any compiled language | Low-level systems programming |
| AI VM | Python, TensorFlow | AI/ML inference on-chain |
| ZK VM | Circom, Noir | Zero-knowledge proof verification |

Each VM is simulated in the MVP and hardened for production in subsequent phases.

---

## 5. Governance

Atoll implements on-chain governance with:

- **Proposal deposit**: 100 ATOLL (refunded if passed, burned if rejected)
- **Voting periods**: 7 days (standard), 30 days (constitutional)
- **Approval thresholds**: 51% (standard), 75% (constitutional)
- **Quorum**: 10% of PROOF holders
- **Timelocks**: 30 days (standard), 60 days (constitutional), 1 day (emergency)
- **Vote weight**: Quadratic (sqrt of PROOF balance)

Proposal types:
- **Standard**: Parameter changes, treasury allocations
- **Constitutional**: Protocol upgrades, parameter changes affecting economics
- **Emergency**: Circuit breaker, pause functionality

---

## 6. Security Model

### 6.1 Post-Quantum Cryptography

Atoll uses post-quantum cryptography from genesis:
- **Signatures**: Dilithium2/3/5 (FIPS 204)
- **Hash**: SPHINCS+ (FIPS 205)
- **Key Encapsulation**: Kyber512/768/1024 (FIPS 203)

### 6.2 Security Audits

Atoll undergoes regular security audits covering:
- Automated SAST scanning
- STRIDE threat modeling
- Attack tree analysis
- Dependency vulnerability scanning

### 6.3 Emergency Controls

- **Circuit breaker**: Emergency pause functionality
- **Rate limiting**: 5 failed unlock attempts → 2-second delay
- **Error sanitization**: No internal details leaked in error messages
- **Zeroizing**: Secret keys zeroed from memory after use

---

## 7. Roadmap

| Phase | Focus | Status |
|-------|-------|--------|
| Phase 0 | Research & theoretical design | Complete |
| Phase 1 | Core protocol (Morph-Hash, consensus, sharding) | Complete |
| Phase 2 | Privacy & PQ cryptography | Complete |
| Phase 3 | Multi-VM implementation | Complete |
| Phase 4 | Testing & security hardening | Complete |
| Phase 5 | Mainnet preparation | Complete |
| Phase 6 | Testnet deployment | In Progress |
| Phase 7 | Mainnet launch | Planned |
| Phase 8 | Governance activation | Planned |
| Phase 9 | Tokenomics activation | Planned |

---

## 8. Disclaimer

This litepaper is for informational purposes only and does not constitute an offer to sell or solicitation of an offer to buy any securities or tokens. ATOLL, MIND, and PROOF tokens are utility tokens for use within the Atoll Protocol ecosystem. Cryptocurrency and blockchain technologies involve significant risks. Readers should conduct their own due diligence.

---

*Document version 1.0 — November 2026*
