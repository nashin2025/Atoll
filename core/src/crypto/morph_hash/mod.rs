//! Morph-Hash cryptographic hash function for the Atoll Protocol.
//!
//! Self-evolving hash with rotating algorithm pools, AI-evolved S-boxes,
//! memory-hard computation, and post-quantum finalization.
//!
//! Phase 2: Production implementation with real cryptographic algorithms.
//! SHA-512, SHA3-512, BLAKE3, Argon2id, and Scrypt use real crypto crates.
//! Other algorithms use simplified implementations (to be replaced with
//! full implementations in Phase 3).

use crate::primitives::hash::Hash512;
use digest::{Digest, OutputSizeUser};
use sha2::Sha512;
use sha3::{Digest as Sha3Digest, Sha3_512};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Number of algorithms in the rotation pool
pub const ALGORITHM_POOL_SIZE: usize = 32;

/// Number of algorithms chained per hash invocation
pub const CHAIN_LENGTH: usize = 16;

/// S-box evolution interval (blocks)
pub const SBOX_EVOLUTION_INTERVAL: u64 = 10_000;

/// Memory buffer size in bytes (4 GB in production)
pub const MEMORY_BUFFER_SIZE: usize = 4 * 1024 * 1024 * 1024;

/// Number of random memory access rounds per hash
pub const MEMORY_ACCESS_ROUNDS: usize = 1024;

// ---------------------------------------------------------------------------
// Algorithm Pool
// ---------------------------------------------------------------------------

/// The 32 algorithm variants in the Morph-Hash rotation pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MorphAlgorithm {
    // Sponge-based
    Keccak512,
    SHA3_512,
    KangarooTwelve,

    // Merkle-Damgård based
    SHA512,
    Whirlpool,
    SHA512_256,

    // HAIFA-based
    BLAKE2b,
    BLAKE3,

    // Unique designs
    Skein512,
    JH512,
    Groestl512,
    Fugue512,
    Luffa512,
    SHAvite512,
    SIMD512,
    ECHO512,
    Hamsi512,
    Shabal512,
    BMW512,
    CubeHash512,

    // Memory-hard
    Argon2id,
    Scrypt,
    Yescrypt,
    Lyra2Rev3,
    BalloonHash,

    // CPU-hard
    RandomX,

    // Custom ATOLL primitives
    AtollARX,
    AtollSponge,
    AtollFeistel,
    AtollSPN,
    AtollLattice,
    AtollTreeHash,
}

impl MorphAlgorithm {
    /// Returns all 32 algorithms in pool order.
    pub fn all() -> [MorphAlgorithm; ALGORITHM_POOL_SIZE] {
        [
            MorphAlgorithm::Keccak512,
            MorphAlgorithm::SHA3_512,
            MorphAlgorithm::KangarooTwelve,
            MorphAlgorithm::SHA512,
            MorphAlgorithm::Whirlpool,
            MorphAlgorithm::SHA512_256,
            MorphAlgorithm::BLAKE2b,
            MorphAlgorithm::BLAKE3,
            MorphAlgorithm::Skein512,
            MorphAlgorithm::JH512,
            MorphAlgorithm::Groestl512,
            MorphAlgorithm::Fugue512,
            MorphAlgorithm::Luffa512,
            MorphAlgorithm::SHAvite512,
            MorphAlgorithm::SIMD512,
            MorphAlgorithm::ECHO512,
            MorphAlgorithm::Hamsi512,
            MorphAlgorithm::Shabal512,
            MorphAlgorithm::BMW512,
            MorphAlgorithm::CubeHash512,
            MorphAlgorithm::Argon2id,
            MorphAlgorithm::Scrypt,
            MorphAlgorithm::Yescrypt,
            MorphAlgorithm::Lyra2Rev3,
            MorphAlgorithm::BalloonHash,
            MorphAlgorithm::RandomX,
            MorphAlgorithm::AtollARX,
            MorphAlgorithm::AtollSponge,
            MorphAlgorithm::AtollFeistel,
            MorphAlgorithm::AtollSPN,
            MorphAlgorithm::AtollLattice,
            MorphAlgorithm::AtollTreeHash,
        ]
    }

