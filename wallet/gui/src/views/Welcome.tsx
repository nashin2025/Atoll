import { useState } from "react";
import { walletUnlock, walletNew, walletImport, formatInvokeError } from "@/tauri";
import { Button, ErrorDisplay, Input, Card, CardHeader } from "@/components/ui";

type View = "unlock" | "create" | "import";

interface WelcomeProps {
  onUnlocked: (addresses: string[], passphrase: string) => void;
}

export function Welcome({ onUnlocked }: WelcomeProps) {
  const [view, setView] = useState<View>("unlock");
  const [passphrase, setPassphrase] = useState("");
  const [newPassphrase, setNewPassphrase] = useState("");
  const [secretKey, setSecretKey] = useState("");
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

  return (
    <div className="min-h-screen bg-slate-900 flex items-center justify-center p-4">
      <div className="w-full max-w-md">
        <div className="text-center mb-8">
          <div className="flex items-center justify-center gap-2 mb-4">
            <span className="text-3xl">🪙</span>
          </div>
          <h1 className="text-3xl font-bold text-white tracking-tight">Atoll Wallet</h1>
          <p className="mt-2 text-slate-400 text-sm">
            Ed25519 keystore · Offline signing · Shared with CLI
          </p>
        </div>

        {error && <ErrorDisplay message={error} />}

        {view === "unlock" && (
          <>
            <Card>
              <CardHeader title="Unlock wallet" />
              <form onSubmit={handleUnlock} className="space-y-4">
                <Input
                  label="Passphrase"
                  type="password"
                  autoComplete="new-password"
                  placeholder="Enter your wallet passphrase"
                  value={passphrase}
                  onChange={(e) => setPassphrase(e.target.value)}
                  disabled={loading}
                />
                <Button
                  type="submit"
                  variant="primary"
                  disabled={loading || !passphrase}
                  className="w-full"
                >
                  Unlock wallet
                </Button>
              </form>
            </Card>

            <div className="border-t border-slate-700/50 pt-6 space-y-3 mt-6">
              <p className="text-xs text-slate-500">
                No existing wallet? Create or import one below.
              </p>
              <Button
                variant="secondary"
                onClick={() => {
                  setView("create");
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
                  setView("import");
                  setError(null);
                }}
                className="w-full"
                disabled={loading}
              >
                Import existing key
              </Button>
            </div>
          </>
        )}

        {view === "create" && (
          <Card>
            <CardHeader title="Create new wallet" />
            <p className="text-sm text-slate-400 mb-4">
              A new Ed25519 key will be generated and encrypted to your keystore.
            </p>
            <form onSubmit={handleCreate} className="space-y-4">
              <Input
                label="Passphrase to encrypt keystore"
                type="password"
                autoComplete="new-password"
                placeholder="Choose a passphrase"
                value={newPassphrase}
                onChange={(e) => setNewPassphrase(e.target.value)}
                disabled={loading}
              />
              <Button
                type="submit"
                variant="primary"
                disabled={loading || !newPassphrase}
                className="w-full"
              >
                Create wallet
              </Button>
            </form>
            <Button
              variant="secondary"
              onClick={() => {
                setView("unlock");
                setError(null);
              }}
              className="w-full mt-4"
              disabled={loading}
            >
              Back to unlock
            </Button>
          </Card>
        )}

        {view === "import" && (
          <Card>
            <CardHeader title="Import wallet" />
            <p className="text-sm text-slate-400 mb-4">
              Paste an existing Ed25519 secret key (hex) to import it.
            </p>
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
                autoComplete="new-password"
                placeholder="Choose a passphrase"
                value={newPassphrase}
                onChange={(e) => setNewPassphrase(e.target.value)}
                disabled={loading}
              />
              <Button
                type="submit"
                variant="primary"
                disabled={loading || !secretKey || !newPassphrase}
                className="w-full"
              >
                Import wallet
              </Button>
            </form>
            <Button
              variant="secondary"
              onClick={() => {
                setView("unlock");
                setError(null);
              }}
              className="w-full mt-4"
              disabled={loading}
            >
              Back to unlock
            </Button>
          </Card>
        )}
      </div>
    </div>
  );
}
