//! Alerting for the Atoll Protocol node.
//!
//! Evaluates alert rules against current metrics and emits structured alert
//! events via [`tracing`]. Alert rules are loaded from `alerts.yml` (Prometheus
//! alerting-rule style) and evaluated on demand via [`AlertManager::evaluate`].

use crate::telemetry::metrics::Metrics;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Severity of an alert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Critical — immediate action required (chain stalled, no peers).
    Critical,
    /// High — action required soon (high error rate, high latency).
    High,
    /// Medium — investigate when convenient (elevated but not critical).
    Medium,
    /// Low — informational.
    Low,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Critical => write!(f, "critical"),
            AlertSeverity::High => write!(f, "high"),
            AlertSeverity::Medium => write!(f, "medium"),
            AlertSeverity::Low => write!(f, "low"),
        }
    }
}

impl std::str::FromStr for AlertSeverity {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "critical" => Ok(AlertSeverity::Critical),
            "high" => Ok(AlertSeverity::High),
            "medium" => Ok(AlertSeverity::Medium),
            "low" => Ok(AlertSeverity::Low),
            _ => Err(()),
        }
    }
}

/// A single alert rule: a metric condition, duration, severity, and message.
#[derive(Debug, Clone, Deserialize)]
pub struct AlertRule {
    /// Unique rule name (e.g. `ChainStalled`).
    pub alert: String,
    /// Human-readable description of the condition.
    pub expr: String,
    /// How long the condition must hold before firing.
    #[serde(rename = "for")]
    pub for_duration: String,
    /// Severity level.
    pub severity: AlertSeverity,
    /// Message template sent with the alert.
    pub annotations: HashMap<String, String>,
}

/// Current firing state of an alert rule.
#[derive(Debug, Clone, Serialize)]
pub struct ActiveAlert {
    pub alert: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub fired_at: u64,
    pub duration: String,
}

/// Manages alert rules and evaluates them against current metrics.
pub struct AlertManager {
    rules: Vec<AlertRule>,
    /// Last time each rule was observed firing (used for `for` duration).
    last_fired: HashMap<String, u64>,
}