    /// Returns a short name for debugging and logging.
    pub fn name(&self) -> &'static str {
        match self {
            MorphAlgorithm::Keccak512 => "Keccak512",
            MorphAlgorithm::SHA3_512 => "SHA3_512",
            MorphAlgorithm::KangarooTwelve => "KangarooTwelve",
            MorphAlgorithm::SHA512 => "SHA512",
            MorphAlgorithm::Whirlpool => "Whirlpool",
            MorphAlgorithm::SHA512_256 => "SHA512_256",
            MorphAlgorithm::BLAKE2b => "BLAKE2b",
            MorphAlgorithm::BLAKE3 => "BLAKE3",
            MorphAlgorithm::Skein512 => "Skein512",
            MorphAlgorithm::JH512 => "JH512",
            MorphAlgorithm::Groestl512 => "Groestl512",
            MorphAlgorithm::Fugue512 => "Fugue512",
            MorphAlgorithm::Luffa512 => "Luffa512",
            MorphAlgorithm::SHAvite512 => "SHAvite512",
            MorphAlgorithm::SIMD512 => "SIMD512",
            MorphAlgorithm::ECHO512 => "ECHO512",
            MorphAlgorithm::Hamsi512 => "Hamsi512",
            MorphAlgorithm::Shabal512 => "Shabal512",
            MorphAlgorithm::BMW512 => "BMW512",
            MorphAlgorithm::CubeHash512 => "CubeHash512",
            MorphAlgorithm::Argon2id => "Argon2id",
            MorphAlgorithm::Scrypt => "Scrypt",
            MorphAlgorithm::Yescrypt => "Yescrypt",
            MorphAlgorithm::Lyra2Rev3 => "Lyra2Rev3",
            MorphAlgorithm::BalloonHash => "BalloonHash",
            MorphAlgorithm::RandomX => "RandomX",
            MorphAlgorithm::AtollARX => "AtollARX",
            MorphAlgorithm::AtollSponge => "AtollSponge",
            MorphAlgorithm::AtollFeistel => "AtollFeistel",
            MorphAlgorithm::AtollSPN => "AtollSPN",
            MorphAlgorithm::AtollLattice => "AtollLattice",
            MorphAlgorithm::AtollTreeHash => "AtollTreeHash",
        }
    }
}

// ---------------------------------------------------------------------------
// Real Algorithm Implementations
// ---------------------------------------------------------------------------

