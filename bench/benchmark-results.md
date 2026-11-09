# Atoll Protocol Performance Benchmark Results

## Environment

- **Chain ID**: 777778 (testnet)
- **Node Count**: 4
- **Hardware**: 8 vCPU, 32GB RAM, SSD, 1000Mbps
- **Distribution**: Local
- **Workload**: Transfer, 250 bytes payload, 21000 gas

## Results

| Phase | TPS | Goodput | Latency p50 | Latency p95 | Latency p99 | Error Rate |
|-------|-----|---------|-------------|-------------|-------------|------------|
| Baseline (10 TPS) | - | - | - | - | - | - |
| Ramp-up (peak) | - | - | - | - | - | - |
| Sustained (500 TPS) | - | - | - | - | - | - |
| Spike (2000 TPS) | - | - | - | - | - | - |

## Success Criteria

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Max Latency (p99) | < 2000ms | - | - |
| Min TPS | > 100 | - | - |
| Max Error Rate | < 1% | - | - |

## Notes

- Run benchmarks on testnet before mainnet
- Compare results across releases to detect regressions
- Document all environment parameters
- Run soak tests (hours at target rate) before mainnet
