//! Atoll Protocol Binary Entry Point

use atoll_core::PROTOCOL_NAME;

fn main() {
    println!("{} Protocol Node", PROTOCOL_NAME);
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Chain ID: {}", atoll_core::CHAIN_ID);
    println!("Target block time: {}s", atoll_core::TARGET_BLOCK_TIME);
}
