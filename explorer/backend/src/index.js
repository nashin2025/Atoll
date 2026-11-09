const express = require('express');
const cors = require('cors');

const app = express();
app.use(cors());
app.use(express.json());

// Health check endpoint
app.get('/health', (req, res) => {
  res.json({ status: 'ok', timestamp: new Date().toISOString() });
});

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
    res.status(500).json({ error: err.message });
  }
});

app.get('/api/v1/transactions/:hash', async (req, res) => {
  try {
    const { hash } = req.params;
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
    res.status(500).json({ error: err.message });
  }
});

app.get('/api/v1/address/:address', async (req, res) => {
  try {
    const { address } = req.params;
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
    res.status(500).json({ error: err.message });
  }
});

const PORT = process.env.PORT || 8080;
app.listen(PORT, () => {
  console.log(`Atoll Explorer API listening on port ${PORT}`);
});
