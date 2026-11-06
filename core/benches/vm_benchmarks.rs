//! Criterion benchmarks for the six Atoll VMs (EVM, WASM, Move, RISC-V, AI
//! VM, ZK VM) and the VM router.
//!
//! Each benchmark constructs a fresh executor per iteration so that gas
//! metering does not accumulate across criterion's many iterations. Execution
//! inputs and the measured gas value are wrapped in `criterion::black_box` to
//! prevent the compiler from optimizing the measured work away.

use atoll_core::crypto::morph_hash::MorphHashEngine;
use atoll_core::primitives::hash::Hash512;
use atoll_core::vm::ai_vm::AIVMExecutor;
use atoll_core::vm::evm::EVMExecutor;
use atoll_core::vm::move_vm::MoveExecutor;
use atoll_core::vm::riscv::RISCVExecutor;
use atoll_core::vm::wasm::WASMExecutor;
use atoll_core::vm::zk_vm::ZKVMExecutor;
use atoll_core::vm::{ExecutionContext, ExecutionResult, VMError, VMRouter};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

/// Extracts the gas used from an execution result, or zero on error.
fn gas_of(result: Result<ExecutionResult, VMError>) -> u64 {
    match result {
        Ok(ExecutionResult::Success { gas_used, .. })
        | Ok(ExecutionResult::Revert { gas_used, .. })
        | Ok(ExecutionResult::Error { gas_used, .. }) => gas_used,
        Err(_) => 0,
    }
}

/// Builds a minimal valid WASM module exporting `main: () -> ()` that returns
/// successfully. Sections: header, type, function, export, code.
fn wasm_return_unit_module() -> Vec<u8> {
    vec![
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // magic + version 1
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00, // type section: () -> ()
        0x03, 0x02, 0x01, 0x00, // function section: one function of type 0
        0x07, 0x08, 0x01, 0x04, b'm', b'a', b'i', b'n', 0x00, 0x00, // export "main": func 0
        0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b, // code section: 0 locals, end
    ]
}

/// Builds a minimal valid Move module header: magic, version 1, one function,
/// zero resources, zero imports, empty body (20 bytes total).
fn move_module_bytecode() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]); // magic
    bytes.extend_from_slice(&1u32.to_le_bytes()); // version
    bytes.extend_from_slice(&1u32.to_le_bytes()); // function_count
    bytes.extend_from_slice(&0u32.to_le_bytes()); // resource_count
    bytes.extend_from_slice(&0u32.to_le_bytes()); // import_count
    bytes
}

/// Builds a minimal valid ELF64 header (64 bytes) for a RISC-V executable with
/// entry point 0x1000. The header itself is loaded as the initial instruction
/// stream, so the first decoded instruction is invalid; this exercises the
/// header parsing and decode path without panicking.
fn riscv_elf64_bytecode() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0x7f, 0x45, 0x4c, 0x46]); // \x7fELF
    bytes.push(2); // ELFCLASS64
    bytes.push(1); // ELFDATA2LSB
    bytes.push(1); // EV_CURRENT
    bytes.push(0); // OS/ABI
    bytes.push(0); // ABI version
    bytes.extend_from_slice(&[0u8; 7]); // pad
    bytes.extend_from_slice(&2u16.to_le_bytes()); // e_type = ET_EXEC
    bytes.extend_from_slice(&243u16.to_le_bytes()); // e_machine = EM_RISCV
    bytes.extend_from_slice(&1u32.to_le_bytes()); // e_version
    bytes.extend_from_slice(&0x1000u64.to_le_bytes()); // e_entry
    bytes.extend_from_slice(&0u64.to_le_bytes()); // e_phoff
    bytes.extend_from_slice(&0u64.to_le_bytes()); // e_shoff
    bytes.extend_from_slice(&0u32.to_le_bytes()); // e_flags
    bytes.extend_from_slice(&64u16.to_le_bytes()); // e_ehsize
    bytes.extend_from_slice(&0u16.to_le_bytes()); // e_phentsize
    bytes.extend_from_slice(&0u16.to_le_bytes()); // e_phnum
    bytes.extend_from_slice(&0u16.to_le_bytes()); // e_shentsize
    bytes.extend_from_slice(&0u16.to_le_bytes()); // e_shnum
    bytes.extend_from_slice(&0u16.to_le_bytes()); // e_shstrndx
    bytes
}

