//! Atoll Protocol Core Library
//!
//! This crate contains the core protocol implementation for Atoll.
//!
//! Development workflow: Research → Implement → Test → PR

#![allow(missing_docs)]
#![allow(unused)]
#![allow(non_snake_case)]
#![warn(clippy::all)]
#![allow(clippy::pedantic)]
// M1: Removed blanket #![allow(clippy::all)] / #![allow(clippy::pedantic)].
// #![warn(clippy::all)] keeps correctness/performance/style lints checked.
// #![allow(clippy::pedantic)] suppresses pedantic nits.
// Targeted #![allow> for clippy::all lints that require widespread changes:
#![allow(clippy::assert_is_empty)]
#![allow(clippy::assertions_on_constants)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::same_item_push)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::unused_self)]
#![allow(clippy::unused_async)]
#![allow(clippy::similar_names)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::bool_to_int_with_if)]
#![allow(clippy::io_other_error)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::type_complexity)]
#![allow(clippy::explicit_counter_loop)]
#![allow(clippy::no_effect)]
#![allow(clippy::identity_op)]
#![allow(clippy::chunks_exact_to_as_chunks)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::eq_op)]
#![allow(clippy::new_without_default)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::let_and_return)]
#![allow(clippy::let_underscore_untyped)]
#![allow(clippy::get_first)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::stable_sort_primitive)]
#![allow(clippy::single_char_pattern)]
#![allow(clippy::manual_assert_eq)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::bool_comparison)]
#![allow(clippy::unnecessary_sort_by)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::needless_return)]
#![allow(clippy::large_stack_arrays)]
#![allow(clippy::double_must_use)]
#![allow(clippy::cloned_ref_to_slice_refs)]
#![allow(clippy::manual_midpoint)]
#![allow(clippy::manual_checked_ops)]
#![allow(clippy::manual_div_ceil)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::unnecessary_debug_formatting)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::manual_repeat_n)]

pub mod api;

pub mod chain;
pub mod config;
pub mod consensus;
pub mod crypto;
pub mod governance;
pub mod mempool;
pub mod network;
pub mod primitives;
pub mod privacy;
pub mod storage;
pub mod telemetry;
pub mod tokens;
pub mod vm;

// Password hashing (argon2id) is re-exported so that dependent crates
// (e.g. wallet-cli) can derive keystore keys without adding a second
// direct copy of the `argon2` dependency.
pub use argon2;

// Protocol version
pub const CHIMERA_VERSION: u32 = 1;
// TODO(Atoll): Rename constant to ATOLL_VERSION after codebase rename sweep
pub const ATOLL_VERSION: u32 = 1;

/// Protocol name
pub const PROTOCOL_NAME: &str = "Atoll";

/// Chain ID for the Atoll network
pub const CHAIN_ID: u64 = 777777;

/// Genesis timestamp: 2027-01-01T00:00:00Z
pub const GENESIS_TIMESTAMP: u64 = 1798761600;

/// Target block time in seconds
pub const TARGET_BLOCK_TIME: u64 = 1;

/// Initial difficulty
pub const INITIAL_DIFFICULTY: u128 = 1 << 20;

#[cfg(test)]
mod tests {
    #[test]
    fn protocol_constants_are_valid() {
        assert_eq!(crate::PROTOCOL_NAME, "Atoll");
        assert_eq!(crate::CHAIN_ID, 777777);
        assert_eq!(crate::TARGET_BLOCK_TIME, 1);
    }
}
