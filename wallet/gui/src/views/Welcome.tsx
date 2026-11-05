import { useState } from 'react';
import { walletUnlock, walletNew, walletImport, formatInvokeError } from '@/tauri';
import { Button, ErrorDisplay, Input } from '@/components/ui';

type View = 'unlock' | 'create' | 'import';

interface WelcomeProps {
  onUnlocked: (addresses: string[], passphrase: string) => void;
}

export function Welcome({ onUnlocked }: WelcomeProps) {
  const [view, setView] = useState<View>('unlock');
  const [passphrase, setPassphrase] = useState('');
  const [newPassphrase, setNewPassphrase] = useState('');
  const [secretKey, setSecretKey] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleUnlock = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      const response = await walletUnlock({ passphrase });
      onUnlocked(response.addresses, passphrase);
    } catch (err) {
      setError(formatInvokeError(err));
    } finally {
      setLoading(false);
    }
  };

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      const response = await walletNew({ passphrase: newPassphrase });
      onUnlocked([response.address], newPassphrase);
    } catch (err) {
      setError(formatInvokeError(err));
    } finally {
      setLoading(false);
    }
  };

  const handleImport = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      const response = await walletImport({
        secret_key: secretKey,
        passphrase: newPassphrase,
      });
      onUnlocked([response.address], newPassphrase);
    } catch (err) {
      setError(formatInvokeError(err));
    } finally {
      setLoading(false);
    }
  };

  const renderUnlock = () => (
    <form onSubmit={handleUnlock} className="space-y-4">
      <Input
        label="Passphrase"
        type="password"
        placeholder="Enter your wallet passphrase"
        value={passphrase}
        onChange={(e) => setPassphrase(e.target.value)}
        disabled={loading}
      />
      <Button type="submit" disabled={loading || !passphrase}>
        Unlock wallet
      </Button>
    </form>
  );

  const renderCreate = () => (
    <form onSubmit={handleCreate} className="space-y-4">
      <Input
        label="Passphrase to encrypt keystore"
        type="password"
        placeholder="Choose a passphrase"
        value={newPassphrase}
        onChange={(e) => setNewPassphrase(e.target.value)}
        disabled={loading}
      />
      <Button type="submit" disabled={loading || !newPassphrase}>
        Create wallet
      </Button>
    </form>
  );

  const renderImport = () => (
    <form onSubmit={handleImport} className="space-y-4">
      <Input
        label="Secret key (hex)"
        type="text"
        placeholder="64 hexadecimal characters (32 bytes)"
        value={secretKey}
        onChange={(e) => setSecretKey(e.target.value)}
        disabled={loading}
      />
      <Input
        label="Passphrase to encrypt keystore"
        type="password"
        placeholder="Choose a passphrase"
        value={newPassphrase}
        onChange={(e) => setNewPassphrase(e.target.value)}
        disabled={loading}
      />
      <Button type="submit" disabled={loading || !secretKey || !newPassphrase}>
        Import wallet
      </Button>
    </form>
  );

  return (
    <div className="min-h-screen flex items-center justify-center bg-slate-900 p-4">
      <div className="w-full max-w-md space-y-6">
        <div className="text-center">
          <h1 className="text-3xl font-bold text-white tracking-tight">Atoll Wallet</h1>
          <p className="mt-1 text-slate-400 text-sm">
            Ed25519 keystore — offline signing — shared with the CLI
          </p>
        </div>

        {error && <ErrorDisplay message={error} />}

        {view === 'unlock' && (
          <div className="space-y-4">
            {renderUnlock()}
            <div className="border-t border-slate-700 pt-4 space-y-3">
              <p className="text-xs text-slate-500">
                No existing wallet? Create or import one below.
              </p>
              <Button
                variant="secondary"
                onClick={() => {
                  setView('create');
                  setError(null);
                }}
                className="w-full"
                disabled={loading}
              >
                Create new wallet
              </Button>
              <Button
                variant="secondary"
                onClick={() => {
                  setView('import');
                  setError(null);
                }}
                className="w-full"
                disabled={loading}
              >
                Import existing key
              </Button>
            </div>
          </div>
        )}

        {view === 'create' && (
          <div className="space-y-4">
            <h2 className="text-lg font-semibold text-white">Create new wallet</h2>
            <p className="text-sm text-slate-400">
              A new Ed25519 key will be generated and encrypted to your keystore.
            </p>
            {renderCreate()}
            <Button
              variant="secondary"
              onClick={() => {
                setView('unlock');
                setError(null);
              }}
              className="w-full"
              disabled={loading}
            >
              Back to unlock
            </Button>
          </div>
        )}

        {view === 'import' && (
          <div className="space-y-4">
            <h2 className="text-lg font-semibold text-white">Import wallet</h2>
            <p className="text-sm text-slate-400">
              Paste an existing Ed25519 secret key (hex) to import it.
            </p>
            {renderImport()}
            <Button
              variant="secondary"
              onClick={() => {
                setView('unlock');
                setError(null);
              }}
              className="w-full"
              disabled={loading}
            >
              Back to unlock
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
