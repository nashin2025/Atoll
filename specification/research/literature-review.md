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

### Byzantine Fault Tolerance (BFT)
- **PBFT** (1999): Practical Byzantine Fault Tolerance, f < n/3
- **HotStuff** (2018): Linear communication complexity BFT
- **Tendermint** (2014): PBFT variant for blockchain

**Sources:**
- [Blockchain Consensus Mechanisms: A Comprehensive Review](https://doi.org/10.3390/electronics14173567) (Electronics, 2025-09-08) — 200+ publication systematic review categorizing consensus into high throughput, strong security, low energy, flexible scaling
- [Comprehensive survey of blockchain consensus mechanisms](https://dl.acm.org/doi/10.1016/j.comnet.2025.111661) (Computer Networks, 2025-11-01) — State-of-the-art consensus survey with evaluation model
- [Current Status and Trends of Blockchain Consensus Mechanisms](https://doi.org/10.54691/9wtrd766) (2025-10-29) — Divides mechanisms into PoW, PoS, BFT; analyzes attack methods
- [Reaching Consensus in the Byzantine Empire](https://dl.acm.org/doi/10.1145/3636553) (ACM Computing Surveys, 2024-01-12) — Systematic survey of BFT algorithms with message/time complexity comparisons
- [Systematic Literature Review on Blockchain Consensus Security](https://www.techscience.com/csse/v48n6/58688) (CSSE, 2024-05-31) — 3749 papers reviewed, 290 analyzed; identifies gaps in scalability, privacy, interoperability

### DAG-Based Consensus
- **IOTA Tangle** (2015): DAG-based, no miners, no fees
- **Nano** (2015): Block-lattice, delegated PoS
- **Fantom** (2018): Lachesis, aBFT DAG consensus
- **Kaspa** (2021): GHOSTDAG, blockDAG with high throughput

**Sources:**
- [Performance Comparison of DAG-Based Distributed Ledgers](https://www.mdpi.com/2073-431X/12/12/257) (2023-10-13) — DAGs offer significantly higher throughput than linear blockchains; Fantom Lachesis and Avalanche Snowball use hybrid DAG-blockchain
- [SoK: DAG-based Blockchain Systems](https://doi.org/10.1145/3576899) (2022-12-20) — Six DAG design patterns evaluated on structure, consensus, property, security, performance
- [Chain or DAG? Underlying data structures in DLT](https://dl.acm.org/doi/10.1016/j.sysarc.2022.102720) (2022-10-01) — Taxonomy of chain-based and DAG-based DLT structures

### Hybrid Consensus
- **Decred** (2016): PoW + PoS hybrid
- **Ethereum 2.0**: PoS + sharding + beacon chain

**Sources:**
- [Hybrid Consensus Mechanisms in Blockchain](https://doi.org/10.1155/int/5821997) (2025-01-01) — Combines multiple consensus algorithms to leverage strengths and mitigate weaknesses
- [HybridChain](https://doi.org/10.1109/tpds.2024.3381593) (2024-03-26) — Combines sharded blockchain and DAG with decentralized learning consensus
- [Building Blocks of Sharding Blockchain Systems](https://ar5iv.labs.arxiv.org/html/2102.13364) — Sharding taxonomy: node selection, epoch randomness, node assignment, intra-shard consensus, cross-shard processing, shard reconfiguration, motivation mechanism

## 0.2 Hash Function Survey

### Sponge Construction
- **Keccak/SHA-3** (2015): NIST standard, sponge construction
- **KangarooTwelve** (2016): Keccak-based, tree hashing for parallelism

### Merkle-Damgård Construction
- **SHA-256** (2001): NIST standard, widely deployed (Bitcoin)
- **SHA-512** (2001): 512-bit variant of SHA-2
- **Whirlpool** (2000): Based on modified AES

### HAIFA Construction
- **BLAKE2b** (2012): Fast, parallelizable, SHA-3 finalist
- **BLAKE3** (2020): BLAKE2 successor, tree hashing, SIMD optimized

### SHA-3 Finalists (Not Selected)
- Skein, JH, Groestl, Fugue, Luffa, SHAvite, SIMD, ECHO, Hamsi, Shabal, BMW, CubeHash

### Memory-Hard Functions
- **Scrypt** (2009): Memory-hard, CPU-friendly; vulnerable to Time-Memory Tradeoff attacks
- **Argon2** (2015): Password Hashing Competition winner; Argon2id recommended for general use; resistant to GPU/ASIC attacks
- **Yescrypt** (2015): Scrypt variant; acceptable memory hardness for larger parameters
- **Lyra2** (2014): Sponge-based, memory-hard
- **Balloon Hashing** (2016): Memory-hard with data-independent access; resistant to TMTO attacks
- **RandomX** (2019): Monero's CPU-focused PoW, random code execution

**Sources:**
- [Hashing It Out: Analysis of Cryptographic Hash Functions](https://jonathanw.dev/public/2024/hashingitout.pdf) (2024) — Argon2 and scrypt recommended; memory-hard functions diminish ASIC acceleration advantage
- [On time-memory trade-offs for password hashing](https://www.frontiersin.org/journals/computer-science/articles/10.3389/fcomp.2024.1368362/full) (2024-02-29) — Argon2 and Balloon most resilient to TMTO attacks
- [Comprehensive Guide to Hashing Algorithms](https://jeffreytse.net/computer/2024/11/03/comprehensive-guide-to-hashing-algorithms.html) (2024-11-03) — BLAKE2 adopted by Argon2; blockchain uses SHA-256 (Bitcoin), Scrypt (Litecoin)
- [NIST CSRC Hash Functions](https://csrc.nist.gov/projects/hash-functions/) — FIPS 180-4 (SHA-1, SHA-2 family), FIPS 202 (SHA-3: SHA3-224/256/384/512, SHAKE128/256)
- [Comparison of cryptographic hash functions](https://en.wikipedia.org/wiki/Comparison_of_cryptographic_hash_functions) (2026-10-19) — BLAKE2b: 512-bit block, 12 rounds; BLAKE2s: 256-bit block, 10 rounds

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

**Sources:**
- [A systematic literature review of blockchain technology and energy efficiency](https://link.springer.com/article/10.1007/s44257-025-00041-6) (2025-09-29) — PoS and DAGs reduce energy use by 99%+ vs PoW; sharding and Layer-2 improve efficiency but add complexity
- [Overview and Comparison of Blockchain Consensus Mechanisms](https://link.springer.com/article/10.1007/s44227-025-00087-8) (2025-12-13) — Simulated comparison of PoW, PoS, DPoS, PoA, PoC; PoW excels in decentralization/security but costs highest energy
- [Hybrid Approach to Optimize and Secure Blockchain-based Systems](https://doi.org/10.1109/rusautocon65989.2025.11177330) (2025-09-07) — Hybrid miniblockchain + DAG with checkpoint mechanism and reputation system

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

1. **No production blockchain uses 5 simultaneous consensus layers** — Penta-Consensus fills this gap
2. **No hash function rotates algorithms per block with AI-evolved S-boxes** — Morph-Hash fills this gap
3. **No triple-token system with soulbound reputation + gas + value** — ATOLL/MIND/PROOF fills this gap
4. **No mainstream encrypted mempool at layer 1** — Atoll's encrypted mempool fills this gap
5. **No formal verification of consensus in Coq/TLA+/Isabelle for production chains** — Atoll's formal proofs fill this gap

## 0.6 Research Sources

### Consensus
- [Electronics 2025 consensus review](https://doi.org/10.3390/electronics14173567)
- [Computer Networks 2025 consensus survey](https://dl.acm.org/doi/10.1016/j.comnet.2025.111661)
- [ACM Computing Surveys BFT review (2024)](https://dl.acm.org/doi/10.1145/3636553)
- [CSSE consensus security SLR (2024)](https://www.techscience.com/csse/v48n6/58688)

### Hash Functions
- [Hashing It Out (2024)](https://jonathanw.dev/public/2024/hashingitout.pdf)
- [Frontiers TMTO attacks review (2024)](https://www.frontiersin.org/journals/computer-science/articles/10.3389/fcomp.2024.1368362/full)
- [NIST CSRC Hash Functions](https://csrc.nist.gov/projects/hash-functions/)

### Architecture
- [DAG performance comparison (MDPI, 2023)](https://www.mdpi.com/2073-431X/12/12/257)
- [SoK: DAG-based Blockchain Systems (2022)](https://doi.org/10.1145/3576899)
- [Sharding building blocks (arXiv)](https://ar5iv.labs.arxiv.org/html/2102.13364)
- [Energy efficiency SLR (Springer, 2025)](https://link.springer.com/article/10.1007/s44257-025-00041-6)
