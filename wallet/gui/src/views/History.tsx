import { useState } from "react";
import { walletHistory, nodeGetHistory, nodeGetBalance, formatInvokeError } from "@/tauri";
import { Button, ErrorDisplay, Input, Loading, Card, CardHeader } from "@/components/ui";

interface HistoryProps {
  onBack: () => void;
  nodeConnected: boolean;
}

export function History({ onBack, nodeConnected }: HistoryProps) {
  const [address, setAddress] = useState("");
  const [transactions, setTransactions] = useState<
    Array<{
      height: number;
      hash: string;
      from: string;
      to: string | null;
      value: number;
    }>
  >([]);
  const [balance, setBalance] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleLookup = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setTransactions([]);
    setBalance(null);
    try {
      if (nodeConnected) {
        const response = await nodeGetHistory({ address });
        setTransactions(response.transactions);
        const balResp = await nodeGetBalance({ address });
        setBalance(balResp.balance);
      } else {
        const response = await walletHistory({ address });
        setTransactions([]);
        setBalance(null);
        const msg = response.message;
        if (msg) {
          setError(msg);
        }
      }
    } catch (err) {
      setError(formatInvokeError(err));
    } finally {
      setLoading(false);
    }
  };

  const truncateHash = (hash: string) => {
    if (hash.length <= 16) return hash;
    return `${hash.slice(0, 6)}…${hash.slice(-8)}`;
  };

  return (
    <div className="min-h-screen bg-slate-950 p-6">
      <div className="max-w-2xl mx-auto">
        <header className="flex items-center justify-between mb-8">
          <Button
            variant="secondary"
            onClick={onBack}
            className="flex items-center gap-2 text-sm h-9 px-3"
          >
            ← Back
          </Button>
          <h1 className="text-2xl font-bold text-white tracking-tight">History</h1>
        </header>

        {error && <ErrorDisplay message={error} />}

        <form onSubmit={handleLookup} className="mb-6">
          <Card>
            <CardHeader
              title={nodeConnected ? "Address history (live node)" : "Transaction history"}
            />
            <Input
              label="Address"
              placeholder="Address to look up"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
            />
            <Button type="submit" variant="secondary" disabled={loading || !address}>
              {nodeConnected ? "Look up history" : "Look up history"}
            </Button>
          </Card>
        </form>

        <Card>
          <CardHeader title="Result" />
          {loading ? (
            <Loading className="h-6 w-6" />
          ) : nodeConnected ? (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <h3 className="text-sm font-semibold text-slate-300">
                  Address: {address.slice(0, 8)}…{address.slice(-4)}
                </h3>
                {balance !== null && (
                  <span className="text-xs text-slate-400">Balance: {balance} ATOLL</span>
                )}
              </div>
              {transactions.length === 0 ? (
                <p className="text-slate-500 text-sm text-center py-8">
                  No transactions found for this address.
                </p>
              ) : (
                <div className="overflow-x-auto">
                  <table className="w-full text-left border-collapse">
                    <thead>
                      <tr className="border-b border-slate-700/50">
                        <th className="px-3 py-2 text-xs font-semibold text-slate-400 uppercase tracking-wider">
                          Height
                        </th>
                        <th className="px-3 py-2 text-xs font-semibold text-slate-400 uppercase tracking-wider">
                          Hash
                        </th>
                        <th className="px-3 py-2 text-xs font-semibold text-slate-400 uppercase tracking-wider">
                          From
                        </th>
                        <th className="px-3 py-2 text-xs font-semibold text-slate-400 uppercase tracking-wider">
                          To
                        </th>
                        <th className="px-3 py-2 text-xs font-semibold text-slate-400 uppercase tracking-wider text-right">
                          Value (ATOLL)
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      {transactions.map((tx) => (
                        <tr
                          key={`${tx.height}-${tx.hash}`}
                          className="border-b border-slate-700/20 hover:bg-slate-800/30"
                        >
                          <td className="px-3 py-2 text-xs font-mono text-slate-300">
                            {tx.height}
                          </td>
                          <td className="px-3 py-2 text-xs font-mono text-slate-300 truncate max-w-[200px]">
                            {truncateHash(tx.hash)}
                          </td>
                          <td className="px-3 py-2 text-xs font-mono text-slate-300 truncate">
                            {tx.from}
                          </td>
                          <td className="px-3 py-2 text-xs font-mono text-slate-300 truncate">
                            {tx.to ?? "—"}
                          </td>
                          <td className="px-3 py-2 text-xs font-mono text-slate-300 text-right">
                            {tx.value}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          ) : (
            <p className="text-slate-500 text-sm">Enter an address above to check history.</p>
          )}
        </Card>
      </div>
    </div>
  );
}
