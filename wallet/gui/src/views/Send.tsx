import { useState } from 'react';
import {
  walletSend,
  nodeSendTx,
  formatInvokeError,
} from '@/tauri';
import { Button, ErrorDisplay, Input, AddressDisplay } from '@/components/ui';

interface SendProps {
  from: string;
  passphrase: string;
  onBack: () => void;
  onSendComplete?: () => void;
  nodeConnected: boolean;
}

export function Send({
  from,
  passphrase,
  onBack,
  onSendComplete,
  nodeConnected,
}: SendProps) {
  const [to, setTo] = useState('');
  const [amount, setAmount] = useState('');
  const [token, setToken] = useState('ATOLL');
  const [signedTx, setSignedTx] = useState<string | null>(null);
  const [submitted, setSubmitted] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setSignedTx(null);
    setSubmitted(false);
    try {
      const response = await walletSend({
        from,
        to,
        amount,
        token,
        passphrase,
      });
      setSignedTx(response.signed_tx);
      // The passphrase is no longer needed once the transaction is signed.
      onSendComplete?.();
    } catch (err) {
      setError(formatInvokeError(err));
    } finally {
      setLoading(false);
    }
  };

  const handleSubmitToNode = async () => {
    if (!signedTx) return;
    setLoading(true);
    setError(null);
    try {
      const response = await nodeSendTx({ signed_tx: signedTx });
      setSubmitted(response.accepted);
      if (!response.accepted) {
        setError(response.message);
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
          <h1 className="text-2xl font-bold text-white">Send ATOLL</h1>
        </header>

        {error && <ErrorDisplay message={error} />}

        <form onSubmit={handleSend} className="space-y-6">
          <div className="bg-slate-900/50 border border-slate-700/50 rounded-xl p-6">
            <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider mb-4">
              Transaction
            </h2>
            <Input
              label="From"
              value={from}
              onChange={() => {}}
              readOnly
              className="bg-slate-950"
            />
            <AddressDisplay address={from} shorten={false} />
            <Input
              label="To address"
              placeholder="0x… recipient address"
              value={to}
              onChange={(e) => setTo(e.target.value)}
            />
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <Input
                label="Amount (ATOLL)"
                type="text"
                placeholder="e.g. 10.5"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
              />
              <Input
                label="Token"
                value={token}
                onChange={(e) => setToken(e.target.value)}
              />
            </div>
            {!nodeConnected && (
              <p className="text-xs text-slate-500 mt-2">
                Offline signing only — signed transaction hex is produced for
                manual submission. Balance is validated against the offline
                ChainState.
              </p>
            )}
            {nodeConnected && (
              <p className="text-xs text-slate-500 mt-2">
                Sign offline, then submit to the connected node below.
              </p>
            )}
            <Button type="submit" variant="primary" disabled={loading || !to || !amount}>
              Sign transaction
            </Button>
          </div>

          {signedTx && (
            <div className="bg-slate-900/50 border border-indigo-500/50 rounded-xl p-6">
              <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider mb-3">
                Signed transaction hex
              </h2>
              <div className="relative mb-4">
                <input
                  readOnly
                  value={signedTx}
                  className="block w-full rounded-md border border-slate-600 bg-slate-950 text-slate-100 text-xs font-mono p-3 truncate"
                />
                <Button
                  variant="secondary"
                  className="absolute right-2 top-2 h-8 text-xs"
                  onClick={() => navigator.clipboard.writeText(signedTx)}
                >
                  Copy
                </Button>
              </div>
              {nodeConnected && (
                <div className="space-y-3">
                  <Button
                    variant="primary"
                    onClick={handleSubmitToNode}
                    disabled={loading || submitted}
                    className="w-full"
                  >
                    {loading ? 'Submitting…' : submitted ? 'Submitted' : 'Submit to node'}
                  </Button>
                  {submitted && (
                    <p className="text-xs text-emerald-400">
                      Transaction accepted by node.
                    </p>
                  )}
                </div>
              )}
              {!nodeConnected && (
                <p className="text-xs text-slate-500">
                  Submit this hex via a node RPC or block explorer when ready.
                </p>
              )}
            </div>
          )}
        </form>
      </div>
    </div>
  );
}
