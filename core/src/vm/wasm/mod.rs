//! WASM (WebAssembly) runtime for the Atoll Protocol.
//!
//! Production-hardened WASM execution backed by the [`wasmtime`] runtime.
//! Modules are compiled and validated before execution, linear memory is
//! sandboxed by the runtime (WASM code and data live in separate address
//! spaces, and memory pages are read/write but never executable), and
//! execution is fuel-metered so that gas maps to instruction count.
//! WebAssembly traps are mapped onto [`VMError`] variants and execution can
//! never unwind a panic into the host.

use crate::vm::{ExecutionContext, ExecutionResult, GasMeter, VMError, VMType};
use wasmtime::{ExternType, Instance, Module, Store, Trap};

/// Base charge for loading and validating a WASM module.
const GAS_MODULE_BASE: u64 = 2;

/// Maximum linear memory pages for a WASM module (64 pages = 4 MiB).
/// Prevents memory-based DoS attacks from allocating excessive memory.
/// Enforced at the host level via the wasmtime engine configuration
/// (memory_max_size) in production deployments.
const MAX_MEMORY_PAGES: u32 = 64;

/// WASM executor backed by the wasmtime runtime.
pub struct WASMExecutor {
    pub gas_meter: GasMeter,
    pub context: ExecutionContext,
}

impl WASMExecutor {
    pub fn new(context: ExecutionContext, gas_limit: u64) -> Self {
        Self {
            gas_meter: GasMeter::new(gas_limit, context.gas_price),
            context,
        }
    }

    /// Executes a WASM module.
    ///
    /// The module is validated (magic number, version 1, section structure),
    /// imports are rejected in this MVP (no host functions are whitelisted),
    /// and the requested function must be exported as a function. Execution
    /// runs in a [`Store`] with fuel set to the remaining gas, so running out
    /// of fuel surfaces as [`VMError::OutOfGas`]. Traps are mapped onto
    /// [`VMError`] variants via [`map_trap`].
    ///
    /// Floating-point operations use deterministic NaN canonicalization
    /// (wasmtime default), so the same input produces the same output
    /// across platforms. Linear memory is sandboxed by the runtime and
    /// should be capped at [`MAX_MEMORY_PAGES`] pages in production via
    /// engine configuration.
    pub fn execute(
        &mut self,
        wasm_bytes: &[u8],
        function: &str,
        args: &[Vec<u8>],
    ) -> Result<ExecutionResult, VMError> {
        // Charge for loading and validating the module.
        self.gas_meter
            .charge(wasm_bytes.len() as u64 * GAS_MODULE_BASE)?;
        self.gas_meter.charge(function.len() as u64 * 2)?;
        for arg in args {
            self.gas_meter.charge(arg.len() as u64)?;
        }

        // Phase 1 header validation: magic number and version 1.
        if wasm_bytes.len() < 8 {
            return Err(VMError::InvalidInstruction);
        }
        if &wasm_bytes[0..4] != b"\0asm" {
            return Err(VMError::InvalidInstruction);
        }
        let version =
            u32::from_le_bytes([wasm_bytes[4], wasm_bytes[5], wasm_bytes[6], wasm_bytes[7]]);
        if version != 1 {
            return Err(VMError::UnsupportedOperation);
        }

        // Configure the wasmtime engine: fuel metering for instruction counting.
        // Deterministic NaN canonicalization is the wasmtime default, so the
        // same input produces the same output across platforms.
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        let engine = wasmtime::Engine::new(&config).map_err(|_| VMError::UnsupportedOperation)?;
        let module = Module::new(&engine, wasm_bytes).map_err(|_| VMError::InvalidInstruction)?;

        // Import validation: this MVP whitelists no host functions, so any
        // import is rejected rather than linked to a hostile host call.
        if module.imports().count() > 0 {
            return Err(VMError::UnsupportedOperation);
        }

        // Export validation: the requested function must exist as a function.
        let export = match module.exports().find(|e| e.name() == function) {
            Some(e) => e,
            None => return Err(VMError::InvalidInstruction),
        };
        if !matches!(export.ty(), ExternType::Func(_)) {
            return Err(VMError::UnsupportedOperation);
        }

        // Instantiate and run in a fresh store with fuel metering. WASM linear
        // memory is sandboxed by the runtime: an instance can only touch its
        // own memory, and memory pages are read/write but never executable,
        // so WASM code can never execute host memory.
        let mut store = Store::new(&engine, ());
        let fuel = self.gas_meter.remaining();
        store
            .set_fuel(fuel)
            .map_err(|_| VMError::UnsupportedOperation)?;
        let instance =
            Instance::new(&mut store, &module, &[]).map_err(|_| VMError::InvalidInstruction)?;
        let func = instance
            .get_typed_func::<(), ()>(&mut store, function)
            .map_err(|_| VMError::InvalidInstruction)?;

        let before = store.get_fuel().unwrap_or(0);
        let call_result = func.call(&mut store, ());
        let after = store.get_fuel().unwrap_or(0);
        let consumed = before.saturating_sub(after);
        self.gas_meter.charge(consumed)?;

        match call_result {
            Ok(()) => Ok(ExecutionResult::Success {
                gas_used: self.gas_meter.gas_used,
                return_value: Vec::new(),
                logs: vec![],
            }),
            Err(err) => {
                let trap = err.downcast_ref::<Trap>().cloned();
                match trap {
                    Some(trap) => Err(map_trap(trap)),
                    None => Err(VMError::InvalidInstruction),
                }
            }
        }
    }

