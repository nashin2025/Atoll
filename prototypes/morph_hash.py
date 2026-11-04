"""
Morph-Hash research prototype in Python.

This is NOT production code - it's for research validation only.
Implements the algorithm selection chain, memory-hard mixing,
S-box evolution, and post-quantum finalization as specified
in specification/research/theoretical-design.md.
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass, field
from typing import List


# 32 algorithm variants in the rotation pool
ALGORITHM_POOL: List[str] = [
    # Sponge-based
    "Keccak512",
    "SHA3_512",
    "KangarooTwelve",
    # Merkle-Damgård based
    "SHA512",
    "Whirlpool",
    "SHA512_256",
    # HAIFA-based
    "BLAKE2b",
    "BLAKE3",
    # Unique designs
    "Skein512",
    "JH512",
    "Groestl512",
    "Fugue512",
    "Luffa512",
    "SHAvite512",
    "SIMD512",
    "ECHO512",
    "Hamsi512",
    "Shabal512",
    "BMW512",
    "CubeHash512",
    # Memory-hard
    "Argon2id",
    "Scrypt",
    "Yescrypt",
    "Lyra2Rev3",
    "BalloonHash",
    # CPU-hard
    "RandomX",
    # Custom ATOLL primitives
    "AtollARX",
    "AtollSponge",
    "AtollFeistel",
    "AtollSPN",
    "AtollLattice",
    "AtollTreeHash",
]

POOL_SIZE = 32
CHAIN_LENGTH = 16
SBOX_EVOLUTION_INTERVAL = 10_000  # blocks
MEMORY_BUFFER_SIZE_SIM = 1024  # simulated buffer size (4 GB in production)


@dataclass
class MorphHashState:
    """State for the Morph-Hash engine."""

    algorithm_pool: List[str] = field(default_factory=lambda: ALGORITHM_POOL.copy())
    sbox_generation: int = 0


def derive_index(seed: bytes, i: int) -> int:
    """Derive a deterministic pseudo-random index from seed bytes."""
    # Use a simple LCG seeded from the input bytes
    state = int.from_bytes(seed[:8], "little") if len(seed) >= 8 else int.from_bytes(seed, "little")
    state = (state * 1_103_515_245 + 12_345) & 0x7FFFFFFF
    return state % (i + 1)


def mix_seed(seed: bytes, i: int) -> bytes:
    """Mix the seed for the next Fisher-Yates iteration."""
    return hashlib.blake2b(seed + i.to_bytes(8, "little"), digest_size=64).digest()


def fisher_yates_shuffle(seed: bytes, n: int) -> List[int]:
    """Fisher-Yates shuffle seeded with block hash entropy."""
    indices = list(range(n))
    current_seed = seed

    for i in range(n - 1, 0, -1):
        j = derive_index(current_seed, i) % (i + 1)
        indices[i], indices[j] = indices[j], indices[i]
        current_seed = mix_seed(current_seed, i)

    return indices


def select_algorithm_chain(
    prev_hash: bytes,
    block_height: int,
    pool_size: int = POOL_SIZE,
    chain_length: int = CHAIN_LENGTH,
) -> List[int]:
    """Select 16 of 32 algorithms based on prev hash and block height."""
    shuffled = fisher_yates_shuffle(prev_hash, pool_size)
    selected = shuffled[:chain_length]

    # Additional rotation based on block height
    rotation = block_height % chain_length
    selected = selected[rotation:] + selected[:rotation]

    return selected


def simulate_algorithm(algo_name: str, data: bytes) -> bytes:
    """Simulate applying a hash algorithm to data.

    Research prototype: uses different hash functions to simulate
    different algorithms. Production will use real implementations.
    """
    if algo_name.startswith("Atoll"):
        # Custom ATOLL primitives - simulate with blake2b + salt
        salt = algo_name.encode()[:16]
        return hashlib.blake2b(salt + data, digest_size=64).digest()

    algo_map = {
        "Blake2b": lambda d: hashlib.blake2b(d, digest_size=64).digest(),
        "Blake3": lambda d: hashlib.blake2b(d, digest_size=64).digest(),
        "Sha512": lambda d: hashlib.sha512(d).digest(),
        "Sha3_512": lambda d: hashlib.sha3_512(d).digest(),
        "Keccak512": lambda d: hashlib.sha3_512(d).digest(),
    }

    algo_func = algo_map.get(
        algo_name, lambda d: hashlib.blake2b(d, digest_size=64).digest()
    )
    return algo_func(data)


def evolved_sbox_transform(data: bytes, generation: int) -> bytes:
    """Apply evolved S-box transformation.

    Research prototype: simple XOR with generation-based key.
    Production will use cryptographically strong S-boxes.
    """
    key = (generation * 0x9E3779B97F4A7C15).to_bytes(8, "little")
    result = bytearray(data)
    for i in range(len(result)):
        result[i] ^= key[i % len(key)]
    return bytes(result)


def memory_hard_mix(state: bytearray, buffer_seed: bytes) -> None:
    """Simulate memory-hard mixing (4 GB buffer simulated).

    Research prototype: does not actually allocate 4 GB.
    Simulates the access pattern and mixing.
    """
    buffer_size = MEMORY_BUFFER_SIZE_SIM
    buffer = bytearray(buffer_size)

    # Initialize buffer with pseudo-random data
    for i in range(buffer_size):
        buffer[i] = buffer_seed[i % len(buffer_seed)]

    # Memory-hard mixing passes (3 passes)
    for _ in range(3):
        for i in range(buffer_size):
            j = (i * 2_654_435_761 + 1) % buffer_size
            buffer[i] ^= buffer[j]

    # Random access mixing into state
    for round_idx in range(min(64, 1_024)):  # Limited for prototype
        addr = int.from_bytes(state[0:4], "little") % (buffer_size - 8)
        mem_data = buffer[addr : addr + 8]
        for i in range(min(8, len(state))):
            state[i] = (state[i] + mem_data[i]) & 0xFF
            state[i] = ((state[i] << 3) | (state[i] >> 5)) & 0xFF  # rotate left 3


def lattice_finalize(state: bytes) -> bytes:
    """Post-quantum lattice-based finalization (simulated).

    Production: SWIFFT construction with NTT and matrix multiplication.
    Prototype: SHA-512 as stand-in.
    """
    return hashlib.sha512(state).digest()


def morph_hash(
    input_data: bytes,
    prev_block_hash: bytes,
    block_height: int,
    sbox_generation: int = 0,
) -> bytes:
    """Main Morph-Hash function.

    Args:
        input_data: Data to hash
        prev_block_hash: Previous block hash (512 bits / 64 bytes)
        block_height: Current block height
        sbox_generation: Current S-box generation

    Returns:
        512-bit (64 byte) hash output
    """
    # Phase 1: Select algorithm chain
    chain = select_algorithm_chain(prev_hash=prev_block_hash, block_height=block_height)

    # Phase 2: Initialize memory buffer
    state = bytearray(input_data)
    memory_hard_mix(state, prev_block_hash)

    # Phase 3: Chain through selected algorithms
    for i, algo_idx in enumerate(chain):
        algo_name = ALGORITHM_POOL[algo_idx]

        # Mix with memory-hard component
        mem_index = int.from_bytes(bytes(state[:4]), "little") % 1024
        mem_data = bytes([mem_index % 256]) * 64
        state = bytearray(
            (a ^ b) & 0xFF for a, b in zip(state, mem_data[: len(state)])
        )

        # Apply algorithm
        state = bytearray(simulate_algorithm(algo_name, bytes(state)))

        # Apply evolved S-box
        state = bytearray(
            evolved_sbox_transform(bytes(state), sbox_generation)
        )

    # Phase 4: Post-quantum finalization
    state = bytearray(lattice_finalize(bytes(state)))

    # Phase 5: Final squeeze to 512 bits
    return bytes(state[:64])


def test_morph_hash() -> None:
    """Basic test vectors for Morph-Hash prototype."""

    # Test 1: Empty input at genesis
    prev_hash = b"\x00" * 64
    result1 = morph_hash(b"", prev_hash, 0)
    assert len(result1) == 64, f"Expected 64 bytes, got {len(result1)}"
    print(f"Test 1 (empty input, height 0): {result1.hex()}")

    # Test 2: Same input, different height -> different output (context sensitivity)
    result2 = morph_hash(b"ATOLL", result1, 1)
    print(f"Test 2 (height 1):               {result2.hex()}")

    result3 = morph_hash(b"ATOLL", result2, 2)
    print(f"Test 3 (height 2):               {result3.hex()}")

    # Same input at different heights should produce different results
    assert result2 != result3, "Same input at different heights should differ"

    # Test 3: Deterministic - same inputs produce same output
    result4 = morph_hash(b"ATOLL", result1, 1)
    assert result2 == result4, "Morph-Hash must be deterministic"

    # Test 4: Small input change produces large output change (avalanche)
    result_a = morph_hash(b"hello", prev_hash, 0)
    result_b = morph_hash(b"hellp", prev_hash, 0)
    diff_bits = sum(
        bin(ba ^ bb).count("1") for ba, bb in zip(result_a, result_b)
    )
    assert diff_bits > 16, f"Avalanche effect: expected >16 bit flips, got {diff_bits}"
    print(f"Test 4 (avalanche): {diff_bits} bits changed")

    print("\nAll Morph-Hash prototype tests passed!")


if __name__ == "__main__":
    test_morph_hash()
