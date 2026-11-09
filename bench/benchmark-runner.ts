// Atoll Protocol Performance Benchmark Runner
//
// Measures:
//   - Transaction Throughput (TPS): valid transactions committed / time window
//   - Transaction Latency: submission to confirmation (p50, p95, p99)
//   - Goodput: valid transactions only (excludes invalid/failed)
//   - Error rate: percentage of failed transactions
//
// Best practices:
//   - Measure latency distribution, not just average
//   - Report sustained vs peak throughput
//   - Document all environment parameters
//   - Step-wise ramp to find the knee
//   - Soak runs at target rate for extended periods

import { writeFileSync } from 'fs';
import { join } from 'path';
import { DEFAULT_CONFIG, BenchmarkConfig } from './benchmark-config';

interface TransactionResult {
  submittedAt: number;
  confirmedAt: number | null;
  success: boolean;
  error: string | null;
  txHash: string;
}

interface BenchmarkResult {
  config: BenchmarkConfig;
  phase: string;
  startTime: number;
  endTime: number;
  durationSeconds: number;
  totalSubmitted: number;
  totalConfirmed: number;
  totalFailed: number;
  tps: number;
  goodput: number;
  latencyMs: {
    p50: number;
    p95: number;
    p99: number;
    p999: number;
    min: number;
    max: number;
    mean: number;
  };
  errorRate: number;
  results: TransactionResult[];
}

class BenchmarkRunner {
  private config: BenchmarkConfig;
  private results: TransactionResult[] = [];

  constructor(config: BenchmarkConfig) {
    this.config = config;
  }

  async run(): Promise<void> {
    console.log('🚀 Starting Atoll Protocol Performance Benchmark');
    console.log(`Chain ID: ${this.config.chainId}`);
    console.log(`RPC URL: ${this.config.rpcUrl}`);
    console.log(`Node count: ${this.config.nodeCount}`);
    console.log(`Hardware: ${this.config.nodeHardware.cpuCores} vCPU, ${this.config.nodeHardware.memoryGb}GB RAM`);
    console.log('');

    // Phase 1: Baseline
    await this.runPhase('baseline', this.config.phases.baseline);

    // Phase 2: Ramp-up
    await this.runPhase('rampUp', this.config.phases.rampUp);

    // Phase 3: Sustained
    await this.runPhase('sustained', this.config.phases.sustained);

    // Phase 4: Spike
    await this.runPhase('spike', this.config.phases.spike);

    // Generate report
    this.generateReport();
  }

  private async runPhase(phaseName: string, phase: { tps: number; durationSeconds: number; startTps?: number; endTps?: number; stepTps?: number; stepDurationSeconds?: number }): Promise<void> {
    console.log(`\n📊 Phase: ${phaseName}`);
    console.log(`   Target TPS: ${phase.tps}`);
    console.log(`   Duration: ${phase.durationSeconds}s`);

    const startTime = Date.now();
    const results: TransactionResult[] = [];

    if (phaseName === 'rampUp' && phase.startTps && phase.endTps && phase.stepTps && phase.stepDurationSeconds) {
      // Step-wise ramp-up to find the knee
      for (let tps = phase.startTps; tps <= phase.endTps; tps += phase.stepTps) {
        console.log(`   Step: ${tps} TPS for ${phase.stepDurationSeconds}s`);
        const stepResults = await this.runConstantLoad(tps, phase.stepDurationSeconds);
        results.push(...stepResults);
      }
    } else {
      // Constant load
      const tps = phase.tps;
      const results = await this.runConstantLoad(tps, phase.durationSeconds);
      this.results.push(...results);
    }

    const endTime = Date.now();
    const phaseResult = this.analyzeResults(startTime, endTime, results, phaseName);
    console.log(`   TPS: ${phaseResult.tps.toFixed(2)}`);
    console.log(`   Goodput: ${phaseResult.goodput.toFixed(2)}`);
    console.log(`   Latency p99: ${phaseResult.latencyMs.p99.toFixed(2)}ms`);
    console.log(`   Error rate: ${(phaseResult.errorRate * 100).toFixed(2)}%`);
  }

