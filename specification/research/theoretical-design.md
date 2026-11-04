# Phase 0: Foundation & Research - Theoretical Design

## Overview

Months 4-6 of Phase 0 focus on theoretical design based on the completed literature review. This document specifies the mathematical foundations of the Atoll Protocol's core innovations.

---

## 1. Morph-Hash Mathematical Specification

### 1.1 Design Goals

| Goal | Requirement |
|------|-------------|
| Output size | 512 bits |
| Internal state | 1600 bits (Keccak-style) |
| Security level | 256-bit collision / 512-bit preimage |
| Memory requirement | 4 GB (mining mode) |
| Algorithm pool | 32 variants |
| Active chain length | 16 algorithms per hash |
| S-box evolution | Every 10,000 blocks |
| Post-quantum finalization | Lattice-based (SWIFFT construction) |

### 1.2 Algorithm Pool Definition

The 32 algorithm variants are partitioned into six categories:

**Sponge-based (3):** Keccak512, SHA3_512, KangarooTwelve  
**Merkle-Damgård (3):** SHA512, Whirlpool, SHA512_256  
**HAIFA-based (2):** BLAKE2b, BLAKE3  
**Unique designs (13):** Skein512, JH512, Groestl512, Fugue512, Luffa512, SHAvite512, SIMD512, ECHO512, Hamsi512, Shabal512, BMW512, CubeHash512  
**Memory-hard (5):** Argon2id, Scrypt, Yescrypt, Lyra2Rev3, BalloonHash  
**CPU-hard (1):** RandomX  
**Custom ATOLL primitives (5):** AtollARX, AtollSponge, AtollFeistel, AtollSPN, AtollLattice, AtollTreeHash

### 1.3 Algorithm Selection Function

For each block, 16 of 32 algorithms are selected and chained:

```
Input:  prev_block_hash (512 bits), block_height (uint64)
Output: Ordered list of 16 algorithms

Step 1: Initialize indices = [0, 1, ..., 31]
Step 2: Fisher-Yates shuffle using prev_block_hash as entropy:
  For i from 31 down to 1:
    j = derive_index(prev_block_hash, i) % (i + 1)
    Swap indices[i] and indices[j]
    prev_block_hash = mix_seed(prev_block_hash, i)
Step 3: selected = indices[0..15]  (first 16 after shuffle)
Step 4: rotation = block_height % 16
Step 5: selected.rotate_left(rotation)
Step 6: Return [algorithm_pool[k] for k in selected]
```

**Total possible chains:** P(32,16) = 32! / 16! ≈ 2.7 × 10^23

### 1.4 Hash Computation Pipeline

```
morph_hash(input, prev_hash, height) -> Hash512

Phase 1: Chain Selection
  chain = select_algorithm_chain(prev_hash, height)

Phase 2: Memory Buffer Initialization
  buffer = init_memory_hard_buffer(prev_hash)  // 4 GB

Phase 3: Algorithmic Chain
  state = input
  For i from 0 to 15:
    algo = chain[i]
    mem_index = derive_memory_index(state, i)
    mem_data = read_memory(buffer, mem_index)
    state = xor_mix(state, mem_data)
    state = apply_algorithm(algo, state)
    state = apply_evolved_sbox(state, sbox_generation)

Phase 4: Post-Quantum Finalization
  state = lattice_finalize(state)

Phase 5: Final Squeeze
  Return state[0..64]  // 512 bits
```

### 1.5 Memory-Hard Component

**Buffer:** 4 GB initialized with BLAKE3 in counter mode, seeded with prev_block_hash  
**Mixing passes:** 3 passes of ARX-based diffusion  
**Access rounds:** 1024 random memory accesses per hash  
**Write-back:** Modified data written back to prevent time-memory tradeoffs

### 1.6 S-Box Evolution System

