import { useState } from 'react';
import { walletDeriveAddress, formatInvokeError } from '@/tauri';
import { Button, ErrorDisplay, Input, AddressDisplay, Card, CardHeader } from '@/components/ui';

interface DeriveAddressProps {
  onBack: () => void;
}

export function DeriveAddress({ onBack }: DeriveAddressProps) {
  const [publicKey, setPublicKey] = useState('');
  const [address, setAddress] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleDerive = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setAddress(null);
    try {
      const response = await walletDeriveAddress({ public_key: publicKey });
      setAddress(response.address);
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
          <h1 className="text-2xl font-bold text-white tracking-tight">Derive Address</h1>
        </header>

        {error && <ErrorDisplay message={error} />}

        <form onSubmit={handleDerive} className="space-y-6">
          <Card>
            <CardHeader title="Public key" />
            <Input
              label="Ed25519 public key (hex)"
              placeholder="64 hexadecimal characters (32 bytes)"
              value={publicKey}
              onChange={(e) => setPublicKey(e.target.value)}
            />
            <p className="text-xs text-slate-500 mt-2">
              Enter a 32-byte Ed25519 public key in hexadecimal to derive the corresponding Atoll address.
            </p>
            <Button type="submit" variant="primary" disabled={loading || !publicKey}>
              Derive address
            </Button>
          </Card>

          {address && (
            <Card>
              <CardHeader title="Derived address" />
              <div className="relative mb-4">
                <input
                  readOnly
                  value={address}
                  className="block w-full rounded-md border border-slate-600 bg-slate-950 text-slate-100 text-xs font-mono p-3 truncate"
                />
                <Button
                  variant="secondary"
                  className="absolute right-2 top-2 h-8 text-xs"
                  onClick={() => navigator.clipboard.writeText(address)}
                >
                  Copy
                </Button>
              </div>
              <p className="text-xs text-slate-500">
                Share this address to receive ATOLL.
              </p>
            </Card>
          )}
        </form>
      </div>
    </div>
  );
}
