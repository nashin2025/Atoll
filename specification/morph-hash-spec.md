# Morph-Hash Specification

## Overview

Morph-Hash is the cryptographic hash function used by the Atoll Protocol. It is a self-evolving hash function with rotating algorithm pools, AI-evolved S-boxes, memory-hard computation, and post-quantum finalization.

## Parameters

| Parameter | Value |
|-----------|-------|
| Output size | 512 bits |
| Internal state | 1600 bits |
| Security level | 256-bit collision / 512-bit preimage |
| Memory requirement | 4 GB (mining mode) |
| Algorithm pool size | 32 algorithms |
| Active chain length | 16 algorithms |
| S-box evolution interval | Every 10,000 blocks |

## Algorithm Pool

The 32 algorithm variants in the rotation pool:

### Sponge-based
- Keccak512
- SHA3_512
- KangarooTwelve

### Merkle-Damgård based
- SHA512
- Whirlpool
- SHA512_256

### HAIFA-based
- BLAKE2b
- BLAKE3

### Unique designs
- Skein512
- JH512
- Groestl512
- Fugue512
- Luffa512
- SHAvite512
- SIMD512
- ECHO512
- Hamsi512
- Shabal512
- BMW512
- CubeHash512

### Memory-hard
- Argon2id
- Scrypt
- Yescrypt
- Lyra2Rev3
- BalloonHash

### CPU-hard
- RandomX

### Custom ATOLL primitives
- AtollARX - Custom ARX cipher
- AtollSponge - Custom sponge construction
- AtollFeistel - Custom Feistel network
- AtollSPN - Custom SPN construction
- AtollLattice - Lattice-based hash
- AtollTreeHash - Custom tree hash

## Algorithm Selection

For each block, 16 of the 32 algorithms are selected and chained based on:
1. Previous block hash (as entropy for Fisher-Yates shuffle)
2. Block height (for additional rotation)

Total possible chains: P(32,16) ≈ 2.7 × 10^23

## Memory-Hard Component

- 4GB buffer initialized with BLAKE3 in counter mode
- 3 memory-hard mixing passes
- 1024 random memory accesses per hash
- Write-back modification to prevent time-memory tradeoffs

## Post-Quantum Finalization

Based on SWIFFT construction:
- Number Theoretic Transform (NTT)
- Multiplication by fixed public matrix
- Compression to 512 bits

## S-Box Evolution

Every 10,000 blocks:
1. Generate 1000 candidate S-boxes from on-chain entropy
2. Score by: nonlinearity, differential uniformity, algebraic degree, SAC, BIC, fixed point resistance
3. Select top 8 S-boxes
4. All S-boxes verifiable on-chain

## Security Properties

- **Preimage resistance**: 512 bits (classical), ~256 bits (quantum/Grover)
- **Collision resistance**: 2^256 birthday bound
- **ASIC resistance**: Algorithm changes every block + 4GB memory requirement
- **Quantum resistance**: Lattice-based finalization layer

## Test Vectors

Test vectors are generated during Phase 0 research and stored in `core/tests/`.
