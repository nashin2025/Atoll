# Atoll Protocol Monitoring Documentation

# Monitoring and Observability for Atoll Protocol

This document describes the monitoring and observability setup for the Atoll Protocol network.

## Overview

The Atoll Protocol monitoring stack consists of:

- **Prometheus**: Metrics collection and storage
- **Grafana**: Visualization and dashboards
- **Alertmanager**: Alert routing and notifications
- **Node Exporter**: System metrics (CPU, memory, disk)
- **Custom Metrics**: Protocol-specific metrics exposed by the Atoll node

## Metrics Collected

### Node Metrics

| Metric | Description | Type |
|--------|-------------|------|
| `atoll_block_height` | Current block height | Gauge |
| `atoll_peer_count` | Connected peer count | Gauge |
| `atoll_transactions_total` | Total transactions processed | Counter |
| `atoll_request_duration_seconds` | Request duration histogram | Histogram |
| `atoll_request_errors_total` | Total request errors | Counter |

### System Metrics

| Metric | Description | Type |
|--------|-------------|------|
| `process_cpu_seconds_total` | CPU time consumed | Counter |
| `process_resident_memory_bytes` | Memory used | Gauge |
| `node_filesystem_avail_bytes` | Disk space available | Gauge |

### Explorer Metrics

| Metric | Description | Type |
|--------|-------------|------|
| `explorer_api_requests_total` | API requests | Counter |
| `explorer_api_errors_total` | API errors | Counter |

## Alerts

### Critical

| Alert | Condition | Action |
|-------|-----------|--------|
| AtollNodeDown | Node is down for 1 minute | PagerDuty alert, page on-call engineer |
| AtollExplorerDown | Explorer API is down for 1 minute | PagerDuty alert, page on-call engineer |

### Warning

| Alert | Condition | Action |
|-------|-----------|--------|
| AtollNodeNotSyncing | Node is 5+ blocks behind | Investigate network connectivity |
| AtollLowPeerCount | Fewer than 5 peers | Check firewall, port forwarding |
| AtollHighErrorRate | Error rate > 5% | Investigate node logs |
| AtollHighLatency | p99 latency > 2s | Investigate node performance |
| AtollExplorerHighErrorRate | API error rate > 5% | Investigate API logs |

### Info

| Alert | Condition | Action |
|-------|-----------|--------|
| AtollUnusualTxVolume | > 100 TPS | Monitor for potential attack |
| AtollLargeValueTransfer | > 1M ATOLL | Log for audit trail |

## Dashboards

### Main Dashboard

The main Grafana dashboard (`grafana-dashboard.json`) displays:

- Node status (up/down)
- Block height
- Peer count
- Transaction rate (TPS)
- Transaction latency (p99)
- Error rate
- CPU usage
- Memory usage
- Disk usage
- Active alerts

Access at: http://localhost:3000 (admin/atoll-testnet)

## Alertmanager Configuration

Configure Alertmanager to send notifications via:

- Email
- Slack
- PagerDuty
- Webhook

Example `alertmanager.yml`:

```yaml
route:
  receiver: 'default-receiver'
  group_by: ['alertname']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 12h

receivers:
  - name: 'default-receiver'
    email_configs:
      - to: 'alerts@atoll.network'
    slack_configs:
      - channel: '#alerts'
        api_url: 'https://hooks.slack.com/services/...'
```

## Best Practices

1. **Monitor from outside**: Monitor from outside the node network to detect network issues
2. **Alert on symptoms, not causes**: Alert on user-visible symptoms (node down, high latency) rather than internal causes
3. **Document runbooks**: Create runbooks for each alert with remediation steps
4. **Test alerts**: Regularly test that alerts fire correctly
5. **Retain data**: Keep metrics for at least 30 days for trend analysis
6. **Use recording rules**: Pre-aggregate expensive queries for faster dashboard loading
