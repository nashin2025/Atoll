# Atoll Protocol Performance Benchmarks

# Performance Benchmarking for Atoll Protocol

This directory contains performance benchmarking tools and results for the Atoll Protocol.

## Benchmark Metrics

Based on Hyperledger Blockchain Performance Metrics and DLPS framework:

| Metric | Definition |
|--------|-----------|
| **Transaction Throughput (TPS)** | Valid transactions committed / time window |
| **Transaction Latency** | Submission to confirmation (p50, p95, p99) |
| **Goodput** | Valid transactions only (excludes invalid/failed) |
| **Error Rate** | Percentage of failed transactions |

## Benchmark Phases

1. **Baseline**: Low load (10 TPS) for 60 seconds
2. **Ramp-up**: Step-wise increase from 10 to 1000 TPS (50 TPS steps, 30s each)
3. **Sustained**: Hold at 500 TPS for 10 minutes
4. **Spike**: 2000 TPS for 1 minute

## Running Benchmarks

### Prerequisites

- Atoll node running (testnet or local)
- Node.js 20+
- TypeScript

### Run Benchmarks

```bash
cd bench
npm install
npm run build
npm run benchmark
```

### View Results

Results are saved to `bench/benchmark-results.json`.

## Success Criteria

| Metric | Target |
|--------|--------|
| Max Latency (p99) | < 2000ms |
| Min TPS | > 100 |
| Max Error Rate | < 1% |

## Reporting Results

When reporting benchmark results, include:

1. **Environment**: Node count, hardware specs, network topology
2. **Workload**: Transaction type, payload size, gas limit
3. **Throughput**: TPS (peak and sustained)
4. **Latency**: p50, p95, p99 distributions
5. **Error rate**: Percentage of failed transactions
6. **Duration**: How long each phase ran

## Example Report

```json
{
  "config": {
    "chainId": 777778,
    "nodeCount": 4,
    "nodeHardware": {
      "cpuCores": 8,
      "memoryGb": 32,
      "storageType": "ssd",
      "networkMbps": 1000
    }
  },
  "summary": {
    "totalTransactions": 50000,
    "confirmed": 49500,
    "failed": 500
  },
  "phases": [
    {
      "phase": "sustained",
      "tps": 498.5,
      "goodput": 495.0,
      "latencyMs": {
        "p50": 150,
        "p95": 450,
        "p99": 800
      },
      "errorRate": 0.01
    }
  ]
}
```