// ---------------------------------------------------------------------------
// Per-VM benchmarks
// ---------------------------------------------------------------------------

fn bench_evm_execute(c: &mut Criterion) {
    let context = ExecutionContext::default();
    // PUSH1 7, PUSH1 6, MUL, STOP  -> stack top is 42, then halt.
    let bytecode = vec![0x60, 0x07, 0x60, 0x06, 0x02, 0x00];
    c.bench_function("evm_execute", |b| {
        b.iter(|| {
            let mut evm = EVMExecutor::new(context.clone(), 10_000_000);
            let result = evm.execute(black_box(&bytecode), black_box(&[]));
            black_box(gas_of(result))
        })
    });
}

fn bench_wasm_execute(c: &mut Criterion) {
    let context = ExecutionContext::default();
    let module = wasm_return_unit_module();
    let function = "main";
    let args: &[Vec<u8>] = &[];
    c.bench_function("wasm_execute", |b| {
        b.iter(|| {
            let mut wasm = WASMExecutor::new(context.clone(), 10_000_000);
            let result = wasm.execute(black_box(&module), black_box(function), black_box(args));
            black_box(gas_of(result))
        })
    });
}

fn bench_move_execute(c: &mut Criterion) {
    let context = ExecutionContext::default();
    let bytecode = move_module_bytecode();
    let function = "main";
    let args: &[Vec<u8>] = &[];
    c.bench_function("move_execute", |b| {
        b.iter(|| {
            let mut mv = MoveExecutor::new(context.clone(), 10_000_000);
            let result = mv.execute(black_box(&bytecode), black_box(function), black_box(args));
            black_box(gas_of(result))
        })
    });
}

fn bench_riscv_execute(c: &mut Criterion) {
    let context = ExecutionContext::default();
    let binary = riscv_elf64_bytecode();
    let entry_point = 0x1000u64;
    let args: &[Vec<u8>] = &[];
    c.bench_function("riscv_execute", |b| {
        b.iter(|| {
            let mut riscv = RISCVExecutor::new(context.clone(), 10_000_000);
            let result = riscv.execute(black_box(&binary), black_box(entry_point), black_box(args));
            black_box(gas_of(result))
        })
    });
}

fn bench_ai_vm_deploy_and_infer(c: &mut Criterion) {
    let context = ExecutionContext::default();
    let model_bytes = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    ];
    let input = vec![1u8, 2, 3, 4, 5];
    c.bench_function("ai_vm_deploy_and_infer", |b| {
        b.iter(|| {
            let mut ai = AIVMExecutor::new(context.clone(), 50_000_000);
            let deploy = black_box(ai.deploy_model(black_box(&model_bytes)));
            let model_hash = match deploy {
                Ok(ExecutionResult::Success { return_value, .. }) => {
                    Hash512::from_bytes(black_box(&return_value)).unwrap()
                }
                _ => Hash512::zero(),
            };
            let result = ai.execute_inference(black_box(&model_hash), black_box(&input));
            black_box(gas_of(result))
        })
    });
}

fn bench_zk_vm_deploy_and_execute(c: &mut Criterion) {
    let context = ExecutionContext::default();
    let circuit_bytes = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    ];
    let public_inputs = vec![vec![4u8, 5], vec![6, 7]];
    let proof = vec![0x01, 0x02, 0x03];
    c.bench_function("zk_vm_deploy_and_execute", |b| {
        b.iter(|| {
            let mut zk = ZKVMExecutor::new(context.clone(), 100_000_000);
            let deploy = black_box(zk.deploy_circuit(black_box(&circuit_bytes)));
            let circuit_id = match deploy {
                Ok(ExecutionResult::Success { return_value, .. }) => {
                    Hash512::from_bytes(black_box(&return_value)).unwrap()
                }
                _ => Hash512::zero(),
            };
            let result = zk.execute(
                black_box(&circuit_id),
                black_box(&public_inputs),
                black_box(&proof),
            );
            black_box(gas_of(result))
        })
    });
}

