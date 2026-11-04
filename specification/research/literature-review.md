# Phase 0: Foundation & Research - Literature Review

## Overview

Phase 0 spans Months 1-3 and focuses on comprehensive literature review across all domains relevant to the Atoll Protocol.

## 0.1 Consensus Mechanisms Survey

### Proof of Work (PoW)
- **Bitcoin** (2008): SHA-256d, 10-minute blocks, 21M supply cap
- **Ethereum 1.0** (2015): Ethash, memory-hard ASIC resistance
- **Litecoin** (2011): Scrypt, memory-hard alternative to SHA-256

### Proof of Stake (PoS)
- **Peercoin** (2012): First PoS implementation, coin age mechanism
- **Ethereum 2.0** (2020): Beacon chain, Casper FFG finality, sharding
- **Cardano** (2017): Ouroboros, VRF-based slot leader selection
- **Solana** (2020): Proof of History + Tower BFT, parallel processing

### Byzantine Fault Tolerance
- **PBFT** (1999): Practical Byzantine Fault Tolerance, f < n/3
- **HotStuff** (2018): Linear communication complexity BFT
- **Tendermint** (2014): PBFT variant for blockchain

### DAG-Based Consensus
- **IOTA Tangle** (2015): DAG-based, no miners, no fees
- **Nano** (2015): Block-lattice, delegated PoS
- **Fantom** (2018): Lachesis, aBFT DAG consensus
- **Kaspa** (2021): GHOSTDAG, blockDAG with high throughput

### Hybrid Consensus
- **Decred** (2016): PoW + PoS hybrid
- **Ethereum 2.0**: PoS + sharding + beacon chain

## 0.2 Hash Function Survey

### Sponge Construction
- **Keccak/SHA-3** (2015): NIST standard, sponge construction
- **KangarooTwelve** (2016): Keccak-based, tree hashing for parallelism

### Merkle-Damgård Construction
- **SHA-256** (2001): NIST standard, widely deployed
- **SHA-512** (2001): 512-bit variant of SHA-2
- **Whirlpool** (2000): Based on modified AES

### HAIFA Construction
- **BLAKE2b** (2012): Fast, parallelizable, SHA-3 finalist
- **BLAKE3** (2020): BLAKE2 successor, tree hashing, SIMD optimized

### SHA-3 Finalists (Not Selected)
- Skein, JH, Groestl, Fugue, Luffa, SHAvite, SIMD, ECHO, Hamsi, Shabal, BMW, CubeHash

### Memory-Hard Functions
- **Scrypt** (2009): Memory-hard, CPU-friendly
- **Argon2** (2015): Password Hashing Competition winner
- **Yescrypt** (2015): Scrypt variant with additional features
- **Lyra2** (2014): Sponge-based, memory-hard
- **Balloon Hashing** (2016): Memory-hard with data-independent access
- **RandomX** (2019): Monero's CPU-focused PoW, random code execution

## 0.3 Blockchain Architecture Survey

### Monolithic Blockchains
- **Bitcoin**: Single chain, all nodes validate all transactions
- **Ethereum**: Single chain, EVM smart contracts

### Sharded Blockchains
- **Ethereum 2.0**: 64 shards, beacon chain, crosslinks
- **Polkadot**: Parachains, relay chain, shared security
- **Cosmos**: Zones, hubs, IBC protocol

### DAG-Blockchain Hybrids
- **Kaspa**: GHOSTDAG, blockDAG, high throughput
- **Conflux**: Tree-Graph, PoW + PoS hybrid
- **Fantom**: Lachesis DAG, aBFT

## 0.4 Known Attacks and Mitigations

| Attack | Description | Mitigation |
|--------|-------------|------------|
| 51% Attack | Majority hash power controls chain | Multiple consensus layers, economic cost |
| Selfish Mining | Miner withholds blocks | Fork choice rules, block withholding penalties |
| Eclipse Attack | Node isolated from honest peers | Diverse peer discovery, encrypted connections |
| Long-Range Attack | Historical chain rewritten | Checkpoints, finality gadgets |
| Nothing-at-Stake | Validators vote on multiple forks | Slashing conditions, deposit locks |
| Sybil Attack | Fake identities overwhelm network | Stake requirement, reputation system |
| MEV Extraction | Transaction reordering for profit | Encrypted mempool, threshold decryption |
| Quantum Attack | Shor's/Grover's break crypto | Post-quantum signatures, lattice-based hash |

## 0.5 Innovation Gaps Identified

1. **No production blockchain uses 5 simultaneous consensus layers**
2. **No hash function rotates algorithms per block with AI-evolved S-boxes**
3. **No triple-token system with soulbound reputation + gas + value**
4. **No mainstream encrypted mempool at layer 1**
5. **No formal verification of consensus in Coq/TLA+/Isabelle for production chains**

## 0.6 Research Sources

- Bitcoin Whitepaper (Nakamoto, 2008)
- Ethereum Whitepaper (Buterin, 2013)
- Algorand (Micali et al., 2017)
- HotStuff (Yin et al., 2018)
- GHOSTDAG (Sompolinsky et al., 2021)
- Argon2 (Biryukov et al., 2015)
- RandomX (Monero, 2019)
- Dilithium (NIST PQC standard, 2024)
- SWIFFT (Lyubashevsky et al., 2008)
