import { useState, useEffect } from 'react';
import {
  Button,
  ErrorDisplay,
  Input,
  AddressDisplay,
  Loading,
  Card,
  CardHeader,
  Badge,
  SuccessDisplay,
} from '@/components/ui';
import { walletStake, walletUnstake, walletStakeInfo } from '@/tauri';

interface StakingProps {
  address: string;
  passphrase: string;
  onBack: () => void;
}

export function Staking({ address, passphrase, onBack }: StakingProps) {
  const [stakeAmount, setStakeAmount] = useState('');
  const [unstakeAmount, setUnstakeAmount] = useState('');
  const [stakedAmount, setStakedAmount] = useState<string | null>(null);
  const [rewards, setRewards] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const fetchStakeInfo = async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await walletStakeInfo({ address });
      setStakedAmount(response.staked_amount);
      setRewards(response.rewards);
    } catch (err) {
      setError(formatInvokeError(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchStakeInfo();
  }, [address, passphrase]);

  const handleStake = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setSuccess(null);
    try {
      await walletStake({
        from: address,
        amount: stakeAmount,
        passphrase,
      });
      setSuccess('Staked successfully');
      setStakeAmount('');
      await fetchStakeInfo();
    } catch (err) {
      setError(formatInvokeError(err));
    } finally {
      setLoading(false);
    }
  };

  const handleUnstake = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setSuccess(null);
    try {
      await walletUnstake({
        from: address,
        amount: unstakeAmount,
        passphrase,
      });
      setSuccess('Unstaked successfully');
      setUnstakeAmount('');
      await fetchStakeInfo();
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
          <h1 className="text-2xl font-bold text-white tracking-tight">Staking</h1>
        </header>

        {error && <ErrorDisplay message={error} />}
        {success && <SuccessDisplay message={success} />}

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
          <Card>
            <CardHeader title="Staked ATOLL" />
            <div className="flex items-baseline gap-2">
              {loading ? (
                <Loading className="h-6 w-6" />
              ) : (
                <span className="text-2xl font-bold text-white font-mono">
                  {stakedAmount === null ? '–' : stakedAmount}
                </span>
              )}
              <span className="text-slate-500 text-sm">ATOLL</span>
            </div>
          </Card>

          <Card>
            <CardHeader title="Pending rewards" />
            <div className="flex items-baseline gap-2">
              {loading ? (
                <Loading className="h-6 w-6" />
              ) : (
                <span className="text-2xl font-bold text-emerald-400 font-mono">
                  {rewards === null ? '–' : rewards}
                </span>
              )}
              <span className="text-slate-500 text-sm">ATOLL</span>
            </div>
          </Card>

          <Card>
            <CardHeader title="Estimated APR" />
            <div className="flex items-baseline gap-2">
              <span className="text-2xl font-bold text-white font-mono">~8.5%</span>
              <span className="text-slate-500 text-sm">APY</span>
            </div>
          </Card>
        </div>

        <Card>
          <CardHeader title="Stake ATOLL" />
          <form onSubmit={handleStake} className="space-y-4">
            <Input
              label="Amount (ATOLL)"
              type="text"
              placeholder="Enter amount to stake"
              value={stakeAmount}
              onChange={(e) => setStakeAmount(e.target.value)}
            />
            <Button type="submit" variant="primary" disabled={loading || !stakeAmount}>
              {loading ? 'Staking…' : 'Stake ATOLL'}
            </Button>
            <p className="text-xs text-slate-500 mt-2">
              Staked ATOLL earns rewards and contributes to network security. Funds are locked during the staking period.
            </p>
          </form>
        </Card>

        <Card className="mt-6">
          <CardHeader title="Unstake ATOLL" />
          <form onSubmit={handleUnstake} className="space-y-4">
            <Input
              label="Amount (ATOLL)"
              type="text"
              placeholder="Enter amount to unstake"
              value={unstakeAmount}
              onChange={(e) => setUnstakeAmount(e.target.value)}
            />
            <Button type="submit" variant="secondary" disabled={loading || !unstakeAmount}>
              {loading ? 'Unstaking…' : 'Unstake ATOLL'}
            </Button>
            <p className="text-xs text-slate-500 mt-2">
              Unstaked ATOLL will be available after the unbonding period. Rewards stop accruing once unstaking begins.
            </p>
          </form>
        </Card>

        <Card className="mt-6">
          <CardHeader title="About staking" />
          <ul className="space-y-2 text-sm text-slate-400">
            <li className="flex items-start gap-2">
              <span className="text-emerald-400">✓</span>
              Earn rewards for securing the network
            </li>
            <li className="flex items-start gap-2">
              <span className="text-emerald-400">✓</span>
              Participate in network consensus
            </li>
            <li className="flex items-start gap-2">
              <span className="text-emerald-400">✓</span>
              Funds are locked during staking period
            </li>
            <li className="flex items-start gap-2">
              <span className="text-emerald-400">✓</span>
              Unstaking has a cooldown period
            </li>
          </ul>
        </Card>
      </div>
    </div>
  );
}
