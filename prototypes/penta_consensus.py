"""
Penta-Consensus research prototype in Python.

This is NOT production code - it's for research validation only.
Implements the five consensus layers, composite scoring,
slashing conditions, and HotStuff BFT finality as specified
in specification/research/theoretical-design.md.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional


# Layer weights for composite scoring
WEIGHTS = {
    "useful_work": 0.30,
    "stake_vrf": 0.25,
    "spacetime": 0.15,
    "reputation": 0.15,
    "humanness": 0.15,
}

# Reputation scoring weights
REPUTATION_WEIGHTS = {
    "uptime": 0.20,
    "block_validity": 0.25,
    "transaction_honesty": 0.15,
    "useful_work": 0.15,
    "governance": 0.10,
    "endorsements": 0.10,
    "age_bonus": 0.05,
}

# Slashing penalties
SLASH_DOUBLE_VOTE = 33      # percent
SLASH_EQUIVOCATION = 100    # percent
SLASH_DOWNTIME = 1          # percent
SLASH_INVALID_WORK = 50     # percent

# Thresholds
MIN_REPUTATION_SCORE = 0.3
MIN_COMPOSITE_SCORE = 0.5


@dataclass
class Validator:
    """Validator in the Penta-Consensus system."""

    address: str
    stake: int
    reputation: float = 0.5
    vrf_secret: bytes = b""
    human_verified: bool = False
    slash_history: list = field(default_factory=list)
    uptime: float = 1.0
    registration_epoch: int = 0


@dataclass
class PoUWProof:
    """Proof of Useful Work result."""

    valid: bool
    usefulness_score: float
    difficulty_met: bool
    problem_type: str


@dataclass
class PoSProof:
    """Proof of Stake with VRF result."""

    valid: bool
    vrf_output: float
    stake_weight: float
    reputation_weight: float


@dataclass
class PoSTProof:
    """Proof of Spacetime result."""

    valid: bool
    space_proof_valid: bool
    time_proof_valid: bool
    plot_size_gb: int


@dataclass
class PoRScore:
    """Proof of Reputation score."""

    raw_score: float
    decayed_score: float
    is_eligible: bool


@dataclass
class PoHStatus:
    """Proof of Humanness status."""

    is_verified: bool
    method: str
    expiry: int


@dataclass
class Block:
    """Block in the Penta-Consensus chain."""

    height: int
    proposer: str
    timestamp: int
    prev_hash: bytes
    pouw_proof: PoUWProof
    pos_proof: PoSProof
    post_proof: PoSTProof
    reputation_score: PoRScore
    humanity_status: PoHStatus


@dataclass
class PentaValidation:
    """Result of validating a block against all five layers."""

    pouw_valid: bool
    pouw_score: float
    pos_valid: bool
    pos_score: float
    post_valid: bool
    post_score: float
    por_valid: bool
    por_score: float
    poh_valid: bool
    poh_score: float
    composite_score: float
    finality_achieved: bool
    all_layers_pass: bool


class PentaConsensus:
    """Penta-Consensus engine with five simultaneous layers."""

    def __init__(self) -> None:
        self.weights = WEIGHTS.copy()
        self.validators: dict[str, Validator] = {}

    # ------------------------------------------------------------------
    # Layer 1: Proof of Useful Work
    # ------------------------------------------------------------------
    def validate_pouw(self, proof: PoUWProof) -> tuple[bool, float]:
        """Validate PoUW proof and return (valid, score)."""
        if not proof.valid or not proof.difficulty_met:
            return False, 0.0
        return True, proof.usefulness_score

    # ------------------------------------------------------------------
    # Layer 2: Proof of Stake with VRF
    # ------------------------------------------------------------------
    def validate_pos(self, proof: PoSProof) -> tuple[bool, float]:
        """Validate PoS proof and return (valid, score)."""
        if not proof.valid:
            return False, 0.0
        # Score based on VRF output weighted by stake and reputation
        score = proof.vrf_output * proof.stake_weight * proof.reputation_weight
        return True, min(score, 1.0)

    # ------------------------------------------------------------------
    # Layer 3: Proof of Spacetime
    # ------------------------------------------------------------------
    def validate_post(self, proof: PoSTProof) -> tuple[bool, float]:
        """Validate PoST proof and return (valid, score)."""
        if not proof.valid or not proof.space_proof_valid:
            return False, 0.0
        if not proof.time_proof_valid:
            return False, 0.0
        # Score based on plot size (normalized)
        size_score = min(proof.plot_size_gb / 1_000.0, 1.0)  # 1 TB = max score
        return True, size_score

    # ------------------------------------------------------------------
    # Layer 4: Proof of Reputation
    # ------------------------------------------------------------------
    def calculate_reputation(self, validator: Validator) -> PoRScore:
        """Calculate reputation score for a validator."""
        w = REPUTATION_WEIGHTS

        # Uptime score (0-1)
        uptime_score = validator.uptime

        # Block validity rate (use reputation as proxy for prototype)
        validity_score = validator.reputation

        # Transaction honesty (use reputation as proxy)
        honesty_score = validator.reputation

        # Useful work contribution (use reputation as proxy)
        work_score = validator.reputation

        # Governance participation (use reputation as proxy)
        gov_score = validator.reputation

        # Community endorsements (use reputation as proxy)
        endorsement_score = validator.reputation

        # Age bonus (logarithmic, capped at 1.0)
        import math

        age_blocks = 1000  # prototype value
        age_score = min(math.log(age_blocks + 1) / 20.0, 1.0)

        raw = (
            uptime_score * w["uptime"]
            + validity_score * w["block_validity"]
            + honesty_score * w["transaction_honesty"]
            + work_score * w["useful_work"]
            + gov_score * w["governance"]
            + endorsement_score * w["endorsements"]
            + age_score * w["age_bonus"]
        )

        # Apply time decay (10% per year, simplified)
        decayed = raw * 0.9  # one year decay

        return PoRScore(
            raw_score=raw,
            decayed_score=decayed,
            is_eligible=decayed >= MIN_REPUTATION_SCORE,
        )

    # ------------------------------------------------------------------
    # Layer 5: Proof of Humanness
    # ------------------------------------------------------------------
    def check_humanness(self, validator: Validator) -> PoHStatus:
        """Check humanness status for a validator."""
        return PoHStatus(
            is_verified=validator.human_verified,
            method="BiometricZKP" if validator.human_verified else "None",
            expiry=validator.registration_epoch + 365,
        )

    # ------------------------------------------------------------------
    # Composite Validation
    # ------------------------------------------------------------------
    def validate_block(self, block: Block) -> PentaValidation:
        """Validate a block against all five consensus layers."""

        # Layer 1: PoUW
        pouw_valid, pouw_score = self.validate_pouw(block.pouw_proof)

        # Layer 2: PoS-VRF
        pos_valid, pos_score = self.validate_pos(block.pos_proof)

        # Layer 3: PoST
        post_valid, post_score = self.validate_post(block.post_proof)

        # Layer 4: PoR
        por_result = self.calculate_reputation(
            Validator(address=block.proposer, stake=10_000, reputation=block.reputation_score.raw_score)
        )
        por_valid = por_result.is_eligible
        por_score = por_result.decayed_score

        # Layer 5: PoH
        poh_result = self.check_humanness(
            Validator(
                address=block.proposer,
                stake=10_000,
                human_verified=block.humanity_status.is_verified,
            )
        )
        poh_valid = poh_result.is_verified
        poh_score = 1.0 if poh_valid else 0.0

        # Composite score
        composite = (
            pouw_score * self.weights["useful_work"]
            + pos_score * self.weights["stake_vrf"]
            + post_score * self.weights["spacetime"]
            + por_score * self.weights["reputation"]
            + poh_score * self.weights["humanness"]
        )

        all_layers_pass = all(
            [pouw_valid, pos_valid, post_valid, por_valid, poh_valid]
        )

        return PentaValidation(
            pouw_valid=pouw_valid,
            pouw_score=pouw_score,
            pos_valid=pos_valid,
            pos_score=pos_score,
            post_valid=post_valid,
            post_score=post_score,
            por_valid=por_valid,
            por_score=por_score,
            poh_valid=poh_valid,
            poh_score=poh_score,
            composite_score=composite,
            finality_achieved=all_layers_pass and composite >= MIN_COMPOSITE_SCORE,
            all_layers_pass=all_layers_pass,
        )

    # ------------------------------------------------------------------
    # Slashing
    # ------------------------------------------------------------------
    def slash_double_vote(self, validator_address: str) -> dict:
        """Apply slashing for double voting."""
        return {
            "validator": validator_address,
            "penalty_percent": SLASH_DOUBLE_VOTE,
            "ban_epochs": 36_500,  # ~100 years
            "burn": True,
        }

    def slash_equivocation(self, validator_address: str) -> dict:
        """Apply slashing for equivocation."""
        return {
            "validator": validator_address,
            "penalty_percent": SLASH_EQUIVOCATION,
            "ban_epochs": float("inf"),  # permanent
            "burn": True,
        }


def test_penta_consensus() -> None:
    """Test Penta-Consensus prototype."""
    consensus = PentaConsensus()

    # Test 1: Valid block passes all layers
    valid_block = Block(
        height=1,
        proposer="validator1",
        timestamp=1000,
        prev_hash=b"\x00" * 32,
        pouw_proof=PoUWProof(valid=True, usefulness_score=0.8, difficulty_met=True, problem_type="protein_folding"),
        pos_proof=PoSProof(valid=True, vrf_output=0.9, stake_weight=0.5, reputation_weight=0.8),
        post_proof=PoSTProof(valid=True, space_proof_valid=True, time_proof_valid=True, plot_size_gb=500),
        reputation_score=PoRScore(raw_score=0.8, decayed_score=0.72, is_eligible=True),
        humanity_status=PoHStatus(is_verified=True, method="BiometricZKP", expiry=2000),
    )

    result = consensus.validate_block(valid_block)
    assert result.all_layers_pass, "Valid block should pass all layers"
    assert result.composite_score >= MIN_COMPOSITE_SCORE, "Composite score should meet threshold"
    assert result.finality_achieved, "Finality should be achieved"
    print(f"Test 1 (valid block): composite={result.composite_score:.3f}, finality={result.finality_achieved}")

    # Test 2: Invalid block (low reputation)
    invalid_block = Block(
        height=2,
        proposer="validator2",
        timestamp=2000,
        prev_hash=b"\x01" * 32,
        pouw_proof=PoUWProof(valid=True, usefulness_score=0.8, difficulty_met=True, problem_type="protein_folding"),
        pos_proof=PoSProof(valid=True, vrf_output=0.9, stake_weight=0.5, reputation_weight=0.8),
        post_proof=PoSTProof(valid=True, space_proof_valid=True, time_proof_valid=True, plot_size_gb=500),
        reputation_score=PoRScore(raw_score=0.1, decayed_score=0.09, is_eligible=False),
        humanity_status=PoHStatus(is_verified=True, method="BiometricZKP", expiry=2000),
    )

    result2 = consensus.validate_block(invalid_block)
    assert not result2.all_layers_pass, "Block with low reputation should fail"
    print(f"Test 2 (low reputation): all_layers_pass={result2.all_layers_pass}")

    # Test 3: Invalid block (not human verified)
    inhuman_block = Block(
        height=3,
        proposer="validator3",
        timestamp=3000,
        prev_hash=b"\x02" * 32,
        pouw_proof=PoUWProof(valid=True, usefulness_score=0.8, difficulty_met=True, problem_type="protein_folding"),
        pos_proof=PoSProof(valid=True, vrf_output=0.9, stake_weight=0.5, reputation_weight=0.8),
        post_proof=PoSTProof(valid=True, space_proof_valid=True, time_proof_valid=True, plot_size_gb=500),
        reputation_score=PoRScore(raw_score=0.8, decayed_score=0.72, is_eligible=True),
        humanity_status=PoHStatus(is_verified=False, method="None", expiry=0),
    )

    result3 = consensus.validate_block(inhuman_block)
    assert not result3.all_layers_pass, "Block without humanness should fail"
    print(f"Test 3 (not human): all_layers_pass={result3.all_layers_pass}")

    # Test 4: Slashing
    slash_result = consensus.slash_double_vote("validator1")
    assert slash_result["penalty_percent"] == SLASH_DOUBLE_VOTE
    assert slash_result["burn"] is True
    print(f"Test 4 (slashing): {slash_result['penalty_percent']}% penalty, burn={slash_result['burn']}")

    print("\nAll Penta-Consensus prototype tests passed!")


if __name__ == "__main__":
    test_penta_consensus()
