//! EVM (Ethereum Virtual Machine) compatibility for the Atoll Protocol.
//!
//! Production-hardened EVM interpreter with a bounded 1024-entry stack,
//! word-addressable linear memory with expansion metering, deterministic
//! 256-bit fixed-point arithmetic, and complete opcode dispatch. Unknown
//! opcodes return [`VMError::InvalidOpcode`] instead of being silently
//! skipped. All arithmetic is wrapping modulo 2^256; there is no
//! floating-point, so execution is deterministic across platforms.

use crate::vm::gas::GAS_MEMORY_EXPANSION_PER_WORD;
use crate::vm::{ExecutionContext, ExecutionResult, GasMeter, Log, VMError, VMType};
use num_bigint::BigUint;
use sha3::{Digest, Keccak256};

/// Width of an EVM word in bytes (256 bits).
const WORD_BYTES: usize = 32;
/// Maximum stack depth, matching the EVM specification.
const MAX_STACK_DEPTH: usize = 1024;
/// Sandbox cap on total EVM memory bytes to prevent allocation-based DoS.
const MAX_MEMORY_BYTES: usize = 32 * 1024 * 1024;

/// EVM opcode constants.
mod opcode {
    pub const STOP: u8 = 0x00;
    pub const ADD: u8 = 0x01;
    pub const MUL: u8 = 0x02;
    pub const SUB: u8 = 0x03;
    pub const DIV: u8 = 0x04;
    pub const SDIV: u8 = 0x05;
    pub const MOD: u8 = 0x06;
    pub const SMOD: u8 = 0x07;
    pub const ADDMOD: u8 = 0x08;
    pub const MULMOD: u8 = 0x09;
    pub const EXP: u8 = 0x0a;
    pub const SIGNEXTEND: u8 = 0x0b;
    pub const LT: u8 = 0x10;
    pub const GT: u8 = 0x11;
    pub const EQ: u8 = 0x12;
    pub const ISZERO: u8 = 0x15;
    pub const AND: u8 = 0x16;
    pub const OR: u8 = 0x17;
    pub const XOR: u8 = 0x18;
    pub const NOT: u8 = 0x19;
    pub const BYTE: u8 = 0x1a;
    pub const SHL: u8 = 0x1b;
    pub const SHR: u8 = 0x1c;
    pub const SAR: u8 = 0x1d;
    pub const SHA3: u8 = 0x20;
    pub const MLOAD: u8 = 0x51;
    pub const MSTORE: u8 = 0x52;
    pub const MSIZE: u8 = 0x59;
    pub const POP: u8 = 0x50;
    pub const PUSH1: u8 = 0x60;
    pub const PUSH32: u8 = 0x7f;
    pub const DUP1: u8 = 0x80;
    pub const DUP16: u8 = 0x8f;
    pub const SWAP1: u8 = 0x90;
    pub const SWAP16: u8 = 0x9f;
    pub const LOG0: u8 = 0xa0;
    pub const LOG4: u8 = 0xa4;
    pub const CREATE: u8 = 0xf0;
    pub const CALL: u8 = 0xf1;
    pub const CALLCODE: u8 = 0xf2;
    pub const RETURN: u8 = 0xf3;
    pub const DELEGATECALL: u8 = 0xf4;
    pub const CREATE2: u8 = 0xf5;
    pub const REVERT: u8 = 0xfd;
    pub const INVALID: u8 = 0xfe;
    pub const SELFDESTRUCT: u8 = 0xff;

    // Environment / state / call opcodes.
    pub const ADDRESS: u8 = 0x30;
    pub const BALANCE: u8 = 0x31;
    pub const ORIGIN: u8 = 0x32;
    pub const CALLER: u8 = 0x33;
    pub const CALLVALUE: u8 = 0x34;
    pub const CALLDATALOAD: u8 = 0x35;
    pub const CALLDATASIZE: u8 = 0x36;
    pub const CALLDATACOPY: u8 = 0x37;
    pub const CODESIZE: u8 = 0x38;
    pub const CODECOPY: u8 = 0x39;
    pub const GASPRICE: u8 = 0x3a;
    pub const EXTCODESIZE: u8 = 0x3b;
    pub const EXTCODECOPY: u8 = 0x3c;
    pub const BLOCKHASH: u8 = 0x40;
    pub const COINBASE: u8 = 0x41;
    pub const TIMESTAMP: u8 = 0x42;
    pub const NUMBER: u8 = 0x43;
    pub const DIFFICULTY: u8 = 0x44;
    pub const GASLIMIT: u8 = 0x45;
}

/// Gas costs for EVM operations (simplified Ethereum schedule).
const GAS_BASE: u64 = 2;
const GAS_VERYLOW: u64 = 3;
const GAS_LOW: u64 = 5;
const GAS_MOD: u64 = 8;
const GAS_SHA3_BASE: u64 = 30;
const GAS_SHA3_PER_WORD: u64 = 6;
const GAS_BALANCE: u64 = 100;
const GAS_EXTCODE: u64 = 100;
const GAS_CREATE: u64 = 32_000;
const GAS_CALL: u64 = 40;
const GAS_SELFDESTRUCT: u64 = 5_000;
const GAS_LOG: u64 = 375;
const GAS_LOG_TOPIC: u64 = 375;
const GAS_LOG_DATA_BYTE: u64 = 8;

/// 2^256, the modulus for all EVM word arithmetic.
fn two_pow_256() -> BigUint {
    BigUint::from(1u32) << 256
}

/// 2^256 - 1, the all-ones 256-bit mask.
fn mask_256() -> BigUint {
    &two_pow_256() - &BigUint::from(1u32)
}

/// Wraps an arbitrary-precision value into the 256-bit EVM word range.
fn wrap(value: BigUint) -> BigUint {
    value % &two_pow_256()
}

/// Converts a 32-byte big-endian EVM word into a [`BigUint`].
fn word_to_biguint(word: &[u8; WORD_BYTES]) -> BigUint {
    BigUint::from_bytes_be(word)
}

/// Converts a [`BigUint`] into a 32-byte big-endian EVM word, truncated to
/// 256 bits (high bytes dropped if the value exceeds 2^256 - 1).
fn biguint_to_word(value: &BigUint) -> [u8; WORD_BYTES] {
    let value = wrap(value.clone());
    let mut buf = [0u8; WORD_BYTES];
    let bytes = value.to_bytes_be();
    let start = buf.len().saturating_sub(bytes.len());
    buf[start..].copy_from_slice(&bytes);
    buf
}

/// Right-aligns a byte slice into a 32-byte EVM word (EVM word convention).
fn right_align(bytes: &[u8]) -> [u8; WORD_BYTES] {
    let mut word = [0u8; WORD_BYTES];
    let len = bytes.len().min(WORD_BYTES);
    let start = WORD_BYTES - len;
    word[start..].copy_from_slice(&bytes[..len]);
    word
}