Every 10,000 blocks:
1. Generate 1,000 candidate S-boxes from on-chain entropy
2. Score each by cryptographic properties:
   - Nonlinearity (Walsh-Hadamard transform)
   - Differential uniformity
   - Algebraic degree
   - Strict Avalanche Criterion (SAC)
   - Bit Independence Criterion (BIC)
   - Fixed point count
3. Select top 8 S-boxes
4. Store generation number on-chain; all S-boxes verifiable

### 1.7 Post-Quantum Finalization Layer

Based on SWIFFT construction:
1. Convert state to polynomial coefficients
2. Apply Number Theoretic Transform (NTT)
3. Multiply by fixed public matrix A
4. Compress to 512 bits

**Security:** Resists Grover's algorithm (effective 256-bit quantum security)

---

## 2. Penta-Consensus Formal Model

### 2.1 Five Consensus Layers

Each block must pass all five layers for validation:

| Layer | Mechanism | Weight | Purpose |
|-------|-----------|--------|---------|
| 1 | Proof of Useful Work (PoUW) | 0.30 | Computationally useful problems |
| 2 | Proof of Stake with VRF (PoS-VRF) | 0.25 | Validator selection |
| 3 | Proof of Spacetime (PoST) | 0.15 | Storage provision |
| 4 | Proof of Reputation (PoR) | 0.15 | Participant trustworthiness |
| 5 | Proof of Humanness (PoH) | 0.15 | Sybil resistance |

### 2.2 Layer 1: Proof of Useful Work

**Problem types:** Protein folding, climate simulation, prime search, ML training, molecular docking, math proof verification, traditional PoW (fallback)

**Validation:**
1. Verify work was actually done
2. Score usefulness (0.0–1.0)
3. Check difficulty target met
4. Return composite work score

**Registration:** Research institutions submit problems; governance verifies submitter authorization and verification method soundness.

### 2.3 Layer 2: Proof of Stake with VRF

**Validator selection:**
```
For each eligible validator v:
  vrf_output, vrf_proof = VRF.evaluate(v.secret_key, epoch, slot, randomness)
  stake_weight = v.stake / total_stake
  rep_weight = v.reputation_score
  score = vrf_output × stake_weight × rep_weight
proposer = argmax(validators, score)
```

**Slashing conditions:**
- Double vote: 33% stake penalty, ~100 year ban
- Equivocation: 100% stake penalty, permanent ban
- Downtime (>1000 missed slots): 1% penalty, returned to pool
- Invalid work: 50% penalty, 1-year ban

### 2.4 Layer 3: Proof of Spacetime

**Requirements:** Minimum 100 GB storage, proof of replication, VDF time proof

**Verification:**
1. Space commitment valid (data stored)
2. Time proof valid (continuous storage)
3. Plot size ≥ 100 GB
4. Data uniqueness verified

**Challenge interval:** 30 seconds

### 2.5 Layer 4: Proof of Reputation

**Scoring weights:**
- Uptime: 0.20
- Block validity rate: 0.25
- Transaction honesty: 0.15
- Useful work contribution: 0.15
- Governance participation: 0.10
- Community endorsements: 0.10
- Age bonus: 0.05

**Decay:** Exponential decay at 10%/year  
**Minimum score:** 0.3 to participate

### 2.6 Layer 5: Proof of Humanness

**Verification methods:**
- Biometric ZKP (nullifier prevents double-registration)
- Social vouching (5+ existing human validators)
- Video verification with liveness detection
- Hardware attestation (TEE-based)
- Periodic reverification (every 365 days)

### 2.7 Composite Scoring

```
composite_score =
  pouw_score × 0.30 +
  pos_score  × 0.25 +
  post_score × 0.15 +
  por_score  × 0.15 +
  poh_score  × 0.15
```

All five layers must pass individually; composite score determines block priority.

### 2.8 Finality

HotStuff BFT provides finality after ~3 rounds (~3 seconds).  
Fault tolerance: f < n/3 Byzantine validators.

### 2.9 Fork Choice

