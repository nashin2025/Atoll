# Penta-Consensus Specification

## Overview

Penta-Consensus is the multi-layer consensus mechanism of the Atoll Protocol. Five consensus layers operate simultaneously, with HotStuff BFT providing finality.

## Consensus Layers

### Layer 1: Proof of Useful Work (PoUW)
Miners solve computationally useful problems:
- Protein structure prediction
- Climate simulation
- Prime number discovery
- ML training
- Molecular docking
- Mathematical proof verification
- Traditional PoW (fallback)

### Layer 2: Proof of Stake with VRF (PoS-VRF)
- Validators selected via VRF weighted by stake and reputation
- Minimum stake: 10,000 ATOLL
- Slashing: 33% for double-voting, 100% for equivocation

### Layer 3: Proof of Spacetime (PoST)
- Farmers provide storage (minimum 100 GB)
- Proof of replication + time (VDF)
- Challenge every 30 seconds

### Layer 4: Proof of Reputation (PoR)
- Score based on uptime, block validity, transaction honesty, useful work, governance, endorsements, age
- Minimum score: 0.3 to participate
- Time decay applied

### Layer 5: Proof of Humanness (PoH)
- Biometric ZKP, social vouching, video verification, hardware attestation, reverification
- Must re-verify every 365 days
- Nullifier prevents double-registration

## Composite Scoring

```
composite = pouw_score × 0.30
          + pos_score   × 0.25
          + post_score  × 0.15
          + por_score   × 0.15
          + poh_score   × 0.15
```

## Finality

HotStuff BFT provides finality after ~3 rounds (~3 seconds).

## Fork Choice

GHOSTDAG-based fork choice with DAG structure.

## Security

- Byzantine fault tolerance: f < n/3
- All five layers must pass for block validation
- Formal verification in Coq