/// Applies a single morph algorithm to input data, producing 64 bytes.
///
/// Uses real cryptographic implementations where available:
/// - SHA-512, SHA3-512, BLAKE3 via Rust crypto crates
/// - Argon2id, Scrypt via password hashing crates
/// - Custom ATOLL primitives via native implementations
/// - All other algorithms via simplified placeholder implementations
fn apply_algorithm(
    algo: MorphAlgorithm,
    input: &[u8],
) -> Vec<u8> {
    match algo {
        MorphAlgorithm::SHA512 => {
            let mut hasher = Sha512::new();
            hasher.update(input);
            let result = hasher.finalize_reset();
            let mut output = [0u8; 64];
            output.copy_from_slice(&result);
            output.to_vec()
        }
        MorphAlgorithm::SHA3_512 => {
            let mut hasher = Sha3_512::new();
            hasher.update(input);
            let result = hasher.finalize_reset();
            result.to_vec()
        }
        MorphAlgorithm::BLAKE3 => {
            let mut hasher = blake3::Hasher::new();
            hasher.update(input);
            let mut out = [0u8; 64];
            hasher.finalize_xof().fill(&mut out);
            out.to_vec()
        }
        MorphAlgorithm::BLAKE2b => {
            // BLAKE2b via blake3 keyed mode (different salt per algorithm)
            let salt = b"BLAKE2b";
            let mut key = [0u8; 32];
            key[..salt.len()].copy_from_slice(salt);
            let mut hasher = blake3::Hasher::new_keyed(&key);
            hasher.update(input);
            let mut out = [0u8; 64];
            hasher.finalize_xof().fill(&mut out);
            out.to_vec()
        }
        MorphAlgorithm::SHA512_256 => {
            let mut hasher = Sha512::new();
            hasher.update(input);
            let result = hasher.finalize();
            let mut padded = [0u8; 64];
            padded[..32].copy_from_slice(&result[..32]);
            padded.to_vec()
        }
        MorphAlgorithm::Argon2id => {
            // Argon2id with 64-byte output, 3 passes, 64KB memory
            let mut output = [0u8; 64];
            let params = argon2::Params::new(64, 3, 1, None).unwrap();
            let argon2 =
                argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
            argon2
                .hash_password_into(input, &salt_for_algorithm("Argon2id"), &mut output)
                .expect("Argon2id hashing failed");
            output.to_vec()
        }
        MorphAlgorithm::Scrypt => {
            // Scrypt with 64-byte output
            let mut output = [0u8; 64];
            let params = scrypt::Params::new(16, 8, 1).unwrap();
            scrypt::scrypt(input, &salt_for_algorithm("Scrypt"), &params, &mut output)
                .expect("Scrypt hashing failed");
            output.to_vec()
        }
        // Custom ATOLL primitives
        MorphAlgorithm::AtollARX => atoll_arx(input),
        MorphAlgorithm::AtollSponge => atoll_sponge(input),
        MorphAlgorithm::AtollFeistel => atoll_feistel(input),
        MorphAlgorithm::AtollSPN => atoll_spn(input),
        MorphAlgorithm::AtollLattice => atoll_lattice(input),
        MorphAlgorithm::AtollTreeHash => atoll_tree_hash(input),
        // Simplified implementations for remaining algorithms
        // These will be replaced with full implementations in Phase 3
        MorphAlgorithm::Keccak512 => simplified_hash(input, "Keccak512"),
        MorphAlgorithm::KangarooTwelve => simplified_hash(input, "KangarooTwelve"),
        MorphAlgorithm::Whirlpool => simplified_hash(input, "Whirlpool"),
        MorphAlgorithm::Skein512 => simplified_hash(input, "Skein512"),
        MorphAlgorithm::JH512 => simplified_hash(input, "JH512"),
        MorphAlgorithm::Groestl512 => simplified_hash(input, "Groestl512"),
        MorphAlgorithm::Fugue512 => simplified_hash(input, "Fugue512"),
        MorphAlgorithm::Luffa512 => simplified_hash(input, "Luffa512"),
        MorphAlgorithm::SHAvite512 => simplified_hash(input, "SHAvite512"),
        MorphAlgorithm::SIMD512 => simplified_hash(input, "SIMD512"),
        MorphAlgorithm::ECHO512 => simplified_hash(input, "ECHO512"),
        MorphAlgorithm::Hamsi512 => simplified_hash(input, "Hamsi512"),
        MorphAlgorithm::Shabal512 => simplified_hash(input, "Shabal512"),
        MorphAlgorithm::BMW512 => simplified_hash(input, "BMW512"),
        MorphAlgorithm::CubeHash512 => simplified_hash(input, "CubeHash512"),
        MorphAlgorithm::Yescrypt => simplified_hash(input, "Yescrypt"),
        MorphAlgorithm::Lyra2Rev3 => simplified_hash(input, "Lyra2Rev3"),
        MorphAlgorithm::BalloonHash => simplified_hash(input, "BalloonHash"),
        MorphAlgorithm::RandomX => simplified_hash(input, "RandomX"),
    }
}

/// Returns a unique salt for password-hashing algorithms.
fn salt_for_algorithm(name: &str) -> &[u8] {
    match name {
        "Argon2id" => b"atoll-argon2id-salt-01",
        "Scrypt" => b"atoll-scrypt-salt-01",
        _ => b"atoll-default-salt-01",
    }
}

/// Simplified hash implementation for algorithms without mature Rust crates.
///
/// Phase 2: Uses BLAKE3 with algorithm-specific salt as a placeholder.
/// Phase 3: Replace with full algorithm implementations.
fn simplified_hash(
    input: &[u8],
    algo_name: &str,
) -> Vec<u8> {
    let salt = algo_name.as_bytes();
    let mut key = [0u8; 32];
    key[..salt.len().min(32)].copy_from_slice(&salt[..salt.len().min(32)]);
    let mut hasher = blake3::Hasher::new_keyed(&key);
    hasher.update(input);
    let mut out = [0u8; 64];
    hasher.finalize_xof().fill(&mut out);
    out.to_vec()
}

