# Atoll Phase 0 Python Prototypes

## Overview

Research prototypes for Morph-Hash and Penta-Consensus algorithms.
These are NOT production code — they validate the theoretical design
from `specification/research/theoretical-design.md`.

## Files

| File | Description |
|------|-------------|
| `prototypes/morph_hash.py` | Morph-Hash hash function prototype |
| `prototypes/penta_consensus.py` | Penta-Consensus engine prototype |
| `prototypes/test_prototypes.py` | Test runner for all prototypes |

## Running the Tests

```bash
python prototypes/test_prototypes.py
```

## Morph-Hash Prototype

Implements:
- Algorithm selection via Fisher-Yates shuffle seeded with prev block hash
- 16 of 32 algorithms chained per hash
- Memory-hard mixing simulation (4 GB buffer in production)
- S-box evolution transform
- Post-quantum finalization (SHA-512 stand-in for SWIFFT)

### Test Vectors

1. Empty input at genesis height
2. Same input at different heights → different outputs (context sensitivity)
3. Deterministic output for same inputs
4. Avalanche effect (small input change → large output change)

## Penta-Consensus Prototype

Implements:
- Five consensus layers: PoUW, PoS-VRF, PoST, PoR, PoH
- Composite scoring with configurable weights
- Slashing conditions (double vote, equivocation)
- Block validation against all layers

### Test Scenarios

1. Valid block passes all layers
2. Block with low reputation fails
3. Block without humanness verification fails
4. Slashing penalty calculation

## Next Steps

After prototype validation:
1. Run statistical tests on hash function output
2. Validate game-theoretic assumptions
3. Submit academic papers
4. Begin formal verification in Coq/TLA+/Isabelle
5. Transition to Rust implementation (Phase 1)
