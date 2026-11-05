//! Chain state management for the Atoll Protocol.
//!
//! Tracks account balances, nonces, and contract state across the chain.

use crate::primitives::address::Address;
use crate::primitives::hash::Hash512;
use std::collections::HashMap;

/// Account state
#[derive(Debug, Clone, Default)]
pub struct AccountState {
    pub balance: u128,
    pub nonce: u64,
    pub code_hash: Option<Hash512>,
    pub storage_root: Hash512,
}

/// Chain state: tracks all account states
pub struct ChainState {
    /// Account states by address
    accounts: HashMap<Address, AccountState>,
    /// Total supply of ATOLL
    total_supply: u128,
    /// Total MIND supply
    mind_supply: u128,
    /// Block number
    block_number: u64,
}

impl ChainState {
    /// Creates a new chain state
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            total_supply: 0,
            mind_supply: 0,
            block_number: 0,
        }
    }

    /// Returns the balance of an address
    pub fn balance(&self, address: &Address) -> u128 {
        self.accounts.get(address).map(|a| a.balance).unwrap_or(0)
    }

    /// Returns the nonce of an address
    pub fn nonce(&self, address: &Address) -> u64 {
        self.accounts.get(address).map(|a| a.nonce).unwrap_or(0)
    }

    /// Sets the balance of an address
    pub fn set_balance(&mut self, address: Address, balance: u128) {
        self.accounts.entry(address).or_default().balance = balance;
    }

    /// Increments the nonce of an address
    pub fn increment_nonce(&mut self, address: &Address) {
        self.accounts.entry(*address).or_default().nonce += 1;
    }

    /// Transfers balance from one address to another
    pub fn transfer(&mut self, from: &Address, to: &Address, amount: u128) -> Result<(), String> {
        let from_balance = self.balance(from);
        if from_balance < amount {
            return Err("Insufficient balance".to_string());
        }

        let new_from = from_balance
            .checked_sub(amount)
            .ok_or_else(|| "Balance subtraction overflow".to_string())?;
        let to_balance = self.balance(to);
        let new_to = to_balance
            .checked_add(amount)
            .ok_or_else(|| "Balance addition overflow".to_string())?;

        if from != to {
            self.set_balance(*from, new_from);
            self.set_balance(*to, new_to);
        }

        self.increment_nonce(from);

        Ok(())
    }

    /// Sets the code hash for a contract address
    pub fn set_code_hash(&mut self, address: Address, code_hash: Hash512) {
        self.accounts.entry(address).or_default().code_hash = Some(code_hash);
    }

    /// Gets the code hash for an address
    pub fn get_code_hash(&self, address: &Address) -> Option<&Hash512> {
        self.accounts
            .get(address)
            .and_then(|a| a.code_hash.as_ref())
    }

    /// Sets the storage root for an address
    pub fn set_storage_root(&mut self, address: Address, storage_root: Hash512) {
        self.accounts.entry(address).or_default().storage_root = storage_root;
    }

    /// Returns the storage root for an address
    pub fn get_storage_root(&self, address: &Address) -> Option<&Hash512> {
        self.accounts.get(address).map(|a| &a.storage_root)
    }

    /// Returns the total supply
    pub fn total_supply(&self) -> u128 {
        self.total_supply
    }

    /// Sets the total supply
    pub fn set_total_supply(&mut self, supply: u128) {
        self.total_supply = supply;
    }

    /// Returns the MIND supply
    pub fn mind_supply(&self) -> u128 {
        self.mind_supply
    }

    /// Sets the MIND supply
    pub fn set_mind_supply(&mut self, supply: u128) {
        self.mind_supply = supply;
    }

    /// Returns the current block number
    pub fn block_number(&self) -> u64 {
        self.block_number
    }

    /// Sets the block number
    pub fn set_block_number(&mut self, number: u64) {
        self.block_number = number;
    }

    /// Returns the number of accounts
    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    /// Returns an account state if it exists
    pub fn get_account(&self, address: &Address) -> Option<&AccountState> {
        self.accounts.get(address)
    }

    /// Mints new ATOLL tokens
    pub fn mint_atoll(&mut self, address: Address, amount: u128) -> Result<(), String> {
        let current = self.balance(&address);
        let new_balance = current
            .checked_add(amount)
            .ok_or_else(|| "Balance addition overflow".to_string())?;
        self.set_balance(address, new_balance);
        let new_total = self
            .total_supply
            .checked_add(amount)
            .ok_or_else(|| "Total supply addition overflow".to_string())?;
        self.total_supply = new_total;
        Ok(())
    }

    /// Burns ATOLL tokens
    pub fn burn_atoll(&mut self, address: &Address, amount: u128) -> Result<(), String> {
        let current = self.balance(address);
        if current < amount {
            return Err("Insufficient balance for burn".to_string());
        }
        let new_balance = current
            .checked_sub(amount)
            .ok_or_else(|| "Balance subtraction underflow".to_string())?;
        self.set_balance(*address, new_balance);
        let new_total = self
            .total_supply
            .checked_sub(amount)
            .ok_or_else(|| "Total supply subtraction underflow".to_string())?;
        self.total_supply = new_total;
        Ok(())
    }
}