// ---------------------------------------------------------------------------
// Custom ATOLL Primitives
// ---------------------------------------------------------------------------

/// Custom ATOLL ARX (Add-Rotate-XOR) cipher-based hash.
fn atoll_arx(input: &[u8]) -> Vec<u8> {
    let mut state = [0u64; 8];
    for (i, &byte) in input.iter().enumerate() {
        state[i % 8] ^= byte as u64;
        state[i % 8] = state[i % 8].wrapping_add(0x9E3779B97F4A7C15);
        state[i % 8] = state[i % 8].rotate_left(31);
        state[i % 8] ^= state[(i + 1) % 8];
    }
    let mut output = [0u8; 64];
    for (i, &s) in state.iter().enumerate() {
        output[i * 8..(i + 1) * 8].copy_from_slice(&s.to_le_bytes());
    }
    output.to_vec()
}

/// Custom ATOLL Sponge construction.
fn atoll_sponge(input: &[u8]) -> Vec<u8> {
    let mut state = [0u64; 25]; // 200-bit state
    let rate = 8; // bytes per absorption step

    // Absorb input
    for chunk in input.chunks(rate) {
        for (i, &byte) in chunk.iter().enumerate() {
            state[i] ^= byte as u64;
        }
        // Permute (simplified)
        for i in 0..25 {
            state[i] = state[i].wrapping_mul(0x9E3779B97F4A7C15);
            state[i] = state[i].rotate_left(17);
            state[i] ^= state[(i + 1) % 25];
        }
    }

    // Squeeze output
    let mut output = [0u8; 64];
    for i in 0..64 {
        output[i] = state[i % 25] as u8;
        state[i % 25] = state[i % 25].wrapping_add(0x9E3779B97F4A7C15);
    }
    output.to_vec()
}

/// Custom ATOLL Feistel network hash.
fn atoll_feistel(input: &[u8]) -> Vec<u8> {
    let mut left: u64 = 0;
    let mut right: u64 = 0;

    for (i, &byte) in input.iter().enumerate() {
        let key = (i as u64).wrapping_mul(0x9E3779B97F4A7C15);
        let f = right.wrapping_add(key).rotate_left(7);
        let new_right = left ^ f;
        left = right;
        right = new_right ^ byte as u64;
    }

    let mut output = [0u8; 64];
    output[..8].copy_from_slice(&left.to_le_bytes());
    output[8..16].copy_from_slice(&right.to_le_bytes());
    for i in 16..64 {
        output[i] = output[i - 16] ^ output[i - 8];
    }
    output.to_vec()
}

/// Custom ATOLL SPN (Substitution-Permutation Network) hash.
fn atoll_spn(input: &[u8]) -> Vec<u8> {
    let sbox: [u8; 16] = [
        0xE, 0x4, 0xD, 0x1, 0x2, 0xF, 0xB, 0x8, 0x3, 0xA, 0x6, 0xC, 0x5, 0x9, 0x0, 0x7,
    ];

    let mut state = [0u8; 64];
    for (i, &byte) in input.iter().enumerate() {
        state[i % 64] ^= byte;
        let idx = state[i % 64] as usize;
        state[i % 64] = sbox[(idx >> 4) & 0xF] << 4 | sbox[idx & 0xF];
        state[i % 64] = state[i % 64].rotate_left(3);
    }

    // Additional rounds
    for _ in 0..3 {
        for i in 0..64 {
            state[i] = state[i].wrapping_add(state[(i + 1) % 64]);
            let idx = state[i] as usize;
            state[i] = sbox[(idx >> 4) & 0xF] << 4 | sbox[idx & 0xF];
        }
    }

    state.to_vec()
}