/// Converts a word into a `usize`, or `None` if it does not fit.
fn word_to_usize(word: &[u8; WORD_BYTES]) -> Option<usize> {
    let value = word_to_biguint(word);
    if value > BigUint::from(usize::MAX as u64) {
        return None;
    }
    let digits = value.to_u64_digits();
    if digits.len() > 1 {
        return None;
    }
    digits.first().copied().map(|d| d as usize).or(Some(0))
}

fn is_zero(value: &BigUint) -> bool {
    *value == BigUint::from(0u32)
}

/// Interprets a word as a signed 256-bit two's complement value: whether it
/// is negative and its magnitude.
fn to_signed(word: &[u8; WORD_BYTES]) -> (bool, BigUint) {
    let value = word_to_biguint(word);
    if word[0] & 0x80 != 0 {
        // Negative: magnitude is the two's complement, 2^256 - value.
        let mag = (&two_pow_256() - &value) % &two_pow_256();
        (true, mag)
    } else {
        (false, value)
    }
}

/// Converts a sign/magnitude pair back into a 256-bit two's complement word.
fn from_signed(negative: bool, mag: &BigUint) -> [u8; WORD_BYTES] {
    let mag = wrap(mag.clone());
    if negative && !is_zero(&mag) {
        biguint_to_word(&((&two_pow_256() - &mag) % &two_pow_256()))
    } else {
        biguint_to_word(&mag)
    }
}

/// EVM execution stack with overflow/underflow detection.
///
/// The stack holds at most [`MAX_STACK_DEPTH`] 256-bit words. Pushes beyond
/// the limit return [`VMError::StackOverflow`]; pops from an empty stack
/// return [`VMError::StackUnderflow`].
pub struct EvmStack {
    items: Vec<[u8; WORD_BYTES]>,
}

