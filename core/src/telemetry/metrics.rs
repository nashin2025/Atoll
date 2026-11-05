//! Prometheus metrics exposition for the Atoll Protocol node.
//!
//! Counters, gauges, and histograms track chain, network, and RPC telemetry.
//! All metrics are registered with a [`prometheus::Registry`] and exposed in
//! Prometheus text format at `GET /metrics`.

use prometheus::{Gauge, Histogram, HistogramOpts, IntCounter, Opts, Registry, TextEncoder};

/// Namespace prefix for every Atoll metric name.
const NAMESPACE: &str = "atoll";

/// Buckets shared by the latency histograms (milliseconds).
///
/// Ranges from sub-millisecond to one second, capturing both fast RPC
/// requests and the slower block validation path.
const LATENCY_BUCKETS: &[f64] = &[1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1_000.0];

/// Collection of protocol telemetry metrics.
///
/// Metric handles (`IntCounter`, `Gauge`, `Histogram`) are `Sync` and cheap
/// to clone, so [`Metrics`] can be shared behind an `Arc` across threads
/// without any interior locking.
pub struct Metrics {
    registry: Registry,
    blocks_validated: IntCounter,
    txs_processed: IntCounter,
    peers_connected: IntCounter,
    errors_total: IntCounter,
    block_height: Gauge,
    supply_atoll: Gauge,
    supply_mind: Gauge,
    block_validation_ms: Histogram,
    rpc_request_ms: Histogram,
}

impl Metrics {
    /// Creates a new metrics registry and registers all Atoll metrics.
    ///
    /// # Errors
    ///
    /// Returns a [`prometheus::Error`] if a metric with the same name is
    /// already registered in the registry.
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let blocks_validated = IntCounter::with_opts(Opts::new(
            format!("{NAMESPACE}_blocks_validated_total"),
            "Total number of blocks validated by the node",
        ))?;
        registry.register(Box::new(blocks_validated.clone()))?;

        let txs_processed = IntCounter::with_opts(Opts::new(
            format!("{NAMESPACE}_txs_processed_total"),
            "Total number of transactions processed by the node",
        ))?;
        registry.register(Box::new(txs_processed.clone()))?;

        let peers_connected = IntCounter::with_opts(Opts::new(
            format!("{NAMESPACE}_peers_connected_total"),
            "Total number of peer connections established",
        ))?;
        registry.register(Box::new(peers_connected.clone()))?;

        let errors_total = IntCounter::with_opts(Opts::new(
            format!("{NAMESPACE}_errors_total"),
            "Total number of errors encountered by the node",
        ))?;
        registry.register(Box::new(errors_total.clone()))?;

        let block_height = Gauge::with_opts(Opts::new(
            format!("{NAMESPACE}_block_height"),
            "Current chain tip height",
        ))?;
        registry.register(Box::new(block_height.clone()))?;

        let supply_atoll = Gauge::with_opts(Opts::new(
            format!("{NAMESPACE}_supply_atoll"),
            "Current total supply of ATOLL tokens",
        ))?;
        registry.register(Box::new(supply_atoll.clone()))?;

        let supply_mind = Gauge::with_opts(Opts::new(
            format!("{NAMESPACE}_supply_mind"),
            "Current total supply of MIND tokens",
        ))?;
        registry.register(Box::new(supply_mind.clone()))?;

        let block_validation_ms = Histogram::with_opts(
            HistogramOpts::new(
                format!("{NAMESPACE}_block_validation_ms"),
                "Time spent validating blocks, in milliseconds",
            )
            .buckets(LATENCY_BUCKETS.to_vec()),
        )?;
        registry.register(Box::new(block_validation_ms.clone()))?;

        let rpc_request_ms = Histogram::with_opts(
            HistogramOpts::new(
                format!("{NAMESPACE}_rpc_request_ms"),
                "Time spent processing RPC/REST requests, in milliseconds",
            )
            .buckets(LATENCY_BUCKETS.to_vec()),
        )?;
        registry.register(Box::new(rpc_request_ms.clone()))?;