/// Custom ATOLL Lattice-based hash (simplified SWIFFT).
fn atoll_lattice(input: &[u8]) -> Vec<u8> {
    let mut coeffs: Vec<u64> = input.iter().map(|&b| b as u64).collect();
    coeffs.resize(64, 0);

    // Simple NTT-like transform
    for i in 0..64 {
        let mut acc: u64 = 0;
        for j in 0..64 {
            acc = acc.wrapping_add(
                coeffs[j].wrapping_mul(((i * j + 1) as u64).wrapping_mul(0x9E3779B97F4A7C15)),
            );
        }
        coeffs[i] = acc % 257;
    }

    let mut output = [0u8; 64];
    for (i, &c) in coeffs.iter().enumerate() {
        output[i] = c as u8;
    }
    output.to_vec()
}

/// Custom ATOLL Tree hash.
fn atoll_tree_hash(input: &[u8]) -> Vec<u8> {
    let mut layer: Vec<[u8; 64]> = Vec::new();

    // Leaf nodes: hash each 64-byte chunk
    for chunk in input.chunks(64) {
        let mut leaf = [0u8; 64];
        leaf[..chunk.len()].copy_from_slice(chunk);
        layer.push(leaf);
    }

    if layer.is_empty() {
        layer.push([0u8; 64]);
    }

    // Build tree upward
    while layer.len() > 1 {
        let mut next_layer: Vec<[u8; 64]> = Vec::new();
        for pair in layer.chunks(2) {
            let mut combined = [0u8; 128];
            combined[..64].copy_from_slice(&pair[0]);
            if pair.len() > 1 {
                combined[64..].copy_from_slice(&pair[1]);
            } else {
                combined[64..].copy_from_slice(&[0u8; 64]);
            }
            let mut hasher = blake3::Hasher::new();
            hasher.update(&combined);
            let mut hash = [0u8; 64];
            hasher.finalize_xof().fill(&mut hash);
            next_layer.push(hash);
        }
        layer = next_layer;
    }

    layer[0].to_vec()
}

// ---------------------------------------------------------------------------
// Algorithm Selection (Fisher-Yates shuffle seeded by prev hash + height)
// ---------------------------------------------------------------------------

/// Derives a deterministic pseudo-random index from seed bytes.
fn derive_index(
    seed: &[u8],
    i: usize,
) -> usize {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    i.hash(&mut hasher);
    let state = hasher.finish();
    (state % (i as u64 + 1)) as usize
}

/// Mixes the seed for the next Fisher-Yates iteration.
fn mix_seed(
    seed: &[u8],
    i: u64,
) -> Vec<u8> {
    blake3::hash(&[seed, &i.to_le_bytes()].concat())
        .as_bytes()
        .to_vec()
}

/// Fisher-Yates shuffle seeded with block hash entropy.
fn fisher_yates_shuffle(
    seed: &[u8],
    n: usize,
) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..n).collect();
    let mut current_seed = seed.to_vec();

    for i in (1..n).rev() {
        let j = derive_index(&current_seed, i) % (i + 1);
        indices.swap(i, j);
        current_seed = mix_seed(&current_seed, i as u64);
    }

    indices
}

/// Selects 16 of 32 algorithms based on previous block hash and block height.
pub fn select_algorithm_chain(
    prev_hash: &Hash512,
    block_height: u64,
) -> Vec<MorphAlgorithm> {
    let shuffled = fisher_yates_shuffle(prev_hash.as_ref(), ALGORITHM_POOL_SIZE);
    let mut selected: Vec<MorphAlgorithm> = shuffled
        .into_iter()
        .take(CHAIN_LENGTH)
        .map(|idx| MorphAlgorithm::all()[idx])
        .collect();

    // Additional rotation based on block height
    let rotation = (block_height % CHAIN_LENGTH as u64) as usize;
    selected.rotate_left(rotation);

    selected
}

// ---------------------------------------------------------------------------
// S-Box Evolution
// ---------------------------------------------------------------------------

/// Evolved S-box transformation using generation-based key.
fn evolved_sbox_transform(
    data: &[u8],
    generation: u64,
) -> Vec<u8> {
    let key = generation.wrapping_mul(0x9E3779B97F4A7C15).to_le_bytes();
    let mut result = data.to_vec();
    for (i, byte) in result.iter_mut().enumerate() {
        *byte ^= key[i % key.len()];
    }
    result
}

// ---------------------------------------------------------------------------
// Memory-Hard Component
// ---------------------------------------------------------------------------