GHOSTDAG-based fork choice with DAG structure.  
Beacon chain coordinates shards and finality checkpoints.

---

## 3. Security Proofs (Draft)

### 3.1 Consensus Safety

**Theorem:** No two honest validators finalize conflicting blocks at the same height.

**Proof sketch:**
- HotStuff BFT guarantees safety under partial synchrony when f < n/3
- All five Penta-Consensus layers must pass before block proposal
- Composite score threshold ensures only sufficiently validated blocks proceed
- Slashing conditions disincentivize equivocation

### 3.2 Consensus Liveness

**Theorem:** If >2/3 of validators are honest and the network is synchronous, new blocks are finalized.

**Proof sketch:**
- View change mechanism ensures progress when proposer is unavailable
- VRF-based proposer selection is unpredictable and fair
- PoST and PoR layers ensure continuous participation
- Emergency timelock allows governance intervention

### 3.3 Hash Function Security

**Theorem:** Morph-Hash is collision-resistant if at least one algorithm in the pool is collision-resistant.

**Proof sketch:**
- A collision in Morph-Hash requires a collision in the full chain of 16 algorithms
- If any algorithm in the chain is collision-resistant, finding a collision in that algorithm is infeasible
- Therefore finding a collision in the full chain is infeasible
- Preimage resistance follows similarly: inverting requires inverting all 16 algorithms

**Quantum security:** Grover's algorithm reduces 512-bit preimage to 256-bit effective security, still secure. Lattice-based finalization resists quantum attacks.

### 3.4 S-Box Evolution Security

**Theorem:** S-box evolution prevents static cryptanalysis.

**Proof sketch:**
- New S-boxes every 10,000 blocks (~2.7 hours at 1-second blocks)
- Attacker must break each S-box generation separately
- Minimum nonlinearity score enforced
- All S-boxes verifiable on-chain

---

## 4. Economic Model Simulations

### 4.1 Triple-Token System

| Token | Symbol | Purpose | Supply | Transferable |
|-------|--------|---------|--------|-------------|
| Atoll | ATOLL | Value store, governance | 21,000,000 (capped) | Yes |
| Mind | MIND | Gas/computation | Unlimited (minted + burned) | Yes |
| Proof | PROOF | Soulbound reputation | Unlimited (decays) | No |

### 4.2 ATOLL Emission Schedule

| Era | Blocks | Reward | Years | Cumulative |
|-----|--------|--------|-------|------------|
| 0 | 0–210,000 | 50 ATOLL | 0–4 | 10.5M |
| 1 | 210,001–420,000 | 25 ATOLL | 4–8 | 15.75M |
| 2 | 420,001–630,000 | 12.5 ATOLL | 8–12 | 18.375M |
| 3 | 630,001–840,000 | 6.25 ATOLL | 12–16 | 19.6875M |
| 4 | 840,001–1,050,000 | 3.125 ATOLL | 16–20 | 20.34375M |
| ... | ... | ... | ... | ... |
| ∞ | ~2140 | 0 | — | 21,000,000 |

**Halving interval:** 210,000 blocks (~1 year at 1-second blocks)

### 4.3 Block Reward Distribution

| Recipient | Percentage | Amount (Era 0) |
|-----------|------------|----------------|
| Block proposer (PoS-VRF winner) | 30% | 15 ATOLL |
| PoUW miner (useful work solver) | 25% | 12.5 ATOLL |
| PoST farmers (storage providers) | 15% | 7.5 ATOLL |
| Treasury/development fund | 15% | 7.5 ATOLL |
| Staking rewards pool | 10% | 5 ATOLL |
| Public goods / quadratic funding | 5% | 2.5 ATOLL |

### 4.4 Transaction Fee Distribution

- 70% burned (deflationary pressure on MIND)
- 20% to block proposer
- 10% to treasury

### 4.5 MIND Token Dynamics

- Minted: 100 MIND per block (decreasing rate)
- Burned: 70% of transaction fees
- Net: Deflationary if usage exceeds minting

