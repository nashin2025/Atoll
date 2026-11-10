const express = require('express');
const cors = require('cors');

const app = express();
app.use(cors({ origin: process.env.CORS_ORIGIN || 'http://localhost:5173' }));
app.use(express.json());

// API key authentication middleware
const API_KEY = process.env.EXPLORER_API_KEY;
if (!API_KEY) {
  console.warn('WARNING: EXPLORER_API_KEY not set — API is unprotected');
}

function requireApiKey(req, res, next) {
  if (!API_KEY) {
    return next(); // no key configured, allow (dev mode)
  }
  const key = req.headers['x-api-key'];
  if (!key || key !== API_KEY) {
    return res.status(401).json({ error: 'unauthorized' });
  }
  next();
}

app.use('/api/v1/*', requireApiKey);

// Health check endpoint
app.get('/health', (req, res) => {
  res.json({ status: 'ok', timestamp: new Date().toISOString() });
});

// Input validation helpers
function isValidHash(hash) {
  return /^[0-9a-fA-F]{64}$/.test(hash);
}

function isValidAddress(address) {
  return /^[1-9A-HJ-NP-Za-km-z]{26,35}$/.test(address);
}

// Basic API routes - proxy to atoll-node RPC
app.get('/api/v1/blocks/latest', async (req, res) => {
  try {
    const response = await fetch(`${process.env.RPC_URL || 'http://atoll-node:8081'}/rpc`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'atoll.getLatestBlock',
        params: [],
        id: 1
      })
    });
    const data = await response.json();
    res.json(data);
  } catch (err) {
    res.status(500).json({ error: 'internal server error' });
  }
});

app.get('/api/v1/transactions/:hash', async (req, res) => {
  try {
    const { hash } = req.params;
    if (!isValidHash(hash)) {
      return res.status(400).json({ error: 'invalid transaction hash' });
    }
    const response = await fetch(`${process.env.RPC_URL || 'http://atoll-node:8081'}/rpc`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'atoll.getTransaction',
        params: [{ hash }],
        id: 1
      })
    });
    const data = await response.json();
    res.json(data);
  } catch (err) {
    res.status(500).json({ error: 'internal server error' });
  }
});

app.get('/api/v1/address/:address', async (req, res) => {
  try {
    const { address } = req.params;
    if (!isValidAddress(address)) {
      return res.status(400).json({ error: 'invalid address' });
    }
    const response = await fetch(`${process.env.RPC_URL || 'http://atoll-node:8081'}/rpc`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'atoll.getAddress',
        params: [{ address }],
        id: 1
      })
    });
    const data = await response.json();
    res.json(data);
  } catch (err) {
    res.status(500).json({ error: 'internal server error' });
  }
});

const PORT = process.env.PORT || 8080;
app.listen(PORT, () => {
  console.log(`Atoll Explorer API listening on port ${PORT}`);
});