  private async runConstantLoad(tps: number, durationSeconds: number): Promise<TransactionResult[]> {
    const results: TransactionResult[] = [];
    const intervalMs = 1000 / tps;
    const endTime = Date.now() + durationSeconds * 1000;

    let txCount = 0;
    while (Date.now() < endTime) {
      const start = Date.now();
      const txResult = await this.submitTransaction(txCount);
      results.push(txResult);
      txCount++;

      // Wait for next interval
      const elapsed = Date.now() - start;
      if (elapsed < intervalMs) {
        await new Promise(resolve => setTimeout(resolve, intervalMs - elapsed));
      }
    }

    return results;
  }

  private async submitTransaction(txIndex: number): Promise<TransactionResult> {
    const submittedAt = Date.now();
    const txHash = `0x${txIndex.toString(16).padStart(64, '0')}`; // Mock for now

    try {
      // Simulate submitting transaction to node
      // In production, this would call the actual RPC
      await new Promise(resolve => setTimeout(resolve, Math.random() * 50 + 10));

      const confirmedAt = Date.now();
      return {
        submittedAt,
        confirmedAt,
        success: true,
        error: null,
        txHash,
      };
    } catch (error) {
      return {
        submittedAt,
        confirmedAt: null,
        success: false,
        error: error instanceof Error ? error.message : String(error),
        txHash,
      };
    }
  }

  private analyzeResults(startTime: number, endTime: number, results: TransactionResult[], phaseName: string): BenchmarkResult {
    const durationSeconds = (endTime - startTime) / 1000;
    const totalSubmitted = results.length;
    const totalConfirmed = results.filter(r => r.success).length;
    const totalFailed = results.filter(r => !r.success).length;

    const tps = totalSubmitted / durationSeconds;
    const goodput = totalConfirmed / durationSeconds;
    const errorRate = totalFailed / totalSubmitted;

    const latencies = results
      .filter(r => r.success && r.confirmedAt)
      .map(r => r.confirmedAt! - r.submittedAt)
      .sort((a, b) => a - b);

    const latencyMs = {
      p50: this.percentile(latencies, 50),
      p95: this.percentile(latencies, 95),
      p99: this.percentile(latencies, 99),
      p999: this.percentile(latencies, 99.9),
      min: latencies[0] || 0,
      max: latencies[latencies.length - 1] || 0,
      mean: latencies.reduce((a, b) => a + b, 0) / latencies.length || 0,
    };

    const result: BenchmarkResult = {
      config: this.config,
      phase: phaseName,
      startTime,
      endTime,
      durationSeconds,
      totalSubmitted,
      totalConfirmed,
      totalFailed,
      tps,
      goodput,
      latencyMs,
      errorRate,
      results,
    };

    this.results.push(...results);
    return result;
  }

  private percentile(sorted: number[], p: number): number {
    if (sorted.length === 0) return 0;
    const index = (p / 100) * (sorted.length - 1);
    const lower = Math.floor(index);
    const upper = Math.ceil(index);
    if (lower === upper) return sorted[lower];
    return sorted[lower] + (sorted[upper] - sorted[lower]) * (index - lower);
  }

  private generateReport(): void {
    const report = {
      config: this.config,
      summary: {
        totalPhases: 4,
        totalTransactions: this.results.length,
        totalConfirmed: this.results.filter(r => r.success).length,
        totalFailed: this.results.filter(r => !r.success).length,
      },
      results: this.results,
    };

    const reportPath = join(__dirname, 'benchmark-results.json');
    writeFileSync(reportPath, JSON.stringify(report, null, 2));
    console.log(`\n📊 Benchmark report saved to ${reportPath}`);
    console.log('\n📈 Summary:');
    console.log(`   Total transactions: ${report.summary.totalTransactions}`);
    console.log(`   Confirmed: ${report.summary.totalConfirmed}`);
    console.log(`   Failed: ${report.summary.totalFailed}`);
  }
}

// Run if executed directly
if (require.main === module) {
  const runner = new BenchmarkRunner(DEFAULT_CONFIG);
  runner.run().catch(console.error);
}

export { BenchmarkRunner, DEFAULT_CONFIG };