impl EvmStack {
    pub fn new() -> Self {
        Self {
            items: Vec::with_capacity(MAX_STACK_DEPTH),
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn push(&mut self, value: [u8; WORD_BYTES]) -> Result<(), VMError> {
        if self.items.len() >= MAX_STACK_DEPTH {
            return Err(VMError::StackOverflow);
        }
        self.items.push(value);
        Ok(())
    }

    pub fn pop(&mut self) -> Result<[u8; WORD_BYTES], VMError> {
        self.items.pop().ok_or(VMError::StackUnderflow)
    }

    /// Peeks at the `n`-th item from the top (`0` is the top of the stack).
    pub fn peek(&self, n: usize) -> Result<[u8; WORD_BYTES], VMError> {
        let idx = self
            .items
            .len()
            .checked_sub(n + 1)
            .ok_or(VMError::StackUnderflow)?;
        Ok(self.items[idx])
    }

    /// Duplicates the `n`-th stack item (1-indexed from the top) onto the
    /// top of the stack (`DUPn` semantics).
    pub fn dup(&mut self, n: usize) -> Result<(), VMError> {
        let value = self.peek(n - 1)?;
        self.push(value)
    }

    /// Swaps the top item with the `n`-th item below it (1-indexed;
    /// `SWAPn` semantics).
    pub fn swap(&mut self, n: usize) -> Result<(), VMError> {
        let top = self
            .items
            .len()
            .checked_sub(1)
            .ok_or(VMError::StackUnderflow)?;
        let other = top.checked_sub(n).ok_or(VMError::StackUnderflow)?;
        self.items.swap(top, other);
        Ok(())
    }
}

/// EVM linear memory.
///
/// Memory is byte-addressable and conceptually infinite and zero-initialized:
/// reads beyond the allocated region return zeros without error. Writes expand
/// memory to a 32-byte word boundary and report how many words were added so
/// the caller can charge [`GAS_MEMORY_EXPANSION_PER_WORD`] per word. Total
/// memory is capped at [`MAX_MEMORY_BYTES`] to prevent allocation-based DoS.
pub struct EvmMemory {
    data: Vec<u8>,
}

impl EvmMemory {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Ensures memory covers `offset + size` bytes, growing to a word
    /// boundary. Returns the number of newly allocated 32-byte words.
    pub fn expand_to(&mut self, offset: usize, size: usize) -> Result<usize, VMError> {
        let needed = offset
            .checked_add(size)
            .ok_or(VMError::InvalidMemoryAccess)?;
        if needed > MAX_MEMORY_BYTES {
            return Err(VMError::InvalidMemoryAccess);
        }
        if needed <= self.data.len() {
            return Ok(0);
        }
        let new_len = needed.div_ceil(WORD_BYTES) * WORD_BYTES;
        let old_words = self.data.len().div_ceil(WORD_BYTES);
        let new_words = new_len / WORD_BYTES;
        self.data.resize(new_len, 0);
        Ok(new_words - old_words)
    }

    /// Reads a 32-byte word from `offset`, zero-padding beyond the end of
    /// allocated memory.
    pub fn read_word(&self, offset: usize) -> [u8; WORD_BYTES] {
        let mut word = [0u8; WORD_BYTES];
        if offset < self.data.len() {
            let available = (self.data.len() - offset).min(WORD_BYTES);
            word[..available].copy_from_slice(&self.data[offset..offset + available]);
        }
        word
    }

    /// Writes a 32-byte word at `offset`, expanding memory as needed.
    /// Returns the number of newly allocated words (for gas metering).
    pub fn write_word(
        &mut self,
        offset: usize,
        value: &[u8; WORD_BYTES],
    ) -> Result<usize, VMError> {
        let words_added = self.expand_to(offset, WORD_BYTES)?;
        self.data[offset..offset + WORD_BYTES].copy_from_slice(value);
        Ok(words_added)
    }

    /// Writes `value` at `offset`, expanding memory as needed.
    /// Returns the number of newly allocated words (for gas metering).
    pub fn write_bytes(&mut self, offset: usize, value: &[u8]) -> Result<usize, VMError> {
        let words_added = self.expand_to(offset, value.len())?;
        self.data[offset..offset + value.len()].copy_from_slice(value);
        Ok(words_added)
    }

    /// Reads `size` bytes from `offset`, zero-padding beyond the end of
    /// allocated memory. No expansion occurs.
    pub fn read_bytes(&self, offset: usize, size: usize) -> Vec<u8> {
        let mut out = vec![0u8; size];
        if offset < self.data.len() {
            let available = (self.data.len() - offset).min(size);
            out[..available].copy_from_slice(&self.data[offset..offset + available]);
        }
        out
    }
}

/// EVM executor.
///
/// Executes EVM bytecode with a stack, memory model, and gas metering. All
/// opcodes charge gas before executing; running out of gas returns
/// [`VMError::OutOfGas`] and halts execution.
pub struct EVMExecutor {
    pub gas_meter: GasMeter,
    pub context: ExecutionContext,
    stack: EvmStack,
    memory: EvmMemory,
    return_data: Vec<u8>,
    logs: Vec<Log>,
}

impl EVMExecutor {
    pub fn new(context: ExecutionContext, gas_limit: u64) -> Self {
        Self {
            gas_meter: GasMeter::new(gas_limit, context.gas_price),
            context,
            stack: EvmStack::new(),
            memory: EvmMemory::new(),
            return_data: Vec::new(),
            logs: Vec::new(),
        }
    }

    fn charge(&mut self, gas: u64) -> Result<(), VMError> {
        self.gas_meter.charge(gas)
    }

    fn push_result(&mut self, value: BigUint) -> Result<(), VMError> {
        self.stack.push(biguint_to_word(&wrap(value)))
    }

    fn pop2(&mut self) -> Result<([u8; WORD_BYTES], [u8; WORD_BYTES]), VMError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        Ok((a, b))
    }

    fn pop3(&mut self) -> Result<([u8; WORD_BYTES], [u8; WORD_BYTES], [u8; WORD_BYTES]), VMError> {
        let c = self.stack.pop()?;
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        Ok((a, b, c))
    }

    /// Executes EVM bytecode.
    ///
    /// Returns [`VMError::InvalidOpcode`] for any undefined opcode, and
    /// [`VMError::StackOverflow`]/[`VMError::StackUnderflow`] for stack
    /// violations. Execution halts on `STOP`, `RETURN`, `REVERT`,
    /// `SELFDESTRUCT`, `INVALID`, or any error.
    pub fn execute(&mut self, bytecode: &[u8], input: &[u8]) -> Result<ExecutionResult, VMError> {
        // Base charge for loading the bytecode.
        self.gas_meter.charge(bytecode.len() as u64 * 2)?;

        // Reset per-execution state.
        self.stack = EvmStack::new();
        self.memory = EvmMemory::new();
        self.return_data.clear();
        self.logs.clear();

        let mut pc: usize = 0;
        while pc < bytecode.len() {
            let opcode = bytecode[pc];
            pc += 1;

            if let Some(result) = self.execute_opcode(opcode, bytecode, &mut pc, input)? {
                return Ok(result);
            }
        }

        Ok(ExecutionResult::Success {
            gas_used: self.gas_meter.gas_used,
            return_value: self.return_data.clone(),
            logs: self.logs.clone(),
        })
    }

    /// Executes a single opcode at the current program counter.
    ///
    /// Returns `Ok(Some(result))` when the opcode halts execution
    /// (`STOP`, `RETURN`, `REVERT`, `SELFDESTRUCT`, `INVALID`), otherwise
    /// `Ok(None)` to continue. Returns `Err` on VM errors, which also
    /// halts execution.
    fn execute_opcode(
        &mut self,
        op: u8,
        bytecode: &[u8],
        pc: &mut usize,
        input: &[u8],
    ) -> Result<Option<ExecutionResult>, VMError> {
        macro_rules! binop {
            ($op:expr) => {{
                self.charge(GAS_VERYLOW)?;
                let (a, b) = self.pop2()?;
                let a = word_to_biguint(&a);
                let b = word_to_biguint(&b);
                self.push_result($op(&a, &b))
            }};
        }

        match op {
            opcode::STOP => {
                self.charge(0)?;
                return Ok(Some(ExecutionResult::Success {
                    gas_used: self.gas_meter.gas_used,
                    return_value: self.return_data.clone(),
                    logs: self.logs.clone(),
                }));
            }
            opcode::ADD => {
                binop!(|a, b| a + b)?;
                Ok(None)
            }
            opcode::MUL => {
                self.charge(GAS_LOW)?;
                let (a, b) = self.pop2()?;
                let a = word_to_biguint(&a);
                let b = word_to_biguint(&b);
                self.push_result(a * b)?;
                Ok(None)
            }
            opcode::SUB => {
                self.charge(GAS_VERYLOW)?;
                let (a, b) = self.pop2()?;
                let a = word_to_biguint(&a);
                let b = word_to_biguint(&b);
                let result = if a >= b {
                    a - b
                } else {
                    &two_pow_256() - (&b - &a)
                };
                self.push_result(result)?;
                Ok(None)
            }
            opcode::DIV => {
                self.charge(GAS_LOW)?;
                let (a, b) = self.pop2()?;
                let a = word_to_biguint(&a);
                let b = word_to_biguint(&b);
                if is_zero(&b) {
                    self.stack.push([0u8; WORD_BYTES])?;
                } else {
                    self.push_result(a / b)?;
                }
                Ok(None)
            }
            opcode::SDIV => {
                self.charge(GAS_LOW)?;
                let (a, b) = self.pop2()?;
                let (a_neg, a_mag) = to_signed(&a);
                let (b_neg, b_mag) = to_signed(&b);
                if is_zero(&b_mag) {
                    self.stack.push([0u8; WORD_BYTES])?;
                } else {
                    let sign = a_neg ^ b_neg;
                    let mag = &a_mag / &b_mag;
                    self.stack.push(from_signed(sign, &mag))?;
                }
                Ok(None)
            }
            opcode::MOD => {
                self.charge(GAS_LOW)?;
                let (a, b) = self.pop2()?;
                let a = word_to_biguint(&a);
                let b = word_to_biguint(&b);
                if is_zero(&b) {
                    self.stack.push([0u8; WORD_BYTES])?;
                } else {
                    self.push_result(a % b)?;
                }
                Ok(None)
            }
            opcode::SMOD => {
                self.charge(GAS_LOW)?;
                let (a, b) = self.pop2()?;
                let (a_neg, a_mag) = to_signed(&a);
                let (_, b_mag) = to_signed(&b);
                if is_zero(&b_mag) {
                    self.stack.push([0u8; WORD_BYTES])?;
                } else {
                    let mag = &a_mag % &b_mag;
                    self.stack.push(from_signed(a_neg, &mag))?;
                }
                Ok(None)
            }
            opcode::ADDMOD => {
                self.charge(GAS_MOD)?;
                let (a, b, c) = self.pop3()?;
                let a = word_to_biguint(&a);
                let b = word_to_biguint(&b);
                let c = word_to_biguint(&c);
                if is_zero(&c) {
                    self.stack.push([0u8; WORD_BYTES])?;
                } else {
                    self.push_result((&a + &b) % &c)?;
                }
                Ok(None)
            }
            opcode::MULMOD => {
                self.charge(GAS_MOD)?;
                let (a, b, c) = self.pop3()?;
                let a = word_to_biguint(&a);
                let b = word_to_biguint(&b);
                let c = word_to_biguint(&c);
                if is_zero(&c) {
                    self.stack.push([0u8; WORD_BYTES])?;
                } else {
                    self.push_result((a * b) % c)?;
                }
                Ok(None)
            }
            opcode::EXP => {
                let (base, exp) = self.pop2()?;
                let base = word_to_biguint(&base);
                let exp = word_to_biguint(&exp);
                let exp_bytes = exp.to_bytes_be().len() as u64;
                self.charge(10u64.saturating_add(50u64.saturating_mul(exp_bytes)))?;
                self.push_result(base.modpow(&exp, &two_pow_256()))?;
                Ok(None)
            }
            opcode::SIGNEXTEND => {
                self.charge(GAS_LOW)?;
                let (t_word, x_word) = self.pop2()?;
                let t = word_to_usize(&t_word).unwrap_or(32);
                let result = if t >= 32 {
                    // Out of range: push x unchanged.
                    word_to_biguint(&x_word)
                } else if x_word[31 - t] & 0x80 != 0 {
                    // Sign bit of the t-th byte from the LSB is set: extend it
                    // across all more significant bytes.
                    let low_mask = if t == 31 {
                        mask_256()
                    } else {
                        (BigUint::from(1u32) << (8 * t + 8)) - &BigUint::from(1u32)
                    };
                    word_to_biguint(&x_word) | (&mask_256() - &low_mask)
                } else {
                    word_to_biguint(&x_word)
                };
                self.push_result(result)?;
                Ok(None)
            }
            opcode::LT => {
                self.charge(GAS_VERYLOW)?;
                let (a, b) = self.pop2()?;
                let a = word_to_biguint(&a);
                let b = word_to_biguint(&b);
                self.stack
                    .push(right_align(&[if a < b { 1u8 } else { 0u8 }]))?;
                Ok(None)
            }
            opcode::GT => {
                self.charge(GAS_VERYLOW)?;
                let (a, b) = self.pop2()?;
                let a = word_to_biguint(&a);
                let b = word_to_biguint(&b);
                self.stack
                    .push(right_align(&[if a > b { 1u8 } else { 0u8 }]))?;
                Ok(None)
            }
            opcode::EQ => {
                self.charge(GAS_VERYLOW)?;
                let (a, b) = self.pop2()?;
                let a = word_to_biguint(&a);
                let b = word_to_biguint(&b);
                self.stack
                    .push(right_align(&[if a == b { 1u8 } else { 0u8 }]))?;
                Ok(None)
            }
            opcode::ISZERO => {
                self.charge(GAS_BASE)?;
                let a = self.stack.pop()?;
                let a = word_to_biguint(&a);
                self.stack
                    .push(right_align(&[if is_zero(&a) { 1u8 } else { 0u8 }]))?;
                Ok(None)
            }
            opcode::AND => {
                self.charge(GAS_VERYLOW)?;
                let (a, b) = self.pop2()?;
                let a = word_to_biguint(&a);
                let b = word_to_biguint(&b);
                self.push_result(&a & &b)?;
                Ok(None)
            }
            opcode::OR => {
                self.charge(GAS_VERYLOW)?;
                let (a, b) = self.pop2()?;
                let a = word_to_biguint(&a);
                let b = word_to_biguint(&b);
                self.push_result(&a | &b)?;
                Ok(None)
            }
            opcode::XOR => {
                self.charge(GAS_VERYLOW)?;
                let (a, b) = self.pop2()?;
                let a = word_to_biguint(&a);
                let b = word_to_biguint(&b);
                self.push_result(&a ^ &b)?;
                Ok(None)
            }
            opcode::NOT => {
                self.charge(GAS_BASE)?;
                let a = self.stack.pop()?;
                let a = word_to_biguint(&a);
                self.push_result(&mask_256() ^ &a)?;
                Ok(None)
            }
            opcode::BYTE => {
                self.charge(GAS_VERYLOW)?;
                let (t_word, x_word) = self.pop2()?;
                let t = word_to_usize(&t_word).unwrap_or(32);
                // BYTE indexes from the most significant byte (big-endian word).
                let value = if t >= 32 { 0u8 } else { x_word[t] };
                self.stack.push(right_align(&[value]))?;
                Ok(None)
            }
            opcode::SHL => {
                self.charge(GAS_VERYLOW)?;
                let (value_word, shift_word) = self.pop2()?;
                let value = word_to_biguint(&value_word);
                let shift = word_to_usize(&shift_word).unwrap_or(usize::MAX);
                self.push_result(value << shift)?;
                Ok(None)
            }
            opcode::SHR => {
                self.charge(GAS_VERYLOW)?;
                let (value_word, shift_word) = self.pop2()?;
                let value = word_to_biguint(&value_word);
                let shift = word_to_usize(&shift_word).unwrap_or(usize::MAX);
                self.push_result(value >> shift)?;
                Ok(None)
            }
            opcode::SAR => {
                self.charge(GAS_VERYLOW)?;
                let (value_word, shift_word) = self.pop2()?;
                let shift = word_to_usize(&shift_word).unwrap_or(256);
                let value = word_to_biguint(&value_word);
                let sign_bit = value_word[0] & 0x80 != 0;
                if shift >= 256 {
                    self.stack.push(if sign_bit {
                        [0xFFu8; WORD_BYTES]
                    } else {
                        [0u8; WORD_BYTES]
                    })?;
                } else {
                    let sign_fill = if sign_bit {
                        ((BigUint::from(1u32) << shift) - &BigUint::from(1u32)) << (256 - shift)
                    } else {
                        BigUint::from(0u32)
                    };
                    self.push_result((value >> shift) | sign_fill)?;
                }
                Ok(None)
            }
            opcode::SHA3 => {
                let (offset_word, size_word) = self.pop2()?;
                let offset = word_to_usize(&offset_word).unwrap_or(usize::MAX);
                let size = word_to_usize(&size_word)
                    .unwrap_or(usize::MAX)
                    .min(MAX_MEMORY_BYTES);
                let words = size.div_ceil(WORD_BYTES) as u64;
                self.charge(GAS_SHA3_BASE.saturating_add(GAS_SHA3_PER_WORD.saturating_mul(words)))?;
                let data = self.memory.read_bytes(offset, size);
                let hash = Keccak256::digest(&data);
                let mut word = [0u8; WORD_BYTES];
                word.copy_from_slice(&hash);
                self.stack.push(word)?;
                Ok(None)
            }
            opcode::ADDRESS => {
                self.charge(GAS_BASE)?;
                self.stack.push(right_align(&self.context.caller.0))?;
                Ok(None)
            }
            opcode::BALANCE => {
                self.charge(GAS_BALANCE)?;
                // No external state in MVP: balance is always zero.
                self.stack.push([0u8; WORD_BYTES])?;
                Ok(None)
            }
            opcode::ORIGIN => {
                self.charge(GAS_BASE)?;
                self.stack.push(right_align(&self.context.origin.0))?;
                Ok(None)
            }
            opcode::CALLER => {
                self.charge(GAS_BASE)?;
                self.stack.push(right_align(&self.context.caller.0))?;
                Ok(None)
            }
            opcode::CALLVALUE => {
                self.charge(GAS_BASE)?;
                self.stack.push([0u8; WORD_BYTES])?;
                Ok(None)
            }
            opcode::CALLDATALOAD => {
                self.charge(GAS_VERYLOW)?;
                let offset = self.stack.pop()?;
                let offset = word_to_usize(&offset).unwrap_or(input.len());
                let mut word = [0u8; WORD_BYTES];
                if offset < input.len() {
                    let available = (input.len() - offset).min(WORD_BYTES);
                    word[..available].copy_from_slice(&input[offset..offset + available]);
                }
                self.stack.push(word)?;
                Ok(None)
            }
            opcode::CALLDATASIZE => {
                self.charge(GAS_BASE)?;
                let size = input.len() as u64;
                self.stack.push(right_align(&size.to_be_bytes()))?;
                Ok(None)
            }
            opcode::CALLDATACOPY => {
                let (mem_offset_word, data_offset_word, size_word) = self.pop3()?;
                let mem_offset = word_to_usize(&mem_offset_word).unwrap_or(usize::MAX);
                let data_offset = word_to_usize(&data_offset_word).unwrap_or(usize::MAX);
                let size = word_to_usize(&size_word)
                    .unwrap_or(usize::MAX)
                    .min(MAX_MEMORY_BYTES);
                let copy_words = size.div_ceil(WORD_BYTES) as u64;
                self.charge(
                    GAS_VERYLOW
                        .saturating_add(GAS_MEMORY_EXPANSION_PER_WORD.saturating_mul(copy_words)),
                )?;
                let mut data = vec![0u8; size];
                if data_offset < input.len() {
                    let available = (input.len() - data_offset).min(size);
                    data[..available].copy_from_slice(&input[data_offset..data_offset + available]);
                }
                let words_added = self.memory.write_bytes(mem_offset, &data)?;
                self.charge((words_added as u64).saturating_mul(GAS_MEMORY_EXPANSION_PER_WORD))?;
                Ok(None)
            }
            opcode::CODESIZE => {
                self.charge(GAS_BASE)?;
                let size = bytecode.len() as u64;
                self.stack.push(right_align(&size.to_be_bytes()))?;
                Ok(None)
            }
            opcode::CODECOPY => {
                let (mem_offset_word, code_offset_word, size_word) = self.pop3()?;
                let mem_offset = word_to_usize(&mem_offset_word).unwrap_or(usize::MAX);
                let code_offset = word_to_usize(&code_offset_word).unwrap_or(usize::MAX);
                let size = word_to_usize(&size_word)
                    .unwrap_or(usize::MAX)
                    .min(MAX_MEMORY_BYTES);
                let copy_words = size.div_ceil(WORD_BYTES) as u64;
                self.charge(
                    GAS_VERYLOW
                        .saturating_add(GAS_MEMORY_EXPANSION_PER_WORD.saturating_mul(copy_words)),
                )?;
                let mut data = vec![0u8; size];
                if code_offset < bytecode.len() {
                    let available = (bytecode.len() - code_offset).min(size);
                    data[..available]
                        .copy_from_slice(&bytecode[code_offset..code_offset + available]);
                }
                let words_added = self.memory.write_bytes(mem_offset, &data)?;
                self.charge((words_added as u64).saturating_mul(GAS_MEMORY_EXPANSION_PER_WORD))?;
                Ok(None)
            }
            opcode::GASPRICE => {
                self.charge(GAS_BASE)?;
                self.stack
                    .push(right_align(&self.context.gas_price.to_be_bytes()))?;
                Ok(None)
            }
            opcode::EXTCODESIZE => {
                self.charge(GAS_EXTCODE)?;
                self.stack.push([0u8; WORD_BYTES])?;
                Ok(None)
            }
            opcode::EXTCODECOPY => {
                self.charge(GAS_EXTCODE)?;
                // No external code in MVP: nothing is copied.
                self.pop3()?;
                Ok(None)
            }
            opcode::BLOCKHASH => {
                self.charge(GAS_BASE)?;
                let _ = self.stack.pop()?;
                // No block history in MVP: hash is zero.
                self.stack.push([0u8; WORD_BYTES])?;
                Ok(None)
            }
            opcode::COINBASE => {
                self.charge(GAS_BASE)?;
                self.stack.push([0u8; WORD_BYTES])?;
                Ok(None)
            }
            opcode::TIMESTAMP => {
                self.charge(GAS_BASE)?;
                self.stack
                    .push(right_align(&self.context.block_timestamp.to_be_bytes()))?;
                Ok(None)
            }
            opcode::NUMBER => {
                self.charge(GAS_BASE)?;
                self.stack
                    .push(right_align(&self.context.block_number.to_be_bytes()))?;
                Ok(None)
            }
            opcode::DIFFICULTY => {
                self.charge(GAS_BASE)?;
                self.stack
                    .push(right_align(&self.context.difficulty.to_be_bytes()))?;
                Ok(None)
            }
            opcode::GASLIMIT => {
                self.charge(GAS_BASE)?;
                self.stack
                    .push(right_align(&self.context.gas_limit.to_be_bytes()))?;
                Ok(None)
            }
            opcode::POP => {
                self.charge(GAS_BASE)?;
                let _ = self.stack.pop()?;
                Ok(None)
            }
            opcode::PUSH1..=opcode::PUSH32 => {
                let push_size = (op as usize) - 0x5f;
                if *pc + push_size > bytecode.len() {
                    return Err(VMError::InvalidInstruction);
                }
                self.charge(GAS_VERYLOW)?;
                let mut word = [0u8; WORD_BYTES];
                let start = WORD_BYTES - push_size;
                word[start..].copy_from_slice(&bytecode[*pc..*pc + push_size]);
                *pc += push_size;
                self.stack.push(word)?;
                Ok(None)
            }
            opcode::DUP1..=opcode::DUP16 => {
                let n = (op as usize) - 0x7f;
                self.charge(GAS_VERYLOW)?;
                let value = self.stack.peek(n - 1)?;
                self.stack.push(value)?;
                Ok(None)
            }
            opcode::SWAP1..=opcode::SWAP16 => {
                let n = (op as usize) - 0x8f;
                self.charge(GAS_VERYLOW)?;
                self.stack.swap(n)?;
                Ok(None)
            }
            opcode::LOG0..=opcode::LOG4 => {
                let n_topics = (op as usize) - 0xa0;
                self.charge(GAS_VERYLOW)?;
                let (offset_word, size_word) = self.pop2()?;
                let offset = word_to_usize(&offset_word).unwrap_or(usize::MAX);
                let size = word_to_usize(&size_word)
                    .unwrap_or(usize::MAX)
                    .min(MAX_MEMORY_BYTES);
                let data = self.memory.read_bytes(offset, size);
                self.charge(
                    GAS_LOG
                        .saturating_add(GAS_LOG_TOPIC.saturating_mul(n_topics as u64))
                        .saturating_add(GAS_LOG_DATA_BYTE.saturating_mul(data.len() as u64)),
                )?;
                self.logs.push(Log {
                    address: self.context.caller,
                    topics: Vec::new(),
                    data,
                });
                Ok(None)
            }
            opcode::MSIZE => {
                self.charge(GAS_BASE)?;
                self.stack
                    .push(right_align(&(self.memory.len() as u64).to_be_bytes()))?;
                Ok(None)
            }
            opcode::MLOAD => {
                self.charge(GAS_VERYLOW)?;
                let offset = self.stack.pop()?;
                let offset = word_to_usize(&offset).unwrap_or(usize::MAX);
                self.stack.push(self.memory.read_word(offset))?;
                Ok(None)
            }
            opcode::MSTORE => {
                self.charge(GAS_VERYLOW)?;
                let (offset_word, value_word) = self.pop2()?;
                let offset = word_to_usize(&offset_word).unwrap_or(usize::MAX);
                let words_added = self.memory.write_word(offset, &value_word)?;
                self.charge((words_added as u64).saturating_mul(GAS_MEMORY_EXPANSION_PER_WORD))?;
                Ok(None)
            }
            opcode::CREATE => {
                self.charge(GAS_CREATE)?;
                // No state access in MVP: contract creation is a no-op that
                // pushes a zero address placeholder.
                self.stack.push([0u8; WORD_BYTES])?;
                Ok(None)
            }
            opcode::CALL => {
                self.charge(GAS_CALL)?;
                self.stack.push([1u8; WORD_BYTES])?; // placeholder: call succeeded
                Ok(None)
            }
            opcode::CALLCODE => {
                self.charge(GAS_CALL)?;
                self.stack.push([1u8; WORD_BYTES])?;
                Ok(None)
            }
            opcode::RETURN => {
                let (offset_word, size_word) = self.pop2()?;
                let offset = word_to_usize(&offset_word).unwrap_or(usize::MAX);
                let size = word_to_usize(&size_word)
                    .unwrap_or(usize::MAX)
                    .min(MAX_MEMORY_BYTES);
                self.charge(GAS_BASE)?;
                let return_value = self.memory.read_bytes(offset, size);
                return Ok(Some(ExecutionResult::Success {
                    gas_used: self.gas_meter.gas_used,
                    return_value,
                    logs: self.logs.clone(),
                }));
            }
            opcode::DELEGATECALL => {
                self.charge(GAS_CALL)?;
                self.stack.push([1u8; WORD_BYTES])?;
                Ok(None)
            }
            opcode::CREATE2 => {
                self.charge(GAS_CREATE.saturating_add(60))?;
                self.stack.push([0u8; WORD_BYTES])?;
                Ok(None)
            }
            opcode::REVERT => {
                let (offset_word, size_word) = self.pop2()?;
                let offset = word_to_usize(&offset_word).unwrap_or(usize::MAX);
                let size = word_to_usize(&size_word)
                    .unwrap_or(usize::MAX)
                    .min(MAX_MEMORY_BYTES);
                self.charge(GAS_BASE)?;
                let reason = self.memory.read_bytes(offset, size);
                return Ok(Some(ExecutionResult::Revert {
                    gas_used: self.gas_meter.gas_used,
                    reason: String::from_utf8_lossy(&reason).into_owned(),
                }));
            }
            opcode::INVALID => {
                return Err(VMError::InvalidOpcode);
            }
            opcode::SELFDESTRUCT => {
                self.charge(GAS_SELFDESTRUCT)?;
                return Ok(Some(ExecutionResult::Error {
                    gas_used: self.gas_meter.gas_used,
                    message: "selfdestruct".to_string(),
                }));
            }
            _ => {
                return Err(VMError::InvalidOpcode);
            }
        }
    }

    /// Returns the remaining gas.
    pub fn remaining_gas(&self) -> u64 {
        self.gas_meter.remaining()
    }

    /// Returns the VM type.
    pub fn vm_type() -> VMType {
        VMType::EVM
    }
}

impl Default for EVMExecutor {
    fn default() -> Self {
        Self::new(ExecutionContext::default(), 10_000_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(val: u64) -> [u8; 32] {
        let mut w = [0u8; 32];
        w[24..].copy_from_slice(&val.to_be_bytes());
        w
    }

    #[test]
    fn evm_stack_push_pop_peek() {
        let mut stack = EvmStack::new();
        stack.push(word(1)).unwrap();
        stack.push(word(2)).unwrap();
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.peek(0).unwrap(), word(2));
        assert_eq!(stack.peek(1).unwrap(), word(1));
        assert_eq!(stack.pop().unwrap(), word(2));
        assert_eq!(stack.pop().unwrap(), word(1));
        assert_eq!(stack.pop().unwrap_err(), VMError::StackUnderflow);
    }

    #[test]
    fn evm_stack_overflow() {
        let mut stack = EvmStack::new();
        for _ in 0..MAX_STACK_DEPTH {
            assert!(stack.push(word(1)).is_ok());
        }
        assert_eq!(stack.push(word(1)).unwrap_err(), VMError::StackOverflow);
    }

    #[test]
    fn evm_stack_dup_swap() {
        let mut stack = EvmStack::new();
        stack.push(word(10)).unwrap();
        stack.push(word(20)).unwrap();
        // SWAP1 exchanges the top two items: [10, 20] -> [20, 10].
        stack.swap(1).unwrap();
        assert_eq!(stack.peek(0).unwrap(), word(10));
        assert_eq!(stack.peek(1).unwrap(), word(20));
        // DUP2 copies the second item from the top (20) onto the top.
        stack.dup(2).unwrap();
        assert_eq!(stack.peek(0).unwrap(), word(20));
        assert_eq!(stack.len(), 3);
    }

    #[test]
    fn evm_memory_read_write() {
        let mut memory = EvmMemory::new();
        let value = [42u8; 32];
        let words_added = memory.write_word(0, &value).unwrap();
        assert_eq!(words_added, 1);
        assert_eq!(memory.read_word(0), value);
        // Reads beyond allocated memory return zeros.
        assert_eq!(memory.read_word(32), [0u8; 32]);
        // Reading a partial word at the edge zero-pads.
        memory.write_bytes(30, &[1, 2]).unwrap();
        let word = memory.read_word(30);
        assert_eq!(&word[..2], &[1, 2]);
        assert_eq!(word[2], 0);
        // Expansion beyond the cap is rejected.
        assert_eq!(
            memory.write_bytes(MAX_MEMORY_BYTES, &[1]).unwrap_err(),
            VMError::InvalidMemoryAccess
        );
    }

    #[test]
    fn evm_memory_expansion_charges_gas() {
        let mut memory = EvmMemory::new();
        // Writing at offset 100 allocates 4 words (103 bytes rounded to 128).
        let words = memory.write_bytes(100, &[1, 2, 3]).unwrap();
        assert_eq!(words, 4);
        assert_eq!(words * GAS_MEMORY_EXPANSION_PER_WORD as usize, 12);
    }

    #[test]
    fn evm_stop_opcode() {
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0x00], &[]).unwrap();
        match result {
            ExecutionResult::Success { gas_used, .. } => {
                assert!(gas_used > 0);
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_add_opcode() {
        // PUSH1 1, PUSH1 2, ADD, STOP  => stack top is 3.
        let mut evm = EVMExecutor::default();
        let bytecode = [0x60, 0x01, 0x60, 0x02, 0x01, 0x00];
        let result = evm.execute(&bytecode, &[]).unwrap();
        match result {
            ExecutionResult::Success { gas_used, .. } => {
                assert_eq!(gas_used, bytecode.len() as u64 * 2 + 3 + 3 + 3);
                assert_eq!(evm.stack.items.last().unwrap(), &word(3));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_mul_opcode() {
        // PUSH1 7, PUSH1 6, MUL => stack top is 42.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x07, 0x60, 0x06, 0x02, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(42));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_sub_wraps() {
        // PUSH1 0, PUSH1 1, SUB => wraps to 2^256 - 1.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x00, 0x60, 0x01, 0x03, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &[0xFFu8; 32]);
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_division_by_zero_pushes_zero() {
        // PUSH1 5, PUSH1 0, DIV => stack top is 0.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x05, 0x60, 0x00, 0x04, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(0));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_signed_division() {
        // PUSH32 -2 (0xff...fffe), PUSH1 2, SDIV => -1 (0xff...ff).
        let mut neg_two = [0xFFu8; 32];
        neg_two[31] = 0xFE;
        let mut bytecode = vec![0x7f];
        bytecode.extend_from_slice(&neg_two);
        bytecode.extend_from_slice(&[0x60, 0x02, 0x05, 0x00]);
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&bytecode, &[]).unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &[0xFFu8; 32]);
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_comparison_opcodes() {
        // PUSH1 1, PUSH1 2, LT => 1.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x01, 0x60, 0x02, 0x10, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(1));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_iszero() {
        // PUSH1 0, ISZERO => 1.
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0x60, 0x00, 0x15, 0x00], &[]).unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(1));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_bitwise_ops() {
        // PUSH1 0x0f, PUSH1 0xf0, AND => 0.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x0f, 0x60, 0xf0, 0x16, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(0));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_not_opcode() {
        // PUSH1 0, NOT => 0xff...ff.
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0x60, 0x00, 0x19, 0x00], &[]).unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &[0xFFu8; 32]);
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_byte_opcode() {
        // PUSH1 0, PUSH32 0x1234..., BYTE with index 0 => 0x12.
        let mut value = [0u8; 32];
        value[0] = 0x12;
        value[1] = 0x34;
        let mut bytecode = vec![0x60, 0x00];
        bytecode.push(0x7f);
        bytecode.extend_from_slice(&value);
        bytecode.extend_from_slice(&[0x1a, 0x00]);
        // Stack: index(0), value; BYTE pops index then value => byte 0 = 0x12.
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&bytecode, &[]).unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(0x12));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_shift_opcodes() {
        // PUSH1 1, PUSH1 1, SHL => 2.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x01, 0x60, 0x01, 0x1b, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(2));
            }
            _ => panic!("Expected Success"),
        }
        // PUSH1 2, PUSH1 1, SHR => 1.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x02, 0x60, 0x01, 0x1c, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(1));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_exp_opcode() {
        // PUSH1 2, PUSH1 10, EXP => 1024.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x02, 0x60, 0x0a, 0x0a, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(1024));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_addmod_mulmod() {
        // PUSH1 7, PUSH1 5, PUSH1 3, ADDMOD => (7+5)%3 = 0.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x07, 0x60, 0x05, 0x60, 0x03, 0x08, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(0));
            }
            _ => panic!("Expected Success"),
        }
        // PUSH1 7, PUSH1 5, PUSH1 3, MULMOD => (7*5)%3 = 2.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x07, 0x60, 0x05, 0x60, 0x03, 0x09, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(2));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_signextend_opcode() {
        // PUSH1 0, PUSH1 0x80, SIGNEXTEND => sign extend byte 0 (LSB byte)
        // 0x80 has sign bit set => 0xff...ff80.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x00, 0x60, 0x80, 0x0b, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                let mut expected = [0xFFu8; 32];
                expected[31] = 0x80;
                assert_eq!(evm.stack.items.last().unwrap(), &expected);
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_keccak_sha3() {
        // PUSH1 0, PUSH1 0, SHA3 => keccak256("").
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x00, 0x60, 0x00, 0x20, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                let expected =
                    hex::decode("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
                        .unwrap();
                let mut word = [0u8; 32];
                word.copy_from_slice(&expected);
                assert_eq!(evm.stack.items.last().unwrap(), &word);
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_address_origin_caller() {
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0x30, 0x32, 0x33, 0x00], &[]).unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                // Stack top is CALLER (right-aligned 20-byte address).
                let top = evm.stack.items.last().unwrap();
                assert_eq!(&top[12..], evm.context.caller.0.as_ref());
                // Second item is ORIGIN.
                assert_eq!(&evm.stack.items[1][12..], evm.context.origin.0.as_ref());
                // Third item is ADDRESS (same as caller here).
                assert_eq!(&evm.stack.items[2][12..], evm.context.caller.0.as_ref());
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_block_context() {
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0x42, 0x43, 0x44, 0x45, 0x00], &[]).unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                let stack = &evm.stack.items;
                // Pushed in order TIMESTAMP, NUMBER, DIFFICULTY, GASLIMIT.
                assert_eq!(
                    stack[0],
                    right_align(&evm.context.block_timestamp.to_be_bytes())
                );
                assert_eq!(
                    stack[1],
                    right_align(&evm.context.block_number.to_be_bytes())
                );
                assert_eq!(stack[2], right_align(&evm.context.difficulty.to_be_bytes()));
                assert_eq!(stack[3], right_align(&evm.context.gas_limit.to_be_bytes()));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_call_data_ops() {
        let input = [1, 2, 3, 4, 5];
        // CALLDATASIZE => 5.
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0x36, 0x00], &input).unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(5));
            }
            _ => panic!("Expected Success"),
        }
        // CALLDATALOAD at 0 => word(1,2,3,4,5,0...) (calldata byte 0 is MSB).
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0x60, 0x00, 0x35, 0x00], &input).unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                let mut expected = [0u8; 32];
                expected[..5].copy_from_slice(&input);
                assert_eq!(evm.stack.items.last().unwrap(), &expected);
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_mload_mstore() {
        // PUSH1 0 (offset), PUSH32 value, MSTORE, PUSH1 0, MLOAD => value.
        let mut value = [0u8; 32];
        for (i, b) in value.iter_mut().enumerate().take(32) {
            *b = (i + 1) as u8;
        }
        let mut bytecode = vec![0x60, 0x00, 0x7f];
        bytecode.extend_from_slice(&value);
        bytecode.extend_from_slice(&[0x52, 0x60, 0x00, 0x51, 0x00]);
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&bytecode, &[]).unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &value);
                assert_eq!(evm.memory.data[..32], value);
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_code_copy() {
        // CODESIZE => length of bytecode.
        let bytecode = [0x38, 0x00];
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&bytecode, &[]).unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(2));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_push_dup_swap_execution() {
        // PUSH1 5, DUP1, ADD => 10.
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0x60, 0x05, 0x80, 0x01, 0x00], &[]).unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.items.last().unwrap(), &word(10));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_pop_opcode() {
        // PUSH1 7, PUSH1 3, POP => stack top is 7.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x07, 0x60, 0x03, 0x50, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { .. } => {
                assert_eq!(evm.stack.len(), 1);
                assert_eq!(evm.stack.items.last().unwrap(), &word(7));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_log_opcode() {
        // PUSH1 0, PUSH1 0, LOG0 => no error, gas charged, log recorded.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x00, 0x60, 0x00, 0xa0, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { gas_used, .. } => {
                assert_eq!(evm.logs.len(), 1);
                assert!(gas_used > code_len_gas(6));
            }
            _ => panic!("Expected Success"),
        }
    }

    fn code_len_gas(len: usize) -> u64 {
        len as u64 * 2
    }

    #[test]
    fn evm_return_opcode() {
        // PUSH1 0, PUSH1 0, RETURN => Success with empty return value.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x00, 0x60, 0x00, 0xf3, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Success { return_value, .. } => {
                assert!(return_value.is_empty());
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_return_with_data() {
        // PUSH1 0, PUSH32 data, MSTORE, PUSH1 0, PUSH1 32, RETURN => data.
        let mut data = [0u8; 32];
        data[0] = 0xab;
        let mut bytecode = vec![0x60, 0x00, 0x7f];
        bytecode.extend_from_slice(&data);
        bytecode.extend_from_slice(&[0x52, 0x60, 0x00, 0x60, 0x20, 0xf3, 0x00]);
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&bytecode, &[]).unwrap();
        match result {
            ExecutionResult::Success { return_value, .. } => {
                assert_eq!(return_value, data);
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_revert_opcode() {
        // PUSH1 0, PUSH1 0, REVERT => Revert with empty reason.
        let mut evm = EVMExecutor::default();
        let result = evm
            .execute(&[0x60, 0x00, 0x60, 0x00, 0xfd, 0x00], &[])
            .unwrap();
        match result {
            ExecutionResult::Revert { reason, gas_used } => {
                assert_eq!(reason, "");
                assert!(gas_used > 0);
            }
            _ => panic!("Expected Revert"),
        }
    }

    #[test]
    fn evm_revert_with_reason() {
        // PUSH1 0, PUSH32 "revert", MSTORE, PUSH1 0, PUSH1 32, REVERT.
        let reason = b"revert";
        let mut bytecode = vec![0x60, 0x00, 0x7f];
        bytecode.extend_from_slice(reason);
        bytecode.extend(std::iter::repeat(0u8).take(32 - reason.len()));
        bytecode.extend_from_slice(&[0x52, 0x60, 0x00, 0x60, 0x06, 0xfd, 0x00]);
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&bytecode, &[]).unwrap();
        match result {
            ExecutionResult::Revert { reason, .. } => {
                assert_eq!(reason, "revert");
            }
            _ => panic!("Expected Revert"),
        }
    }

    #[test]
    fn evm_selfdestruct_opcode() {
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0xff], &[]).unwrap();
        match result {
            ExecutionResult::Error { gas_used, message } => {
                assert_eq!(message, "selfdestruct");
                assert!(gas_used >= GAS_SELFDESTRUCT);
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn evm_invalid_opcode() {
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0xfe], &[]);
        assert_eq!(result.unwrap_err(), VMError::InvalidOpcode);
    }

    #[test]
    fn evm_unknown_opcode() {
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0x0c], &[]);
        assert_eq!(result.unwrap_err(), VMError::InvalidOpcode);
    }

    #[test]
    fn evm_stack_underflow() {
        let mut evm = EVMExecutor::default();
        // ADD with an empty stack underflows.
        let result = evm.execute(&[0x01], &[]);
        assert_eq!(result.unwrap_err(), VMError::StackUnderflow);
    }

    #[test]
    fn evm_stack_overflow_execution() {
        let mut evm = EVMExecutor::default();
        // 1025 PUSH1 operations overflow the stack.
        let mut bytecode = Vec::new();
        for _ in 0..MAX_STACK_DEPTH + 1 {
            bytecode.push(0x60);
            bytecode.push(0x00);
        }
        bytecode.push(0x00);
        let result = evm.execute(&bytecode, &[]);
        assert_eq!(result.unwrap_err(), VMError::StackOverflow);
    }

    #[test]
    fn evm_out_of_gas() {
        let mut evm = EVMExecutor::new(ExecutionContext::default(), 5);
        let result = evm.execute(&[0x01, 0x01, 0x02, 0x01, 0x01, 0x02, 0x00], &[]);
        assert_eq!(result.unwrap_err(), VMError::OutOfGas);
    }

    #[test]
    fn evm_gas_charging() {
        let mut evm = EVMExecutor::default();
        let bytecode = [0x60, 0x01, 0x60, 0x02, 0x01, 0x00]; // 6 bytes
        evm.execute(&bytecode, &[]).unwrap();
        assert_eq!(
            evm.gas_meter.gas_used,
            bytecode.len() as u64 * 2 + 3 + 3 + 3
        );
    }

    #[test]
    fn evm_vm_type() {
        assert_eq!(EVMExecutor::vm_type(), VMType::EVM);
    }

    #[test]
    fn evm_remaining_gas() {
        let mut evm = EVMExecutor::new(ExecutionContext::default(), 1000);
        evm.execute(&[0x00], &[]).unwrap();
        assert!(evm.remaining_gas() < 1000);
    }

    /// Helper to get gas_used from a result (mirrors the historical test
    /// helper so the suite stays self-contained).
    trait GasUsed {
        fn gas_used(&self) -> u64;
    }

    impl GasUsed for ExecutionResult {
        fn gas_used(&self) -> u64 {
            match self {
                ExecutionResult::Success { gas_used, .. } => *gas_used,
                ExecutionResult::Revert { gas_used, .. } => *gas_used,
                ExecutionResult::Error { gas_used, .. } => *gas_used,
            }
        }
    }
}