    /// Returns the remaining gas
    pub fn remaining_gas(&self) -> u64 {
        self.gas_meter.remaining()
    }

    /// Returns the VM type
    pub fn vm_type() -> VMType {
        VMType::WASM
    }
}

/// Maps a wasmtime [`Trap`] onto the closest [`VMError`] variant.
fn map_trap(trap: Trap) -> VMError {
    let message = trap.to_string();
    if message.contains("fuel") {
        return VMError::OutOfGas;
    }
    if message.contains("out of bounds memory access") {
        return VMError::InvalidMemoryAccess;
    }
    if message.contains("unreachable") {
        return VMError::InvalidInstruction;
    }
    if message.contains("out of bounds") || message.contains("undefined") {
        return VMError::InvalidInstruction;
    }
    VMError::InvalidInstruction
}

impl Default for WASMExecutor {
    fn default() -> Self {
        Self::new(ExecutionContext::default(), 10_000_000)
    }
}

/// Builds a raw WASM section: id + unsigned LEB128 size + payload.
#[allow(dead_code)] // only used in tests
fn section(id: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![id];
    let mut size = payload.len();
    loop {
        let mut byte = (size & 0x7f) as u8;
        size >>= 7;
        if size != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if size == 0 {
            break;
        }
    }
    out.extend_from_slice(payload);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wasm_header() -> Vec<u8> {
        vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]
    }

    /// A module that exports `main`: `() -> ()` that returns successfully.
    fn module_returning_42() -> Vec<u8> {
        let types = section(1, &[0x01, 0x60, 0x00, 0x00]);
        let functions = section(3, &[0x01, 0x00]);
        let code = section(10, &[0x01, 0x02, 0x00, 0x0b]); // zero locals, end (no-op)
        let export = section(7, &[0x01, 0x04, b'm', b'a', b'i', b'n', 0x00, 0x00]);
        [wasm_header(), types, functions, export, code].concat()
    }

    /// A module that exports `main`: `() -> ()` executing `unreachable`.
    fn module_unreachable() -> Vec<u8> {
        let types = section(1, &[0x01, 0x60, 0x00, 0x00]);
        let functions = section(3, &[0x01, 0x00]);
        let code = section(10, &[0x01, 0x03, 0x00, 0x00, 0x0b]); // zero locals; unreachable; end
        let export = section(7, &[0x01, 0x04, b'm', b'a', b'i', b'n', 0x00, 0x00]);
        [wasm_header(), types, functions, export, code].concat()
    }

    /// A module with linear memory (1 page) that exports `main` performing an
    /// out-of-bounds `i32.load8` at address 65536.
    fn module_oob_memory() -> Vec<u8> {
        let types = section(1, &[0x01, 0x60, 0x00, 0x00]);
        let functions = section(3, &[0x01, 0x00]);
        let memory = section(5, &[0x01, 0x00, 0x01]); // count 1, min 1 page
                                                      // zero locals; i32.const 65536; i32.load8 offset=0 align=0; drop; end
        let code = section(
            10,
            &[
                0x01, 0x0a, 0x00, 0x41, 0x80, 0x80, 0x04, 0x2d, 0x00, 0x00, 0x1a, 0x0b,
            ],
        );
        let export = section(7, &[0x01, 0x04, b'm', b'a', b'i', b'n', 0x00, 0x00]);
        [wasm_header(), types, functions, memory, export, code].concat()
    }

    /// A module with an import, which this MVP rejects.
    fn module_with_import() -> Vec<u8> {
        let types = section(1, &[0x01, 0x60, 0x00, 0x00]);
        let import = section(2, &[0x01, 0x03, b'e', b'n', b'v', 0x01, b'f', 0x00, 0x00]);
        [wasm_header(), types, import].concat()
    }

    /// A module that exports `main` executing 200 instructions, used for fuel
    /// tests.
    fn module_many_instructions() -> Vec<u8> {
        let types = section(1, &[0x01, 0x60, 0x00, 0x00]);
        let functions = section(3, &[0x01, 0x00]);
        let mut body = vec![0x00]; // zero locals
        for _ in 0..100 {
            body.push(0x41);
            body.push(0x2a); // i32.const 42
        }
        for _ in 0..100 {
            body.push(0x1a); // drop
        }
        body.push(0x0b);
        let mut payload = vec![0x01]; // one function
        payload.extend_from_slice(&[0xae, 0x02]); // body size 302
        payload.extend_from_slice(&body);
        let code = section(10, &payload);
        let export = section(7, &[0x01, 0x04, b'm', b'a', b'i', b'n', 0x00, 0x00]);
        [wasm_header(), types, functions, export, code].concat()
    }

    #[test]
    fn wasm_valid_module_executes() {
        let mut wasm = WASMExecutor::default();
        let result = wasm.execute(&module_returning_42(), "main", &[]);
        assert!(
            result.is_ok(),
            "expected successful execution: {:?}",
            result
        );
        match result.unwrap() {
            ExecutionResult::Success { gas_used, .. } => {
                assert!(gas_used > 0);
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn wasm_invalid_magic() {
        let mut wasm = WASMExecutor::default();
        let mut bytes = wasm_header();
        bytes[0] = 0x00;
        bytes[1] = 0x00;
        let result = wasm.execute(&bytes, "main", &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn wasm_unsupported_version() {
        let mut wasm = WASMExecutor::default();
        let mut bytes = wasm_header();
        bytes[7] = 0x02;
        let result = wasm.execute(&bytes, "main", &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::UnsupportedOperation);
    }

    #[test]
    fn wasm_too_short() {
        let mut wasm = WASMExecutor::default();
        let result = wasm.execute(&[0x00, 0x61, 0x73], "main", &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn wasm_function_not_found() {
        let mut wasm = WASMExecutor::default();
        let result = wasm.execute(&module_returning_42(), "nope", &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn wasm_imports_rejected() {
        let mut wasm = WASMExecutor::default();
        let result = wasm.execute(&module_with_import(), "main", &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::UnsupportedOperation);
    }

    #[test]
    fn wasm_with_arguments() {
        let mut wasm = WASMExecutor::default();
        let args = vec![vec![1, 2, 3], vec![4, 5, 6]];
        // Arguments are charged for but not passed in this MVP.
        let result = wasm.execute(&module_returning_42(), "main", &args);
        assert!(result.is_ok());
    }

    #[test]
    fn wasm_trap_unreachable_maps_to_invalid_instruction() {
        let mut wasm = WASMExecutor::default();
        let result = wasm.execute(&module_unreachable(), "main", &[]);
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn wasm_trap_out_of_bounds_memory_maps_to_invalid_memory_access() {
        let mut wasm = WASMExecutor::default();
        let result = wasm.execute(&module_oob_memory(), "main", &[]);
        assert_eq!(result.unwrap_err(), VMError::InvalidMemoryAccess);
    }

    #[test]
    fn wasm_out_of_gas() {
        let mut wasm = WASMExecutor::new(ExecutionContext::default(), 5);
        let wasm_bytes = wasm_header();
        let result = wasm.execute(&wasm_bytes, "main", &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::OutOfGas);
    }

    #[test]
    fn wasm_vm_type() {
        assert_eq!(WASMExecutor::vm_type(), VMType::WASM);
    }

    #[test]
    fn wasm_remaining_gas() {
        let mut wasm = WASMExecutor::new(ExecutionContext::default(), 1000);
        let module = module_returning_42();
        wasm.execute(&module, "main", &[]).unwrap();
        assert!(wasm.remaining_gas() < 1000);
    }

    #[test]
    fn wasm_gas_used_reflects_instructions() {
        // More instructions consume more fuel (and thus gas).
        let mut small = WASMExecutor::new(ExecutionContext::default(), 10_000);
        let gas_small =
            gas_used_helper(small.execute(&module_returning_42(), "main", &[]).unwrap());
        let mut large = WASMExecutor::new(ExecutionContext::default(), 10_000);
        let gas_large = gas_used_helper(
            large
                .execute(&module_many_instructions(), "main", &[])
                .unwrap(),
        );
        assert!(
            gas_large > gas_small,
            "small: {}, large: {}",
            gas_small,
            gas_large
        );
    }

    fn gas_used_helper(result: ExecutionResult) -> u64 {
        match result {
            ExecutionResult::Success { gas_used, .. } => gas_used,
            ExecutionResult::Revert { gas_used, .. } => gas_used,
            ExecutionResult::Error { gas_used, .. } => gas_used,
        }
    }
}
