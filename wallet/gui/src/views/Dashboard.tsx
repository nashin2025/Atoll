import { useEffect, useState } from 'react';
import { walletBalance, formatInvokeError } from '@/tauri';
import { Button, ErrorDisplay, Loading, AddressDisplay } from '@/components/ui';

interface DashboardProps {
  address: string;
  passphrase: string;
  onLock: () => void;
  onSend: () => void;
  onHistory: () => void;
}

export function Dashboard({
  address,
  passphrase,
  onLock,
  onSend,
  onHistory,
}: DashboardProps) {
  const [balance, setBalance] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      setError(null);
      try {
        const response = await walletBalance({ address });
        if (!cancelled) setBalance(response.balance);
      } catch (err) {
        if (!cancelled) setError(formatInvokeError(err));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [address, passphrase]);

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await walletBalance({ address });
      setBalance(response.balance);
    } catch (err) {
      setError(formatInvokeError(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-slate-950 p-6">
      <div className="max-w-3xl mx-auto">
        <header className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-2xl font-bold text-white">Dashboard</h1>
            <p className="text-slate-400 text-sm">offline wallet</p>
          </div>
          <div className="flex gap-2">
            <Button variant="secondary" onClick={onLock} className="h-9 px-3 text-sm">
              Lock
            </Button>
          </div>
        </header>

        {error && <ErrorDisplay message={error} />}

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div className="bg-slate-900/50 border border-slate-700/50 rounded-xl p-6">
            <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider mb-2">
              Address
            </h2>
            <div className="flex items-center gap-3">
              <AddressDisplay address={address} shorten={true} />
              <Button
                variant="secondary"
                className="h-8 w-8 px-0 text-xs"
                onClick={() => navigator.clipboard.writeText(address)}
                title="Copy address"
              >
                Copy
              </Button>
            </div>
          </div>

          <div className="bg-slate-900/50 border border-slate-700/50 rounded-xl p-6">
            <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider mb-2">
              ATOLL Balance
            </h2>
            <div className="flex items-baseline gap-2">
              {loading ? (
                <Loading className="h-6 w-6" />
              ) : (
                <span className="text-4xl font-bold text-indigo-400">
                  {balance === null ? '–' : balance}
                </span>
              )}
              <span className="text-slate-500 text-sm">ATOLL</span>
            </div>
            {balance === 0 && (
              <p className="mt-2 text-xs text-slate-500">
                Offline ChainState — balance is 0 until a live node is connected.
              </p>
            )}
            <Button
              variant="secondary"
              onClick={refresh}
              className="mt-4 w-full text-xs"
              disabled={loading}
            >
              Refresh balance
            </Button>
          </div>
        </div>

        <div className="mt-8 grid grid-cols-1 md:grid-cols-3 gap-4">
          <Button variant="secondary" onClick={onSend} className="w-full">
            Send ATOLL
          </Button>
          <Button variant="secondary" onClick={onHistory} className="w-full">
            History
          </Button>
        </div>

        <footer className="mt-10 pt-6 border-t border-slate-800 text-xs text-slate-500">
          Keystore: ~/.atoll/keystore.json (shared with the CLI wallet)
        </footer>
      </div>
    </div>
  );
}
