//! Quadratic voting implementation for the Atoll Protocol.
//!
//! Vote weight is calculated as the square root of PROOF tokens staked,
//! giving smaller stakeholders relatively more influence than linear voting
//! while still weighting by economic stake.

use crate::primitives::address::Address;

/// Calculate quadratic voting weight from PROOF token balance.
///
/// Formula: weight = floor(sqrt(balance))
///
/// This ensures that doubling your stake does not double your voting power,
/// reducing whale dominance while still giving economic stakeholders a voice.
pub fn quadratic_weight(proof_balance: u128) -> u128 {
    if proof_balance == 0 {
        return 0;
    }
    // Integer square root
    let mut x = proof_balance;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + proof_balance / x) / 2;
    }
    x
}

/// Calculate the total voting weight for an address given their PROOF balance.
pub fn voting_weight(proof_balance: u128) -> u128 {
    quadratic_weight(proof_balance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadratic_weight_zero_balance() {
        assert_eq!(quadratic_weight(0), 0);
    }

    #[test]
    fn quadratic_weight_small_balances() {
        // sqrt(1) = 1
        assert_eq!(quadratic_weight(1), 1);
        // sqrt(4) = 2
        assert_eq!(quadratic_weight(4), 2);
        // sqrt(9) = 3
        assert_eq!(quadratic_weight(9), 3);
        // sqrt(16) = 4
        assert_eq!(quadratic_weight(16), 4);
    }

    #[test]
    fn quadratic_weight_sublinear() {
        // Doubling stake should less than double weight
        let w1 = quadratic_weight(100);   // sqrt(100) = 10
        let w2 = quadratic_weight(400);   // sqrt(400) = 20
        assert_eq!(w1, 10);
        assert_eq!(w2, 20);
        // 4x stake = 2x weight (not 4x)
        assert!(w2 < w1 * 4);
    }

    #[test]
    fn quadratic_weight_large_balance() {
        let w = quadratic_weight(1_000_000);
        assert_eq!(w, 1000); // sqrt(1,000,000) = 1000
    }

    #[test]
    fn quadratic_weight_monotonic() {
        for i in 1..1000 {
            let w1 = quadratic_weight(i);
            let w2 = quadratic_weight(i + 1);
            assert!(w2 >= w1, "weight should be non-decreasing: {} -> {}", w1, w2);
        }
    }
}
