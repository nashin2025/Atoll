# Atoll Block Explorer Deployment Documentation

# Atoll Block Explorer Deployment Guide

This document describes how to deploy the Atoll Block Explorer.

## Overview

The Atoll Block Explorer is a web-based interface for browsing the Atoll blockchain. It consists of:

- **Frontend**: React + TypeScript + Vite (served via Nginx)
- **Backend API**: Node.js + Express (provides REST and GraphQL endpoints)
- **PostgreSQL**: Indexed blockchain data storage
- **Redis**: Caching and rate limiting
- **Nginx**: Reverse proxy with SSL termination

## Architecture

```
┌─────────────┐     ┌──────────────┐     ┌─────────────────┐
│   Users     │────▶│   Nginx      │────▶│  Explorer       │
│   (Web)     │     │   (SSL/TLS)  │     │  Frontend       │
└─────────────┘     └──────────────┘     └─────────────────┘
                                               │
                                               ▼
┌─────────────┐     ┌──────────────┐     ┌─────────────────┐
│   Users     │────▶│   Nginx      │────▶│  Explorer API   │────▶ PostgreSQL
│   (API)     │     │   (SSL/TLS)  │     │  (Node.js)      │────▶ Redis
└─────────────┘     └──────────────┘     └─────────────────┘
                                               │
                                               ▼
                                        ┌──────────────┐
                                        │  Atoll Node  │
                                        │  (RPC/WS)    │
                                        └──────────────┘
```

## Quick Start

### 1. Clone the repository

```bash
git clone https://github.com/nashin2025/Atoll.git
cd Atoll
```

### 2. Start the block explorer

```bash
cd explorer
docker compose up -d
```

### 3. Access the explorer

- **Frontend**: http://localhost:3000 (or https://localhost:443 with SSL)
- **API**: http://localhost:8080/api
- **Prometheus metrics**: Available via the Atoll node

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `VITE_API_URL` | `http://localhost:8080/api` | Frontend API URL |
| `VITE_NODE_URL` | `http://localhost:8081` | Atoll node RPC URL |
| `NODE_ENV` | `production` | Node.js environment |
| `PORT` | `8080` | API server port |
| `RPC_URL` | `http://atoll-node:8081` | Atoll node RPC endpoint |
| `WS_URL` | `ws://atoll-node:8082` | Atoll node WebSocket endpoint |
| `DATABASE_URL` | `postgres://atoll_explorer:atoll_explorer@postgres:5432/atoll_explorer` | PostgreSQL connection |
| `REDIS_URL` | `redis://redis:6379` | Redis connection |

### SSL/TLS

For production, place SSL certificates in `./ssl/`:
- `cert.pem`: SSL certificate
- `key.pem`: Private key

Obtain free certificates from Let's Encrypt:
```bash
certbot certonly --standalone -d explorer.atoll.network
```

## Ports

| Service | Port | Protocol |
|---------|------|----------|
| Explorer Frontend | 80/443 | HTTP/HTTPS |
| Explorer API | 8080 | HTTP |
| PostgreSQL | 5432 | TCP |
| Redis | 6379 | TCP |
| Nginx | 80/443 | HTTP/HTTPS |

## Architecture Components

### Indexer
The indexer reads blocks from the Atoll node via RPC and stores them in PostgreSQL for efficient querying.

### API Server
The API server provides REST and GraphQL endpoints for the frontend to query blockchain data.

### Frontend
The React frontend provides a user-friendly interface for browsing blocks, transactions, addresses, and validators.

## Security

- **SSL/TLS**: All traffic encrypted via TLS 1.2+
- **Security Headers**: CSP, X-Frame-Options, X-Content-Type-Options, X-XSS-Protection
- **Rate Limiting**: Per-IP rate limiting on API endpoints
- **CORS**: Configurable allowed origins
- **SSRF Protection**: Private IP blocking on proxy endpoints
- **No Secrets in Frontend**: No API keys or secrets in client-side code

## Scaling

- **Horizontal**: Add more frontend/API instances behind the load balancer
- **Database**: Add PostgreSQL read replicas for read-heavy workloads
- **Caching**: Redis caches frequently accessed data
- **CDN**: Use a CDN for static assets

## Troubleshooting

### Explorer not loading

Check if the frontend container is running:
```bash
docker compose ps
```

Check nginx logs:
```bash
docker logs atoll-explorer-nginx
```

### API errors

Check API logs:
```bash
docker logs atoll-explorer-api
```

Verify the Atoll node is reachable:
```bash
curl http://localhost:8081/health
```

### Database connection errors

Check PostgreSQL is running:
```bash
docker logs atoll-explorer-postgres
```

## Backup

Backup the PostgreSQL database:
```bash
docker exec atoll-explorer-postgres pg_dump -U atoll_explorer atoll_explorer > backup.sql
```

Restore:
```bash
cat backup.sql | docker exec -i atoll-explorer-postgres psql -U atoll_explorer atoll_explorer
```