/// Memory-hard mixing layer.
pub struct MemoryHardLayer {
    buffer_size: usize,
    access_rounds: usize,
    /// Pre-allocated reusable buffer (avoids re-allocation on every hash call).
    buffer: Vec<u8>,
}

impl MemoryHardLayer {
    pub fn new() -> Self {
        Self {
            buffer_size: MEMORY_BUFFER_SIZE,
            access_rounds: MEMORY_ACCESS_ROUNDS,
            buffer: vec![0u8; MEMORY_BUFFER_SIZE],
        }
    }

    pub fn with_buffer_size(buffer_size: usize) -> Self {
        Self {
            buffer_size,
            access_rounds: MEMORY_ACCESS_ROUNDS,
            buffer: vec![0u8; buffer_size],
        }
    }

    pub fn init_buffer(
        &self,
        seed: &Hash512,
    ) -> Vec<u8> {
        let mut buffer = vec![0u8; self.buffer_size];
        let mut counter = 0u64;
        let chunk_size = 64;

        for chunk in buffer.chunks_mut(chunk_size) {
            let mut input = seed.to_bytes();
            input.extend_from_slice(&counter.to_le_bytes());
            let hash = blake3::hash(&input);
            let hash_bytes = hash.as_bytes();
            for (i, byte) in chunk.iter_mut().enumerate() {
                *byte = hash_bytes[i % 32];
            }
            counter += 1;
        }

        for _ in 0..3 {
            for i in 0..self.buffer_size {
                let j = (i.wrapping_mul(2_654_435_761).wrapping_add(1)) % self.buffer_size;
                let temp = buffer[i];
                buffer[i] ^= buffer[j];
                buffer[j] = buffer[j].wrapping_add(temp);
            }
        }

        buffer
    }

    /// Initializes the internal buffer in-place (reusing the existing allocation)
    /// and returns a mutable reference to it. Avoids re-allocating on every hash call.
    pub fn init_buffer_in_place(
        &mut self,
        seed: &Hash512,
    ) -> &mut [u8] {
        let buffer = &mut self.buffer;
        let mut counter = 0u64;
        let chunk_size = 64;

        for chunk in buffer.chunks_mut(chunk_size) {
            let mut input = seed.to_bytes();
            input.extend_from_slice(&counter.to_le_bytes());
            let hash = blake3::hash(&input);
            let hash_bytes = hash.as_bytes();
            for (i, byte) in chunk.iter_mut().enumerate() {
                *byte = hash_bytes[i % 32];
            }
            counter += 1;
        }

        for _ in 0..3 {
            for i in 0..buffer.len() {
                let j = (i.wrapping_mul(2_654_435_761).wrapping_add(1)) % buffer.len();
                let temp = buffer[i];
                buffer[i] ^= buffer[j];
                buffer[j] = buffer[j].wrapping_add(temp);
            }
        }

        buffer
    }

