// Atoll Protocol Performance Benchmark Configuration
// Based on Hyperledger Blockchain Performance Metrics and DLPS framework

export interface BenchmarkConfig {
  // Network configuration
  rpcUrl: string;
  wsUrl: string;
  chainId: number;

  // Test environment
  nodeCount: number;
  nodeHardware: {
    cpuCores: number;
    memoryGb: number;
    storageType: 'ssd' | 'hdd';
    networkMbps: number;
  };
  geographicDistribution: 'local' | 'regional' | 'global';

  // Workload configuration
  workload: {
    transactionType: 'transfer' | 'contract-call' | 'mixed';
    payloadSizeBytes: number;
    gasLimit: number;
    gasPrice: number;
  };

  // Test phases
  phases: {
    // Baseline: low load to establish baseline
    baseline: {
      tps: number;
      durationSeconds: number;
    };
    // Ramp-up: step-wise increase to find knee
    rampUp: {
      startTps: number;
      endTps: number;
      stepTps: number;
      stepDurationSeconds: number;
    };
    // Sustained: hold at target rate for extended period
    sustained: {
      tps: number;
      durationSeconds: number;
    };
    // Spike: short burst above target
    spike: {
      tps: number;
      durationSeconds: number;
    };
  };

  // Success criteria
  successCriteria: {
    maxLatencyP99Ms: number;
    minTps: number;
    maxErrorRate: number;
  };
}

export const DEFAULT_CONFIG: BenchmarkConfig = {
  rpcUrl: 'http://localhost:8081',
  wsUrl: 'ws://localhost:8082',
  chainId: 777778, // Testnet

  nodeCount: 4,
  nodeHardware: {
    cpuCores: 8,
    memoryGb: 32,
    storageType: 'ssd',
    networkMbps: 1000,
  },
  geographicDistribution: 'local',

  workload: {
    transactionType: 'transfer',
    payloadSizeBytes: 250,
    gasLimit: 21000,
    gasPrice: 1,
  },

  phases: {
    baseline: {
      tps: 10,
      durationSeconds: 60,
    },
    rampUp: {
      startTps: 10,
      endTps: 1000,
      stepTps: 50,
      stepDurationSeconds: 30,
    },
    sustained: {
      tps: 500,
      durationSeconds: 600, // 10 minutes
    },
    spike: {
      tps: 2000,
      durationSeconds: 60,
    },
  },

  successCriteria: {
    maxLatencyP99Ms: 2000,
    minTps: 100,
    maxErrorRate: 0.01, // 1%
  },
};
