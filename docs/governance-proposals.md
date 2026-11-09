# Atoll Protocol Governance Proposals

# Governance Proposals Documentation

This document describes the governance proposal system for the Atoll Protocol and provides examples of creating and executing proposals.

## Overview

Atoll Protocol implements on-chain governance with the following features:

- **Proposal creation**: Any address can create a proposal with a deposit
- **Quadratic voting**: Vote weight = sqrt(PROOF balance)
- **Voting periods**: 7 days (standard), 30 days (constitutional)
- **Approval thresholds**: 51% (standard), 75% (constitutional)
- **Quorum**: 10% of PROOF holders must participate
- **Timelock execution**: 30 days (standard), 60 days (constitutional), 1 day (emergency)

## Proposal Types

| Type | Approval Threshold | Voting Period | Timelock | Use Case |
|------|-------------------|---------------|----------|----------|
| Standard | 51% | 7 days | 30 days | Parameter changes, treasury allocation |
| Constitutional | 75% | 30 days | 60 days | Protocol upgrades, economics changes |
| Emergency | 51% | 1 day | 1 day | Circuit breaker, pause functionality |

## Proposal Lifecycle

1. **Creation**: Proposer creates a proposal with a 100 ATOLL deposit
2. **Voting**: PROOF holders vote with quadratic weight (sqrt of balance)
3. **Finalization**: After voting period, proposal passes or fails based on quorum and threshold
4. **Timelock**: Passed proposals wait through timelock before execution
5. **Execution**: Proposal is executed, deposit refunded

## Example Proposals

### Proposal 1: Adjust Block Reward

Propose reducing the block reward from 50 to 45 ATOLL:

```rust
use atoll_core::governance::{ProposalType, GovernanceEngine};
use crate::chain::state::ChainState;
use crate::primitives::address::Address;

let mut state = ChainState::new();
let mut engine = GovernanceEngine::new();

// Proposer creates proposal
let proposer = Address::from_bytes([1u8; 20]);
state.mint_atoll(proposer, 1000 * 10_u128.pow(8)).unwrap(); // 1000 ATOLL

let proposal_id = engine.create_proposal(
    &mut state,
    proposer,
    ProposalType::Standard,
    "Reduce block reward from 50 to 45 ATOLL".to_string(),
    0, // current block
).unwrap();

// Other PROOF holders vote
let voter1 = Address::from_bytes([2u8; 20]);
state.award_proof(voter1, 10000).unwrap(); // weight = 100
engine.vote(&state, proposal_id, voter1, true, 0).unwrap(); // Yes

let voter2 = Address::from_bytes([3u8; 20]);
state.award_proof(voter2, 2500).unwrap(); // weight = 50
engine.vote(&state, proposal_id, voter2, false, 0).unwrap(); // No

// Finalize after voting period (7 days = 604800 blocks)
let voting_end = ProposalType::Standard.voting_period_blocks();
let result = engine.finalize_voting(&mut state, proposal_id, voting_end).unwrap();
assert_eq!(result, ProposalState::Passed);

// Wait for timelock (30 days = 2592000 blocks)
let timelock_end = voting_end + ProposalType::Standard.timelock_blocks();
engine.execute_proposal(proposal_id, timelock_end).unwrap();
```

### Proposal 2: Constitutional Change (Parameter Adjustment)

Propose changing the halving interval:

```rust
let proposal_id = engine.create_proposal(
    &mut state,
    proposer,
    ProposalType::Constitutional,
    "Change halving interval from 210000 to 200000 blocks".to_string(),
    current_block,
).unwrap();

// Constitutional requires 75% approval and 30-day voting period
let voting_end = ProposalType::Constitutional.voting_period_blocks();
engine.finalize_voting(&mut state, proposal_id, voting_end).unwrap();

// Timelock is 60 days for constitutional changes
let timelock_end = voting_end + ProposalType::Constitutional.timelock_blocks();
engine.execute_proposal(proposal_id, timelock_end).unwrap();
```

