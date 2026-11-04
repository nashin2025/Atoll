//! Virtual Machine module for the Atoll Protocol.
//!
//! Six VMs: EVM, WASM, Move, RISC-V, AI VM, ZK VM.

pub mod evm;
pub mod wasm;
pub mod move_vm;
pub mod riscv;
pub mod ai_vm;
pub mod zk_vm;

pub enum VMType {
    EVM,
    WASM,
    Move,
    RISCV,
    AI,
    ZK,
}

// Placeholder - full implementations after Phase 0 research
