"""
Test runner for Atoll Phase 0 Python prototypes.

Runs all prototype tests and reports results.
"""

from __future__ import annotations

import sys


def main() -> int:
    """Run all prototype tests."""
    print("=" * 60)
    print("Atoll Phase 0 Prototype Test Suite")
    print("=" * 60)

    # Run Morph-Hash tests
    print("\n[1/2] Running Morph-Hash prototype tests...")
    from prototypes.morph_hash import test_morph_hash

    test_morph_hash()

    # Run Penta-Consensus tests
    print("\n[2/2] Running Penta-Consensus prototype tests...")
    from prototypes.penta_consensus import test_penta_consensus

    test_penta_consensus()

    print("\n" + "=" * 60)
    print("All Phase 0 prototype tests PASSED")
    print("=" * 60)
    return 0


if __name__ == "__main__":
    sys.exit(main())
