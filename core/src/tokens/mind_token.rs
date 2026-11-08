//! MIND token implementation.
//!
//! MIND is the gas/computation token. It is minted per block and burned
//! from transaction fees (70% burn rate).

use crate::chain::state::ChainState;
use crate::primitives::address::Address;

/// MIND minted per block
pub const MIND_MINT_PER_BLOCK: u128 = 100 * 10_u128.pow(8);

/// Percentage of MIND fees burned (70%)
pub const MIND_FEE_BURN_PERCENT: u128 = 70;

/// Mints MIND tokens to an address
pub fn mint_mind(state: &mut ChainState, address: Address, amount: u128) -> Result<(), String> {
    state.mint_mind(address, amount)
}

/// Burns MIND tokens from an address
pub fn burn_mind(state: &mut ChainState, address: &Address, amount: u128) -> Result<(), String> {
    state.burn_mind(address, amount)
}

/// Burns 70% of MIND transaction fees.
///
/// Returns the amount burned.
pub fn burn_mind_fees(
    state: &mut ChainState,
    payer: &Address,
    mind_fees: u128,
) -> Result<u128, String> {
    let burn_amount = (mind_fees as u128)
        .checked_mul(MIND_FEE_BURN_PERCENT)
        .and_then(|v| v.checked_div(100))
        .unwrap_or(0);

    if burn_amount > 0 {
        burn_mind(state, payer, burn_amount)?;
    }

    Ok(burn_amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::address::Address;

    #[test]
    fn mind_mint_and_burn() {
        let mut state = crate::chain::state::ChainState::new();
        let addr = Address::from_bytes([1u8; 20]);

        mint_mind(&mut state, addr, 1000).unwrap();
        assert_eq!(state.mind_supply(), 1000);
        assert_eq!(state.mind_balance(&addr), 1000);

        burn_mind(&mut state, &addr, 300).unwrap();
        assert_eq!(state.mind_supply(), 700);
        assert_eq!(state.mind_balance(&addr), 700);
    }

    #[test]
    fn mind_burn_insufficient_balance() {
        let mut state = crate::chain::state::ChainState::new();
        let addr = Address::from_bytes([1u8; 20]);

        mint_mind(&mut state, addr, 100).unwrap();
        assert!(burn_mind(&mut state, &addr, 500).is_err());
        assert_eq!(state.mind_supply(), 100);
    }

    #[test]
    fn burn_mind_fees_70_percent() {
        let mut state = crate::chain::state::ChainState::new();
        let payer = Address::from_bytes([1u8; 20]);

        // Mint 1000 MIND to payer
        mint_mind(&mut state, payer, 1000).unwrap();

        // Burn 70% of 1000 = 700
        let burned = burn_mind_fees(&mut state, &payer, 1000).unwrap();
        assert_eq!(burned, 700);
        assert_eq!(state.mind_balance(&payer), 300);
        assert_eq!(state.mind_supply(), 300);
    }
}