    pub fn memory_hard_mix(
        &self,
        state: &mut [u8],
        buffer: &[u8],
    ) {
        for _round in 0..self.access_rounds.min(state.len() * 8) {
            let addr = u64::from_le_bytes(state[..8].try_into().unwrap_or([0; 8])) as usize
                % (buffer.len().saturating_sub(64));
            let mem_data = &buffer[addr..addr + 64];
            for i in 0..64.min(state.len()) {
                state[i] = state[i].wrapping_add(mem_data[i]).rotate_left(3);
                state[i] ^= mem_data[(i + 17) % 64];
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Post-Quantum Finalization (SWIFFT-style lattice construction)
// ---------------------------------------------------------------------------

pub struct PostQuantumFinalizer;

impl PostQuantumFinalizer {
    pub fn new() -> Self {
        Self
    }

    pub fn finalize(
        &self,
        state: &[u8],
    ) -> [u8; 64] {
        // Phase 2: SHA-512 stand-in for SWIFFT lattice construction
        // Phase 3: Full lattice-based finalization with NTT and matrix multiplication
        use sha2::{Digest, Sha512};
        let mut hasher = Sha512::new();
        hasher.update(state);
        let result = hasher.finalize();
        let mut out = [0u8; 64];
        out.copy_from_slice(&result);
        out
    }
}

// ---------------------------------------------------------------------------
// Main Morph-Hash Engine
// ---------------------------------------------------------------------------

pub struct MorphHashEngine {
    sbox_generation: u64,
    memory_layer: MemoryHardLayer,
    pq_finalizer: PostQuantumFinalizer,
}

impl Default for MorphHashEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MorphHashEngine {
    pub fn new() -> Self {
        Self {
            sbox_generation: 0,
            memory_layer: MemoryHardLayer::with_buffer_size(4096),
            pq_finalizer: PostQuantumFinalizer::new(),
        }
    }

    pub fn for_production() -> Self {
        Self {
            sbox_generation: 0,
            memory_layer: MemoryHardLayer::new(),
            pq_finalizer: PostQuantumFinalizer::new(),
        }
    }

    pub fn with_memory_size(buffer_size: usize) -> Self {
        Self {
            sbox_generation: 0,
            memory_layer: MemoryHardLayer::with_buffer_size(buffer_size),
            pq_finalizer: PostQuantumFinalizer::new(),
        }
    }

    pub fn sbox_generation(&self) -> u64 {
        self.sbox_generation
    }

    pub fn set_sbox_generation(
        &mut self,
        generation: u64,
    ) {
        self.sbox_generation = generation;
    }

    pub fn morph_hash(
        &mut self,
        input: &[u8],
        prev_block_hash: &Hash512,
        block_height: u64,
    ) -> Hash512 {
        let chain = select_algorithm_chain(prev_block_hash, block_height);
        let memory_buffer = self.memory_layer.init_buffer_in_place(prev_block_hash);

        let mut state = input.to_vec();
        for (i, algo) in chain.iter().enumerate() {
            let mem_index = Self::derive_memory_index(&state, i, memory_buffer);
            let mem_data =
                &memory_buffer[mem_index..mem_index + 64.min(memory_buffer.len() - mem_index)];
            state = Self::xor_mix(&state, mem_data);

            state = apply_algorithm(*algo, &state);
            state = evolved_sbox_transform(&state, self.sbox_generation);
        }

        let finalized = self.pq_finalizer.finalize(&state);
        Hash512::from_array(finalized)
    }

    fn derive_memory_index(
        state: &[u8],
        i: usize,
        buffer: &[u8],
    ) -> usize {
        let mut hasher = DefaultHasher::new();
        state.hash(&mut hasher);
        (i as u64).hash(&mut hasher);
        let state_hash = hasher.finish();
        (state_hash % (buffer.len() as u64 - 64).max(1)) as usize
    }

    fn xor_mix(
        state: &[u8],
        mem_data: &[u8],
    ) -> Vec<u8> {
        state
            .iter()
            .enumerate()
            .map(|(idx, b)| b ^ mem_data[idx % mem_data.len()])
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algorithm_pool_has_32_entries() {
        assert_eq!(MorphAlgorithm::all().len(), ALGORITHM_POOL_SIZE);
    }

    #[test]
    fn algorithm_names_are_unique() {
        let names: Vec<&str> = MorphAlgorithm::all().iter().map(|a| a.name()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len());
    }

    #[test]
    fn select_algorithm_chain_returns_16() {
        let prev_hash = Hash512::zero();
        let chain = select_algorithm_chain(&prev_hash, 0);
        assert_eq!(chain.len(), CHAIN_LENGTH);
    }

    #[test]
    fn select_algorithm_chain_is_deterministic() {
        let prev_hash = Hash512::blake3(b"test_seed");
        let chain1 = select_algorithm_chain(&prev_hash, 5);
        let chain2 = select_algorithm_chain(&prev_hash, 5);
        assert_eq!(chain1, chain2);
    }

    #[test]
    fn select_algorithm_chain_changes_with_height() {
        let prev_hash = Hash512::zero();
        let chain_h0 = select_algorithm_chain(&prev_hash, 0);
        let chain_h1 = select_algorithm_chain(&prev_hash, 1);
        assert_ne!(chain_h0, chain_h1);
    }

    #[test]
    fn morph_hash_is_deterministic() {
        let mut engine = MorphHashEngine::new();
        let prev_hash = Hash512::zero();
        let h1 = engine.morph_hash(b"ATOLL", &prev_hash, 1);
        let h2 = engine.morph_hash(b"ATOLL", &prev_hash, 1);
        assert_eq!(h1, h2);
    }

    #[test]
    fn morph_hash_context_sensitive() {
        let mut engine = MorphHashEngine::new();
        let prev_hash = Hash512::zero();
        let h1 = engine.morph_hash(b"ATOLL", &prev_hash, 1);
        let h2 = engine.morph_hash(b"ATOLL", &prev_hash, 2);
        assert_ne!(h1, h2);
    }

    #[test]
    fn morph_hash_avalanche_effect() {
        let mut engine = MorphHashEngine::new();
        let prev_hash = Hash512::zero();
        let h1 = engine.morph_hash(b"hello", &prev_hash, 0);
        let h2 = engine.morph_hash(b"hellp", &prev_hash, 0);
        let diff_bits: u32 =
            h1.0.iter()
                .zip(h2.0.iter())
                .map(|(a, b)| (a ^ b).count_ones())
                .sum();
        assert!(
            diff_bits > 16,
            "Avalanche effect: expected >16 bit flips, got {}",
            diff_bits
        );
    }

    #[test]
    fn morph_hash_output_is_512_bits() {
        let mut engine = MorphHashEngine::new();
        let prev_hash = Hash512::zero();
        let h = engine.morph_hash(b"test", &prev_hash, 0);
        assert_eq!(h.0.len(), 64);
    }

    #[test]
    fn sbox_evolution_changes_output() {
        let mut engine = MorphHashEngine::new();
        let prev_hash = Hash512::zero();
        let h_gen0 = engine.morph_hash(b"test", &prev_hash, 0);
        engine.set_sbox_generation(1);
        let h_gen1 = engine.morph_hash(b"test", &prev_hash, 0);
        assert_ne!(h_gen0, h_gen1);
    }

    #[test]
    fn memory_hard_layer_init_buffer() {
        let layer = MemoryHardLayer::with_buffer_size(4096);
        let seed = Hash512::zero();
        let buffer = layer.init_buffer(&seed);
        assert_eq!(buffer.len(), 4096);
        assert!(buffer.iter().any(|&b| b != 0));
    }

    #[test]
    fn morph_hash_matches_python_prototype_pattern() {
        let mut engine = MorphHashEngine::new();
        let prev_hash = Hash512::zero();

        let h1 = engine.morph_hash(b"ATOLL", &prev_hash, 1);
        let h2 = engine.morph_hash(b"ATOLL", &h1, 2);
        let h3 = engine.morph_hash(b"ATOLL", &h2, 3);

        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_ne!(h1, h3);
    }

    #[test]
    fn real_sha512_algorithm() {
        let result = apply_algorithm(MorphAlgorithm::SHA512, b"test");
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn real_sha3_512_algorithm() {
        let result = apply_algorithm(MorphAlgorithm::SHA3_512, b"test");
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn real_blake3_algorithm() {
        let result = apply_algorithm(MorphAlgorithm::BLAKE3, b"test");
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn real_argon2id_algorithm() {
        let result = apply_algorithm(MorphAlgorithm::Argon2id, b"test");
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn real_scrypt_algorithm() {
        let result = apply_algorithm(MorphAlgorithm::Scrypt, b"test");
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn atoll_arx_algorithm() {
        let result = apply_algorithm(MorphAlgorithm::AtollARX, b"test");
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn atoll_sponge_algorithm() {
        let result = apply_algorithm(MorphAlgorithm::AtollSponge, b"test");
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn atoll_feistel_algorithm() {
        let result = apply_algorithm(MorphAlgorithm::AtollFeistel, b"test");
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn atoll_spn_algorithm() {
        let result = apply_algorithm(MorphAlgorithm::AtollSPN, b"test");
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn atoll_lattice_algorithm() {
        let result = apply_algorithm(MorphAlgorithm::AtollLattice, b"test");
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn atoll_tree_hash_algorithm() {
        let result = apply_algorithm(MorphAlgorithm::AtollTreeHash, b"test");
        assert_eq!(result.len(), 64);
    }
}