        Ok(Self {
            registry,
            blocks_validated,
            txs_processed,
            peers_connected,
            errors_total,
            block_height,
            supply_atoll,
            supply_mind,
            block_validation_ms,
            rpc_request_ms,
        })
    }

    /// Increments the blocks validated counter by one.
    pub fn inc_blocks_validated(&self) {
        self.blocks_validated.inc();
    }

    /// Increments the transactions processed counter by `count`.
    pub fn inc_txs_processed(
        &self,
        count: u64,
    ) {
        self.txs_processed.inc_by(count);
    }

    /// Increments the peers connected counter by one.
    pub fn inc_peers_connected(&self) {
        self.peers_connected.inc();
    }

    /// Increments the errors total counter by one.
    pub fn inc_errors_total(&self) {
        self.errors_total.inc();
    }

    /// Sets the block height gauge to `height`.
    ///
    /// Prometheus gauges are `f64`, so the `u64` height is intentionally cast;
    /// block heights never approach `2^53` in practice.
    #[allow(clippy::cast_precision_loss)]
    pub fn set_block_height(
        &self,
        height: u64,
    ) {
        self.block_height.set(height as f64);
    }

    /// Sets the ATOLL supply gauge to `supply`.
    ///
    /// Prometheus gauges are `f64`, so the `u128` supply is intentionally cast;
    /// precision loss is inherent to float64 exposition.
    #[allow(clippy::cast_precision_loss)]
    pub fn set_supply_atoll(
        &self,
        supply: u128,
    ) {
        self.supply_atoll.set(supply as f64);
    }

    /// Sets the MIND supply gauge to `supply`.
    ///
    /// Prometheus gauges are `f64`, so the `u128` supply is intentionally cast;
    /// precision loss is inherent to float64 exposition.
    #[allow(clippy::cast_precision_loss)]
    pub fn set_supply_mind(
        &self,
        supply: u128,
    ) {
        self.supply_mind.set(supply as f64);
    }

    /// Observes a block validation duration in milliseconds.
    pub fn observe_block_validation_ms(
        &self,
        ms: f64,
    ) {
        self.block_validation_ms.observe(ms);
    }

    /// Observes an RPC/REST request duration in milliseconds.
    pub fn observe_rpc_request_ms(
        &self,
        ms: f64,
    ) {
        self.rpc_request_ms.observe(ms);
    }

    /// Gathers all metrics and serializes them in Prometheus text format.
    ///
    /// # Errors
    ///
    /// Returns a [`prometheus::Error`] if gathering a metric family fails.
    pub fn gather(&self) -> Result<String, prometheus::Error> {
        let metric_families = self.registry.gather();
        let encoder = TextEncoder::new();
        encoder.encode_to_string(&metric_families)
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new().expect("failed to initialise Prometheus metrics registry")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    /// Every metric family the task requires, by Prometheus name.
    const METRIC_FAMILIES: &[&str] = &[
        "atoll_blocks_validated_total",
        "atoll_txs_processed_total",
        "atoll_peers_connected_total",
        "atoll_errors_total",
        "atoll_block_height",
        "atoll_supply_atoll",
        "atoll_supply_mind",
        "atoll_block_validation_ms",
        "atoll_rpc_request_ms",
    ];

    #[test]
    fn metrics_register_all_required_families() {
        let metrics = Metrics::new().expect("metrics registry should build");
        let text = metrics.gather().expect("gather should succeed");

        for family in METRIC_FAMILIES {
            assert!(
                text.contains(family),
                "missing metric family {family} in output:\n{text}"
            );
        }
    }

    #[test]
    fn counters_and_gauges_reflect_updates() {
        let metrics = Metrics::new().expect("metrics registry should build");

        metrics.inc_blocks_validated();
        metrics.inc_txs_processed(3);
        metrics.inc_peers_connected();
        metrics.inc_errors_total();
        metrics.set_block_height(42);
        metrics.set_supply_atoll(1_000_000);
        metrics.set_supply_mind(500_000);
        metrics.observe_block_validation_ms(12.5);
        metrics.observe_rpc_request_ms(3.0);

        let text = metrics.gather().expect("gather should succeed");

        // HELP and TYPE lines must be present for every metric family.
        assert!(text.contains("# HELP atoll_blocks_validated_total"));
        assert!(text.contains("# TYPE atoll_blocks_validated_total counter"));
        assert!(text.contains("# TYPE atoll_block_height gauge"));
        assert!(text.contains("# TYPE atoll_block_validation_ms histogram"));
        assert!(text.contains("# TYPE atoll_rpc_request_ms histogram"));

        // Sample values must be reflected in the text output.
        assert!(text.contains("atoll_blocks_validated_total 1"));
        assert!(text.contains("atoll_txs_processed_total 3"));
        assert!(text.contains("atoll_block_height 42"));
        assert!(
            text.contains("atoll_supply_atoll 1000000")
                || text.contains("atoll_supply_atoll 1e+06")
        );
        assert!(text.contains("atoll_block_validation_ms_bucket"));
        assert!(text.contains("atoll_rpc_request_ms_bucket"));
    }

    #[test]
    fn metrics_are_thread_safe_shared_handles() {
        let metrics = Arc::new(Metrics::new().expect("metrics registry should build"));
        let clone = Arc::clone(&metrics);

        std::thread::spawn(move || {
            clone.inc_blocks_validated();
            clone.inc_txs_processed(7);
        })
        .join()
        .expect("worker thread should not panic");

        metrics.inc_blocks_validated();
        let text = metrics.gather().expect("gather should succeed");
        assert!(text.contains("atoll_blocks_validated_total 2"));
        assert!(text.contains("atoll_txs_processed_total 7"));
    }
}