### Proposal 3: Emergency Pause

Propose emergency pause of contract operations:

```rust
let proposal_id = engine.create_proposal(
    &mut state,
    proposer,
    ProposalType::Emergency,
    "Emergency pause: Contract vulnerability detected".to_string(),
    current_block,
).unwrap();

// Emergency: 1-day voting period, 1-day timelock
let voting_end = ProposalType::Emergency.voting_period_blocks();
engine.finalize_voting(&mut state, proposal_id, voting_end).unwrap();

let timelock_end = voting_end + ProposalType::Emergency.timelock_blocks();
engine.execute_proposal(proposal_id, timelock_end).unwrap();
```

## Running Governance Proposals

### Prerequisites

- Atoll node running (testnet or mainnet)
- PROOF tokens for voting weight
- ATOLL tokens for proposal deposit

### Creating a Proposal

1. **Connect wallet**: Unlock your wallet with ATOLL and PROOF tokens
2. **Create proposal**: Fill in proposal type, description, and parameters
3. **Pay deposit**: 100 ATOLL deposit is required
4. **Share proposal**: Share proposal ID with community for voting

### Voting on a Proposal

1. **Connect wallet**: Unlock with PROOF tokens
2. **Find proposal**: Browse active proposals on the governance page
3. **Vote**: Vote yes or no with your PROUT voting weight (sqrt of balance)
4. **Wait**: Voting period must elapse before finalization

### Executing a Proposal

1. **Wait for voting period**: Proposal must pass quorum and threshold
2. **Wait for timelock**: Timelock period must elapse (30-60 days)
3. **Execute**: Anyone can execute the proposal after timelock

## Sample Proposals for Mainnet Launch

The following proposals are recommended for mainnet launch:

### Proposal 1: Set Initial Block Reward

```
Title: Set initial block reward to 50 ATOLL
Type: Constitutional
Description: Set the initial block reward to 50 ATOLL per block with halving every 210,000 blocks.
Parameters: { "block_reward": 5000000000, "halving_interval": 210000 }
```

### Proposal 2: Set Initial Treasury Allocation

```
Title: Allocate 2.1M ATOLL to genesis treasury
Type: Standard
Description: Allocate 2.1M ATOLL (10% of supply) to genesis treasury addresses.
Parameters: { "treasury_address": "0x...", "amount": 2100000000000000000000 }
```

### Proposal 3: Set PROOF Token Parameters

```
Title: Configure PROOF token parameters
Type: Constitutional
Description: Set PROOF token award rates and decay parameters.
Parameters: { "valid_block": 10, "useful_work": 50, "governance_vote": 5, "storage_per_block": 3, "decay_rate": 0.10 }
```

## Running Proposals via CLI

The governance CLI allows creating and voting on proposals:

```bash
# Create a proposal
atoll-cli governance propose \
    --proposer <address> \
    --type standard \
    --description "Reduce block reward" \
    --passphrase <passphrase>

# Vote on a proposal
atoll-cli governance vote \
    --proposal-id <id> \
    --support true \
    --passphrase <passphrase>

# Execute a proposal (after timelock)
atoll-cli governance execute \
    --proposal-id <id> \
    --executor <address> \
    --passphrase <passphrase>

# List proposals
atoll-cli governance list

# Get proposal details
atoll-cli governance get --proposal-id <id>
```

## Best Practices

1. **Start with standard proposals**: Use constitutional for major changes only
2. **Allow sufficient voting time**: Don't rush constitutional changes
3. **Quorum matters**: Ensure enough PROOF holders participate
4. **Timelock awareness**: Remember timelock delays execution
5. **Deposit risk**: Proposal deposit is burned if rejected

## Security Considerations

1. **Proposal deposit**: Prevents spam proposals
2. **Quadratic voting**: Prevents whale dominance
3. **Timelock**: Allows time for community review before execution
4. **Emergency pause**: Can halt malicious proposals
5. **Multi-sig treasury**: Treasury funds require multi-sig approval
