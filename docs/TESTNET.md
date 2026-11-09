# Atoll Testnet Node Operator Documentation

# Atoll Testnet Node Operator Guide

This document describes how to run an Atoll testnet node.

## Overview

The Atoll testnet is a separate network from mainnet with its own chain ID (777778) and genesis block. Test ATOLL has no real value and is used for testing and development purposes only.

## Prerequisites

- Docker and Docker Compose
- At least 8GB RAM
- At least 100GB free disk space
- A stable internet connection

## Quick Start

### 1. Clone the repository

```bash
git clone https://github.com/nashin2025/Atoll.git
cd Atoll
```

### 2. Start the testnet node

```bash
cd scripts/deploy
docker compose -f docker-compose.testnet.yml up -d
```

### 3. Verify the node is running

```bash
# Check node health
curl http://localhost:8081/health

# Check peer count
curl http://localhost:8081/net_peerCount

# Check latest block
curl http://localhost:8081/eth_blockNumber
```

## Configuration

### Genesis File (`testnet-genesis.json`)

Contains the testnet genesis block configuration including chain ID, genesis timestamp, and initial token distribution.

### Network Config (`testnet-network.toml`)

Contains network settings including listen ports, bootstrap peers, and consensus parameters.

### Faucet (`faucet.toml`)

The testnet faucet distributes test ATOLL to testnet participants. Configure the faucet address, private key, and dispense amounts.

## Ports

| Service | Port | Protocol |
|---------|------|----------|
| P2P | 7778 | TCP |
| RPC | 8081 | TCP |
| WebSocket | 8082 | TCP |
| Metrics | 9091 | TCP |
| Faucet API | 8083 | TCP |
| Prometheus | 9090 | TCP |
| Grafana | 3000 | TCP |

## Monitoring

- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3000 (admin/atoll-testnet)
- **Node metrics**: http://localhost:9091/metrics

## Faucet

Request test ATOLL from the faucet:

```bash
curl -X POST http://localhost:8083/faucet \
  -H "Content-Type: application/json" \
  -d '{"address": "0xYOUR_ADDRESS"}'
```

## Stopping the Testnet

```bash
docker compose -f docker-compose.testnet.yml down
```

## Data Directory

Node data is stored in Docker volumes:
- `atoll-testnet-data`: Chain state and blocks
- `atoll-testnet-logs`: Node logs

## Troubleshooting

### Node not syncing

Check peer count:
```bash
curl http://localhost:8081/net_peerCount
```

If peer count is 0, check firewall settings and ensure port 7778 is accessible.

### High disk usage

The testnet stores all historical blocks. Periodically prune old data or increase disk capacity.

## Security

- Test ATOLL has no real value
- Do not use mainnet keys on testnet
- The faucet private key is for testing only
- All testnet data is ephemeral and may be reset