impl Default for ChainState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_state_initialization() {
        let state = ChainState::new();
        assert_eq!(state.total_supply(), 0);
        assert_eq!(state.mind_supply(), 0);
        assert_eq!(state.block_number(), 0);
        assert_eq!(state.account_count(), 0);
    }

    #[test]
    fn balance_and_nonce() {
        let mut state = ChainState::new();
        let addr = Address::from_bytes([1u8; 20]);

        assert_eq!(state.balance(&addr), 0);
        assert_eq!(state.nonce(&addr), 0);

        state.set_balance(addr, 1000);
        assert_eq!(state.balance(&addr), 1000);

        state.increment_nonce(&addr);
        assert_eq!(state.nonce(&addr), 1);
    }

    #[test]
    fn transfer_success() {
        let mut state = ChainState::new();
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);

        state.set_balance(from, 1000);
        assert!(state.transfer(&from, &to, 500).is_ok());
        assert_eq!(state.balance(&from), 500);
        assert_eq!(state.balance(&to), 500);
        assert_eq!(state.nonce(&from), 1);
    }

    #[test]
    fn transfer_insufficient_balance() {
        let mut state = ChainState::new();
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);

        state.set_balance(from, 100);
        assert!(state.transfer(&from, &to, 500).is_err());
        assert_eq!(state.balance(&from), 100);
        assert_eq!(state.balance(&to), 0);
    }

    #[test]
    fn mint_and_burn() {
        let mut state = ChainState::new();
        let addr = Address::from_bytes([1u8; 20]);

        state.mint_atoll(addr, 1000).unwrap();
        assert_eq!(state.total_supply(), 1000);
        assert_eq!(state.balance(&addr), 1000);

        state.burn_atoll(&addr, 300).unwrap();
        assert_eq!(state.total_supply(), 700);
        assert_eq!(state.balance(&addr), 700);
    }

    #[test]
    fn burn_insufficient_balance() {
        let mut state = ChainState::new();
        let addr = Address::from_bytes([1u8; 20]);

        state.mint_atoll(addr, 100).unwrap();
        assert!(state.burn_atoll(&addr, 500).is_err());
        assert_eq!(state.total_supply(), 100);
    }

    #[test]
    fn code_and_storage() {
        let mut state = ChainState::new();
        let addr = Address::from_bytes([1u8; 20]);
        let code_hash = Hash512::from_bytes(&[42u8; 64]).unwrap();
        let storage_root = Hash512::from_bytes(&[99u8; 64]).unwrap();

        state.set_code_hash(addr, code_hash);
        state.set_storage_root(addr, storage_root);

        assert_eq!(state.get_code_hash(&addr), Some(&code_hash));
        assert_eq!(state.get_storage_root(&addr), Some(&storage_root));
    }
}
