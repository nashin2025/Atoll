import { useState } from 'react';
import { walletHistory, nodeGetBalance, formatInvokeError } from '@/tauri';
import { Button, ErrorDisplay, Input, Loading, Card, CardHeader, Badge } from '@/components/ui';

interface HistoryProps {
  onBack: () => void;
  nodeConnected: boolean;
}

export function History({ onBack, nodeConnected }: HistoryProps) {
  const [address, setAddress] = useState('');
  const [message, setMessage] = useState<string | null>(null);
  const [balance, setBalance] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleLookup = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setMessage(null);
    setBalance(null);
    try {
      if (nodeConnected) {
        const response = await nodeGetBalance({ address });
        setBalance(response.balance);
        setMessage(`Address ${response.address} has ${response.balance} ATOLL (live node)`);
      } else {
        const response = await walletHistory({ address });
        setMessage(response.message);
      }
    } catch (err) {
      setError(formatInvokeError(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-slate-950 p-6">
      <div className="max-w-2xl mx-auto">
        <header className="flex items-center justify-between mb-8">
          <Button variant="secondary" onClick={onBack} className="flex items-center gap-2 text-sm h-9 px-3">
            ← Back
          </Button>
          <h1 className="text-2xl font-bold text-white tracking-tight">History</h1>
        </header>

        {error && <ErrorDisplay message={error} />}

        <form onSubmit={handleLookup} className="mb-6">
          <Card>
            <CardHeader title={nodeConnected ? 'Address lookup (live node)' : 'Transaction history'} />
            <Input
              label="Address"
              placeholder="Address to look up"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
            />
            <Button type="submit" variant="secondary" disabled={loading || !address}>
              {nodeConnected ? 'Look up on node' : 'Look up history'}
            </Button>
          </Card>
        </form>

        <Card>
          <CardHeader title="Result" />
          {loading ? (
            <Loading className="h-6 w-6" />
          ) : balance !== null ? (
            <div className="space-y-2">
              <p className="text-sm text-slate-300">{message}</p>
              <p className="text-xs font-mono text-slate-500">
                Balance: {balance} ATOLL
              </p>
            </div>
          ) : message ? (
            <p className="text-slate-400 text-sm">{message}</p>
          ) : (
            <p className="text-slate-500 text-sm">
              {nodeConnected
                ? 'Enter an address above to look up its balance on the live node.'
                : 'Enter an address above to check history.'}
            </p>
          )}
        </Card>
      </div>
    </div>
  );
}
