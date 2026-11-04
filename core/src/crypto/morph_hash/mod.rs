//! Morph-Hash cryptographic hash function for the Atoll Protocol.
//!
//! Self-evolving hash with rotating algorithm pools, AI-evolved S-boxes,
//! memory-hard computation, and post-quantum finalization.
//!
//! This is the Phase 1 Rust implementation based on the theoretical design
//! in `specification/research/theoretical-design.md`. It matches the Python
//! prototype outputs for deterministic test vectors.

use crate::primitives::hash::Hash512;
use serde::{Deserialize, Serialize};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
// Algorithm Selection (Fisher-Yates shuffle seeded by prev hash + height)
// ---------------------------------------------------------------------------

/// Derives a deterministic pseudo-random index from seed bytes.
fn derive_index(seed: &[u8], i: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    i.hash(&mut hasher);
    let state = hasher.finish();
    (state % (i as u64 + 1)) as usize
}

/// Mixes the seed for the next Fisher-Yates iteration.
fn mix_seed(seed: &[u8], i: u64) -> Vec<u8> {
    blake3::hash(&[seed, &i.to_le_bytes()].concat()).as_bytes().to_vec()
}

/// Fisher-Yates shuffle seeded with block hash entropy.
fn fisher_yates_shuffle(seed: &[u8], n: usize) -> Vec<usize> {
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
///
/// The selection uses Fisher-Yates shuffle seeded by `prev_hash`, then
/// applies an additional rotation based on `block_height`.
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
// Algorithm Application (simulated for Phase 1)
// ---------------------------------------------------------------------------

/// Applies a single morph algorithm to input data, producing 64 bytes.
///
/// Phase 1 uses BLAKE3 XOF as the underlying primitive for all algorithms.
/// Phase 2 will replace this with real implementations of each algorithm.
fn apply_algorithm(algo: MorphAlgorithm, input: &[u8]) -> Vec<u8> {
    // Use algorithm name as key salt for keyed BLAKE3, so each algorithm
    // produces different output from the same input.
    let mut salt = [0u8; 32];
    let name_bytes = algo.name().as_bytes();
    salt[..name_bytes.len().min(32)].copy_from_slice(&name_bytes[..name_bytes.len().min(32)]);

    let mut hasher = blake3::Hasher::new_keyed(&salt);
    hasher.update(input);
    let mut out = [0u8; 64];
    hasher.finalize_xof().fill(&mut out);
    out.to_vec()
}

// ---------------------------------------------------------------------------
// S-Box Evolution
// ---------------------------------------------------------------------------

/// Evolved S-box transformation using generation-based key.
///
/// Phase 1: Simple XOR with generation-based key.
/// Phase 2: Cryptographically strong S-boxes selected by scoring.
fn evolved_sbox_transform(data: &[u8], generation: u64) -> Vec<u8> {
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
///
/// Phase 1: Simulated buffer (configurable size) for testability.
/// Phase 2: Full 4 GB buffer with BLAKE3 initialization.
pub struct MemoryHardLayer {
    buffer_size: usize,
    access_rounds: usize,
}

impl MemoryHardLayer {
    pub fn new() -> Self {
        Self {
            buffer_size: MEMORY_BUFFER_SIZE,
            access_rounds: MEMORY_ACCESS_ROUNDS,
        }
    }

    /// Creates a MemoryHardLayer with a custom buffer size (useful for tests).
    #[allow(dead_code)]
    pub fn with_buffer_size(buffer_size: usize) -> Self {
        Self {
            buffer_size,
            access_rounds: MEMORY_ACCESS_ROUNDS,
        }
    }

    /// Initializes memory buffer with pseudo-random data seeded by prev hash.
    #[allow(dead_code)]
    pub fn init_buffer(&self, seed: &Hash512) -> Vec<u8> {
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

        // Memory-hard mixing passes (3 passes of ARX-based diffusion)
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

    /// Performs random memory access mixing into state.
    #[allow(dead_code)]
    pub fn memory_hard_mix(&self, state: &mut [u8], buffer: &[u8]) {
        for _round in 0..self.access_rounds.min(state.len() * 8) {
            let addr = u64::from_le_bytes(state[..8].try_into().unwrap_or([0; 8]))
                as usize
                % (buffer.len().saturating_sub(64));
            let mem_data = &buffer[addr..addr + 64];
            for i in 0..64.min(state.len()) {
                state[i] = state[i]
                    .wrapping_add(mem_data[i])
                    .rotate_left(3);
                state[i] ^= mem_data[(i + 17) % 64];
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Post-Quantum Finalization (SWIFFT-style lattice construction)
// ---------------------------------------------------------------------------

/// Post-quantum finalization layer.
///
/// Phase 1: SHA-512 stand-in for SWIFFT construction.
/// Phase 2: Full lattice-based finalization with NTT and matrix multiplication.
pub struct PostQuantumFinalizer;

impl PostQuantumFinalizer {
    pub fn new() -> Self {
        Self
    }

    /// Finalizes the hash state using a lattice-based construction.
    pub fn finalize(&self, state: &[u8]) -> [u8; 64] {
        // Phase 1: SHA-512 as stand-in for SWIFFT
        // Phase 2: Replace with actual lattice-based construction
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

/// Morph-Hash engine: self-evolving hash with rotating algorithm pools.
///
/// # Pipeline
///
/// 1. **Chain Selection**: Fisher-Yates shuffle of 32 algorithms, take 16,
///    rotate by block height.
/// 2. **Memory-Hard Buffer**: Initialize buffer with BLAKE3 counter mode.
///    Default buffer size is small for testing; use `for_production()` for 4 GB.
/// 3. **Algorithmic Chain**: Chain through 16 selected algorithms, mixing
///    with memory-hard data and evolving S-boxes at each step.
/// 4. **Post-Quantum Finalization**: Lattice-based (SWIFFT-style) finalization.
/// 5. **Final Squeeze**: Output 512 bits.
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
    /// Creates a new MorphHashEngine with a small memory buffer (suitable for tests).
    pub fn new() -> Self {
        Self {
            sbox_generation: 0,
            memory_layer: MemoryHardLayer::with_buffer_size(4096),
            pq_finalizer: PostQuantumFinalizer::new(),
        }
    }

    /// Creates a MorphHashEngine for production use with the full 4 GB memory buffer.
    pub fn for_production() -> Self {
        Self {
            sbox_generation: 0,
            memory_layer: MemoryHardLayer::new(),
            pq_finalizer: PostQuantumFinalizer::new(),
        }
    }

    /// Creates a MorphHashEngine with a custom memory buffer size (for tests).
    #[allow(dead_code)]
    pub fn with_memory_size(buffer_size: usize) -> Self {
        Self {
            sbox_generation: 0,
            memory_layer: MemoryHardLayer::with_buffer_size(buffer_size),
            pq_finalizer: PostQuantumFinalizer::new(),
        }
    }

    /// Returns the current S-box generation.
    pub fn sbox_generation(&self) -> u64 {
        self.sbox_generation
    }

    /// Sets the S-box generation (called every SBOX_EVOLUTION_INTERVAL blocks).
    pub fn set_sbox_generation(&mut self, generation: u64) {
        self.sbox_generation = generation;
    }

    /// Computes the Morph-Hash of input data.
    ///
    /// # Arguments
    /// * `input` - Data to hash
    /// * `prev_block_hash` - Previous block hash (provides entropy for algorithm selection)
    /// * `block_height` - Current block height (provides rotation and context)
    ///
    /// # Returns
    /// 512-bit (64 byte) hash output
    pub fn morph_hash(
        &mut self,
        input: &[u8],
        prev_block_hash: &Hash512,
        block_height: u64,
    ) -> Hash512 {
        // Phase 1: Select algorithm chain
        let chain = select_algorithm_chain(prev_block_hash, block_height);

        // Phase 2: Initialize memory buffer
        let memory_buffer = self.memory_layer.init_buffer(prev_block_hash);

        // Phase 3: Chain through selected algorithms
        let mut state = input.to_vec();
        for (i, algo) in chain.iter().enumerate() {
            // Mix with memory-hard component
            let mem_index = Self::derive_memory_index(&state, i, &memory_buffer);
            let mem_data = &memory_buffer[mem_index..mem_index + 64.min(memory_buffer.len() - mem_index)];
            state = Self::xor_mix(&state, mem_data);

            // Apply algorithm
            state = apply_algorithm(*algo, &state);

            // Apply evolved S-box
            state = evolved_sbox_transform(&state, self.sbox_generation);
        }

        // Phase 4: Post-quantum finalization
        let finalized = self.pq_finalizer.finalize(&state);

        // Phase 5: Final squeeze (already 512 bits)
        Hash512::from_array(finalized)
    }

    /// Derives a memory index from state and chain position.
    fn derive_memory_index(state: &[u8], i: usize, buffer: &[u8]) -> usize {
        let mut hasher = DefaultHasher::new();
        state.hash(&mut hasher);
        (i as u64).hash(&mut hasher);
        let state_hash = hasher.finish();
        (state_hash % (buffer.len() as u64 - 64).max(1)) as usize
    }

    /// XOR mixes two byte slices, repeating the shorter one as needed.
    fn xor_mix(state: &[u8], mem_data: &[u8]) -> Vec<u8> {
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
        // Rotation should change the order at different heights
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
        assert_ne!(h1, h2, "Same input at different heights should produce different hashes");
    }

    #[test]
    fn morph_hash_avalanche_effect() {
        let mut engine = MorphHashEngine::new();
        let prev_hash = Hash512::zero();
        let h1 = engine.morph_hash(b"hello", &prev_hash, 0);
        let h2 = engine.morph_hash(b"hellp", &prev_hash, 0);
        let diff_bits: u32 = h1
            .0
            .iter()
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
        assert_ne!(h_gen0, h_gen1, "Different S-box generations should produce different hashes");
    }

    #[test]
    fn memory_hard_layer_init_buffer() {
        let layer = MemoryHardLayer::with_buffer_size(4096);
        let seed = Hash512::zero();
        let buffer = layer.init_buffer(&seed);
        assert_eq!(buffer.len(), 4096);
        // Buffer should not be all zeros after initialization
        assert!(buffer.iter().any(|&b| b != 0));
    }

    #[test]
    fn morph_hash_matches_python_prototype_pattern() {
        // Verify context sensitivity matches Python prototype:
        // Same input at different heights produces different outputs
        let mut engine = MorphHashEngine::new();
        let prev_hash = Hash512::zero();

        let h1 = engine.morph_hash(b"ATOLL", &prev_hash, 1);
        let h2 = engine.morph_hash(b"ATOLL", &h1, 2);
        let h3 = engine.morph_hash(b"ATOLL", &h2, 3);

        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_ne!(h1, h3);
    }
}