// ---------------------------------------------------------------------------
// VM router benchmarks
// ---------------------------------------------------------------------------

/// Registers a single VM router bench. `bytecode` is moved into the benchmark
/// closure and borrowed on every iteration; `context` is cloned per iteration.
fn bench_router(c: &mut Criterion, name: &str, bytecode: Vec<u8>) {
    let context = ExecutionContext::default();
    c.bench_function(name, |b| {
        b.iter(|| {
            let result = VMRouter::execute(context.clone(), black_box(&bytecode));
            black_box(gas_of(Ok(result)))
        })
    });
}

fn router_benches(c: &mut Criterion) {
    // EVM route (prefix 0x00): STOP halts immediately.
    bench_router(c, "router_evm", vec![0x00, 0x60, 0x01, 0x00]);
    // WASM route (prefix 0x01): first byte is not 0x00 so the WASM magic check
    // fails; measures the route + reject path.
    bench_router(
        c,
        "router_wasm",
        vec![0x01, 0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00],
    );
    // Move route (prefix 0x02): bad Move magic -> rejected.
    bench_router(c, "router_move", vec![0x02; 20]);
    // RISC-V route (prefix 0x03): bad ELF magic -> rejected.
    bench_router(c, "router_riscv", vec![0x03; 64]);
    // AI route (prefix 0x04): deploy_model succeeds on >= 12 bytes.
    bench_router(c, "router_ai", vec![0x04; 12]);
    // ZK route (prefix 0x05): deploy_circuit succeeds on >= 12 bytes.
    bench_router(c, "router_zk", vec![0x05; 12]);
}

// ---------------------------------------------------------------------------
// Algorithm chain cache benchmarks
// ---------------------------------------------------------------------------

/// Benchmarks morph_hash with a cache miss (fresh engine, new prev_hash+height).
fn bench_algorithm_chain_cache_miss(c: &mut Criterion) {
    let prev_hash = Hash512::blake3(b"cache_miss_test");
    c.bench_function("morph_hash_cache_miss", |b| {
        b.iter(|| {
            let mut engine = MorphHashEngine::new();
            let result = engine.morph_hash(
                black_box(b"benchmark input"),
                black_box(&prev_hash),
                black_box(42),
            );
            black_box(result)
        })
    });
}

/// Benchmarks morph_hash with a cache hit (same engine, same prev_hash+height).
fn bench_algorithm_chain_cache_hit(c: &mut Criterion) {
    let prev_hash = Hash512::blake3(b"cache_hit_test");
    let mut engine = MorphHashEngine::new();
    // Prime the cache
    engine.morph_hash(b"benchmark input", &prev_hash, 42);
    c.bench_function("morph_hash_cache_hit", |b| {
        b.iter(|| {
            let result = engine.morph_hash(
                black_box(b"benchmark input"),
                black_box(&prev_hash),
                black_box(42),
            );
            black_box(result)
        })
    });
}

/// Benchmarks morph_hash cache hit rate over many repeated calls.
fn bench_algorithm_chain_cache_hit_rate(c: &mut Criterion) {
    let prev_hash = Hash512::blake3(b"cache_hit_rate_test");
    let mut engine = MorphHashEngine::new();
    c.bench_function("morph_hash_cache_hit_rate", |b| {
        b.iter(|| {
            let mut hits = 0u64;
            let mut total = 0u64;
            for _ in 0..100 {
                let result = engine.morph_hash(b"benchmark input", &prev_hash, 42);
                black_box(result);
                total += 1;
                let (h, _, _) = engine.cache_stats();
                if h > hits {
                    hits = h;
                }
            }
            let hit_rate = if total > 0 {
                hits as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            black_box(hit_rate)
        })
    });
}

criterion_group! {
    name = vm_benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(200))
        .measurement_time(Duration::from_millis(200));
    targets =
    bench_evm_execute,
    bench_wasm_execute,
    bench_move_execute,
    bench_riscv_execute,
    bench_ai_vm_deploy_and_infer,
    bench_zk_vm_deploy_and_execute,
    router_benches,
    bench_algorithm_chain_cache_miss,
    bench_algorithm_chain_cache_hit,
    bench_algorithm_chain_cache_hit_rate,
}

criterion_main!(vm_benches);
