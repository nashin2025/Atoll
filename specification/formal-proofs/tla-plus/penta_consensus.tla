\* Penta-Consensus TLA+ specification *
* Phase 0: Formal verification setup *

---------------------------- MODULE PentaConsensus ----------------------------

EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS
  Validators,          \* Set of validator addresses
  Layers,              \* {1, 2, 3, 4, 5} for the five consensus layers
  MinReputation,       \* 0.3
  MinComposite,        \* 0.5
  Weights              \* Layer weights

VARIABLES
  height,              \* Current block height
  proposer,            \* Current block proposer
  layerScores,         \* [layer |-> score] for current block
  compositeScore,      \* Composite score for current block
  finalizedBlocks,     \* Set of finalized block heights
  slashedValidators    \* Set of slashed validator addresses

---------------------------- Init ----------------------------

Init ==
  /\ height = 0
  /\ proposer = NULL
  /\ layerScores = [l \in Layers |-> 0]
  /\ compositeScore = 0
  /\ finalizedBlocks = {}
  /\ slashedValidators = {}

---------------------------- Helpers ----------------------------

AllLayersPass ==
  /\ layerScores[1] > 0      \* PoUW
  /\ layerScores[2] > 0      \* PoS-VRF
  /\ layerScores[3] > 0      \* PoST
  /\ layerScores[4] >= MinReputation  \* PoR
  /\ layerScores[5] = 1      \* PoH
  /\ compositeScore >= MinComposite

CompositeScore ==
  Weights[1] * layerScores[1] +
  Weights[2] * layerScores[2] +
  Weights[3] * layerScores[3] +
  Weights[4] * layerScores[4] +
  Weights[5] * layerScores[5]

---------------------------- ProposeBlock ----------------------------

ProposeBlock(v, scores) ==
  /\ v \in Validators
  /\ v \notin slashedValidators
  /\ scores \in [Layers -> Real]
  /\ proposer' = v
  /\ layerScores' = scores
  /\ compositeScore' = CompositeScore
  /\ height' = height + 1
  /\ UNCHANGED <<finalizedBlocks, slashedValidators>>

---------------------------- FinalizeBlock ----------------------------

FinalizeBlock ==
  /\ AllLayersPass
  /\ finalizedBlocks' = finalizedBlocks \cup {height}
  /\ UNCHANGED <<height, proposer, layerScores, compositeScore, slashedValidators>>

---------------------------- SlashValidator ----------------------------

SlashValidator(v) ==
  /\ v \in Validators
  /\ v \notin slashedValidators
  /\ slashedValidators' = slashedValidators \cup {v}
  /\ UNCHANGED <<height, proposer, layerScores, compositeScore, finalizedBlocks>>

---------------------------- Next ----------------------------

Next ==
  \/ \E v \in Validators, s \in [Layers -> Real] : ProposeBlock(v, s)
  \/ FinalizeBlock
  \/ \E v \in Validators : SlashValidator(v)

---------------------------- Safety Properties ----------------------------

\* No two conflicting blocks at the same height are finalized
NoConflictingFinality ==
  \A b1, b2 \in finalizedBlocks :
    b1 = b2

\* Composite score meets threshold when all layers pass
CompositeThreshold ==
  AllLayersPass => compositeScore >= MinComposite

\* Slashed validators cannot propose
SlashedCannotPropose ==
  \A v \in slashedValidators : proposer /= v

---------------------------- Liveness Properties ----------------------------

\* Eventually a block is finalized
EventualFinality ==
  <> (height > 0 /\ AllLayersPass)

=============================================================================