impl AlertManager {
    /// Creates a new alert manager with no rules loaded.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            last_fired: HashMap::new(),
        }
    }

    /// Loads alert rules from a YAML file.
    ///
    /// File format:
    /// ```yaml
    /// groups:
    ///   - name: default
    ///     rules:
    ///       - alert: ChainStalled
    ///         expr: "atoll_block_height < 1"
    ///         for: "5m"
    ///         severity: critical
    ///         annotations:
    ///           summary: "Chain has stalled"
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load_rules(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let content = fs::read_to_string(path.as_ref())?;
        let doc: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut rules = Vec::new();
        if let Some(groups) = doc.get("groups").and_then(|g| g.as_sequence()) {
            for group in groups {
                if let Some(group_rules) = group.get("rules").and_then(|r| r.as_sequence()) {
                    for rule_val in group_rules {
                        let alert = rule_val
                            .get("alert")
                            .and_then(|a| a.as_str())
                            .ok_or_else(|| {
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "rule missing `alert` field",
                                )
                            })?
                            .to_string();

                        let expr = rule_val
                            .get("expr")
                            .and_then(|e| e.as_str())
                            .ok_or_else(|| {
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "rule missing `expr` field",
                                )
                            })?
                            .to_string();

                        let for_dur = rule_val
                            .get("for")
                            .and_then(|f| f.as_str())
                            .unwrap_or("0s")
                            .to_string();

                        let severity = rule_val
                            .get("severity")
                            .and_then(|s| s.as_str())
                            .unwrap_or("low")
                            .parse::<AlertSeverity>()
                            .unwrap_or(AlertSeverity::Low);

                        let annotations = rule_val
                            .get("annotations")
                            .and_then(|a| a.as_mapping())
                            .map(|m| {
                                m.iter()
                                    .filter_map(|(k, v)| {
                                        k.as_str().map(|k| {
                                            (k.to_string(), v.as_str().unwrap_or("").to_string())
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();

                        rules.push(AlertRule {
                            alert,
                            expr,
                            for_duration: for_dur,
                            severity,
                            annotations,
                        });
                    }
                }
            }
        }

        Ok(Self {
            rules,
            last_fired: HashMap::new(),
        })
    }

    /// Evaluates all rules against the current metrics snapshot.
    ///
    /// Returns a list of [`ActiveAlert`] for rules that are currently firing.
    /// Emits a structured `tracing::warn!`/`error!` event for each new alert.
    pub fn evaluate(&mut self, metrics: &Metrics) -> Vec<ActiveAlert> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut firing = Vec::new();

        for rule in &self.rules {
            let is_firing = Self::eval_expr(&rule.expr, metrics);

            if is_firing {
                let last = self.last_fired.get(&rule.alert).copied().unwrap_or(0);
                let duration_s = now.saturating_sub(last);

                // Only emit alert if the `for` duration has elapsed.
                let for_seconds = parse_duration(&rule.for_duration).unwrap_or(0);
                if duration_s >= for_seconds {
                    let message = self.render_message(&rule, metrics);
                    let message_for_log = message.clone();
                    let alert = ActiveAlert {
                        alert: rule.alert.clone(),
                        severity: rule.severity.clone(),
                        message,
                        fired_at: now,
                        duration: rule.for_duration.clone(),
                    };
                    match rule.severity {
                        AlertSeverity::Critical => tracing::error!(
                            target: "atoll::alerts",
                            alert = %rule.alert,
                            severity = %rule.severity,
                            message = %message_for_log,
                            "alert fired"
                        ),
                        AlertSeverity::High => tracing::warn!(
                            target: "atoll::alerts",
                            alert = %rule.alert,
                            severity = %rule.severity,
                            message = %message_for_log,
                            "alert fired"
                        ),
                        _ => tracing::info!(
                            target: "atoll::alerts",
                            alert = %rule.alert,
                            severity = %rule.severity,
                            message = %message_for_log,
                            "alert fired"
                        ),
                    }
                    firing.push(alert);
                }
                self.last_fired.insert(rule.alert.clone(), now);
            } else {
                self.last_fired.remove(&rule.alert);
            }
        }

        firing
    }

    /// Evaluates a simple expression against metrics.
    ///
    /// Supports comparisons: `atoll_block_height < 1`, `atoll_errors_total > 100`,
    /// `atoll_peers_connected_total < 1`, `atoll_block_validation_ms > 500`,
    /// `atoll_rpc_request_ms > 1000`.
    fn eval_expr(expr: &str, metrics: &Metrics) -> bool {
        let text = metrics.gather().unwrap_or_default();

        // Parse: metric_name <|>|<=|>=|== value
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 3 {
            return false;
        }

        let metric_name = parts[0];
        let op = parts[1];
        let value: f64 = match parts[2].parse() {
            Ok(v) => v,
            Err(_) => return false,
        };

        // Find the metric value in the text output.
        let metric_value = Self::extract_metric_value(&text, metric_name);

        match op {
            "<" => metric_value < value,
            "<=" => metric_value <= value,
            ">" => metric_value > value,
            ">=" => metric_value >= value,
            "==" => (metric_value - value).abs() < f64::EPSILON,
            "!=" => (metric_value - value).abs() >= f64::EPSILON,
            _ => false,
        }
    }

    /// Extracts the current value of a metric from Prometheus text output.
    fn extract_metric_value(text: &str, metric_name: &str) -> f64 {
        // Look for the last sample line for this metric (not HELP/TYPE lines).
        let mut last_value: f64 = 0.0;
        for line in text.lines() {
            if line.starts_with(metric_name) && !line.starts_with("#") {
                if let Some((_, val)) = line.rsplit_once(' ') {
                    if let Ok(v) = val.parse::<f64>() {
                        last_value = v;
                    }
                }
            }
        }
        last_value
    }

    /// Renders the alert message from the rule's annotation template.
    fn render_message(&self, rule: &AlertRule, _metrics: &Metrics) -> String {
        rule.annotations
            .get("summary")
            .cloned()
            .unwrap_or_else(|| format!("Alert {} is firing", rule.alert))
    }
}

/// Parses a Prometheus duration string (e.g. "5m", "30s", "1h") into seconds.
fn parse_duration(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return Some(0);
    }
    let (num, unit) = s.split_at(s.len() - 1);
    let num: u64 = num.parse().ok()?;
    let seconds = match unit {
        "s" => num,
        "m" => num * 60,
        "h" => num * 3600,
        "d" => num * 86400,
        _ => return None,
    };
    Some(seconds)
}

impl Default for AlertSeverity {
    fn default() -> Self {
        AlertSeverity::Low
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manager() -> AlertManager {
        AlertManager::new()
    }

    #[test]
    fn severity_display() {
        assert_eq!(AlertSeverity::Critical.to_string(), "critical");
        assert_eq!(AlertSeverity::High.to_string(), "high");
        assert_eq!(AlertSeverity::Medium.to_string(), "medium");
        assert_eq!(AlertSeverity::Low.to_string(), "low");
    }

    #[test]
    fn parse_duration_seconds() {
        assert_eq!(parse_duration("30s"), Some(30));
        assert_eq!(parse_duration("5m"), Some(300));
        assert_eq!(parse_duration("1h"), Some(3600));
        assert_eq!(parse_duration("1d"), Some(86400));
        assert_eq!(parse_duration("0s"), Some(0));
        assert_eq!(parse_duration("invalid"), None);
    }

    #[test]
    fn extract_metric_value_from_text() {
        let text = "# HELP atoll_block_height Current chain tip height\n\
                    # TYPE atoll_block_height gauge\n\
                    atoll_block_height 42\n";
        assert_eq!(
            AlertManager::extract_metric_value(text, "atoll_block_height"),
            42.0
        );
    }

    #[test]
    fn eval_expr_comparisons() {
        let metrics = Metrics::new().expect("metrics registry should build");
        metrics.set_block_height(100);

        // height < 200 -> true
        assert!(AlertManager::eval_expr("atoll_block_height < 200", &metrics));
        // height < 50 -> false
        assert!(!AlertManager::eval_expr("atoll_block_height < 50", &metrics));
        // height == 100 -> true
        assert!(AlertManager::eval_expr("atoll_block_height == 100", &metrics));
    }

    #[test]
    fn evaluate_returns_empty_when_no_rules() {
        let mut manager = test_manager();
        let metrics = Metrics::new().expect("metrics registry should build");
        let alerts = manager.evaluate(&metrics);
        assert!(alerts.is_empty());
    }

    #[test]
    fn evaluate_detects_chain_stalled() {
        let mut manager = test_manager();
        manager.rules.push(AlertRule {
            alert: "ChainStalled".to_string(),
            expr: "atoll_block_height < 1".to_string(),
            for_duration: "0s".to_string(),
            severity: AlertSeverity::Critical,
            annotations: HashMap::from([("summary".to_string(), "Chain stalled".to_string())]),
        });

        let metrics = Metrics::new().expect("metrics registry should build");
        // Block height is 0 -> chain stalled -> alert fires
        let alerts = manager.evaluate(&metrics);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert, "ChainStalled");
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);

        // Now set height to 100 -> no longer stalled -> no alert
        metrics.set_block_height(100);
        let alerts = manager.evaluate(&metrics);
        assert!(alerts.is_empty());
    }
}