### 4.6 PROOF Token Dynamics

**Award reasons:**
- Valid block: 10 PROOF
- Useful work solved: 50 PROOF
- Governance vote: 5 PROOF
- Storage contribution: 3 PROOF per block
- Humanity verified: 100 PROOF (one-time)
- Bug bounty: 1,000 PROOF
- Community contribution: 20 PROOF

**Decay:** 10% per year, applied continuously

### 4.7 Genesis Distribution

| Recipient | Amount | Percentage | Vesting |
|-----------|--------|------------|---------|
| Mining/staking rewards (future) | 18,900,000 ATOLL | 90% | Earned over time |
| Development treasury | 840,000 ATOLL | 4% | 4-year linear |
| Core team | 630,000 ATOLL | 3% | 1-year cliff + 4-year linear |
| Research grants | 315,000 ATOLL | 1.5% | Milestone-based |
| Bug bounty reserve | 210,000 ATOLL | 1% | As needed |
| Community airdrop | 105,000 ATOLL | 0.5% | To testnet participants |

**Total genesis:** 2,100,000 ATOLL (10% of supply)

---

## 5. Network Parameters

| Parameter | Value |
|-----------|-------|
| Chain ID | 777777 |
| Block time | 1 second |
| Epoch length | 32 slots |
| Slots per epoch | 32 |
| Finality | ~3 seconds (3 HotStuff rounds) |
| Max block size | 2 MB |
| Max transactions per block | 10,000 |
| Max gas per block | 100,000,000 |
| Genesis timestamp | 2027-01-01T00:00:00Z |
| Initial difficulty | 2^20 |
| Difficulty adjustment window | 720 blocks |
| Max difficulty adjustment | ±25% per window |

### Consensus Parameters

| Parameter | Value |
|-----------|-------|
| PoS minimum stake | 10,000 ATOLL |
| PoS max validators | 100,000 |
| PoST minimum storage | 100 GB |
| PoR minimum score | 0.3 |
| PoH reverification | Every 365 days |
| BFT fault tolerance | f < n/3 |
| Slashing (double vote) | 33% |
| Slashing (equivocation) | 100% |
| Validator cooldown | 7 days |

### Sharding Parameters

| Parameter | Value |
|-----------|-------|
| Initial shards | 64 |
| Max shards | 1,024 |
| Min validators per shard | 100 |
| Cross-shard timeout | 10 seconds |
| Rebalancing interval | 1,000 blocks |
| Shard split threshold | 150% average load |
| Shard merge threshold | 30% average load |

### Governance Parameters

| Parameter | Value |
|-----------|-------|
| Proposal deposit | 100 ATOLL |
| Voting period (standard) | 7 days |
| Voting period (constitutional) | 30 days |
| Quorum | 10% of PROOF holders |
| Approval threshold (standard) | 51% |
| Approval threshold (constitutional) | 75% |
| Timelock (standard) | 30 days |
| Timelock (constitutional) | 60 days |
| Emergency timelock | 1 day |

---

## 6. Security Properties Summary

| Property | Classical | Quantum (Grover) |
|----------|-----------|------------------|
| Preimage resistance | 512 bits | ~256 bits |
| Collision resistance | 2^256 | 2^128 |
| ASIC resistance | Algorithm rotation + 4GB memory | Same |
| Quantum signatures | Dilithium Level 5 | Post-quantum secure |
| Backup signatures | SPHINCS+ Level 5 | Post-quantum secure |
| Key encapsulation | Kyber | Post-quantum secure |

---

## 7. Next Steps

1. **Task 4:** Python prototypes of Morph-Hash and Penta-Consensus
2. **Task 5:** Academic paper submissions (CRYPTO, IEEE S&P, USENIX, EC)
3. **Task 6:** Formal verification setup (Coq, TLA+, Isabelle)
4. **Task 7:** Development roadmap finalization
5. **Task 8:** Phase 1 kickoff (core protocol development in Rust)
