import { useState } from 'react';
import { walletHistory, formatInvokeError } from '@/tauri';
import { Button, ErrorDisplay, Input, Loading } from '@/components/ui';

interface HistoryProps {
  onBack: () => void;
}

export function History({ onBack }: HistoryProps) {
  const [address, setAddress] = useState('');
  const [message, setMessage] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleHistory = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setMessage(null);
    try {
      const response = await walletHistory({ address });
      setMessage(response.message);
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
          <h1 className="text-2xl font-bold text-white">History</h1>
        </header>

        {error && <ErrorDisplay message={error} />}

        <form onSubmit={handleHistory} className="mb-6">
          <Input
            label="Address"
            placeholder="0x… address to look up"
            value={address}
            onChange={(e) => setAddress(e.target.value)}
          />
          <Button type="submit" variant="secondary" disabled={loading || !address}>
            Look up history
          </Button>
        </form>

        <div className="bg-slate-900/50 border border-slate-700/50 rounded-xl p-6">
          <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider mb-3">
            Transaction history
          </h2>
          {loading ? (
            <Loading className="h-6 w-6" />
          ) : message ? (
            <p className="text-slate-400 text-sm">{message}</p>
          ) : (
            <p className="text-slate-500 text-sm">
              Enter an address above to check history. MVP: offline mode, no
              transaction log available.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
