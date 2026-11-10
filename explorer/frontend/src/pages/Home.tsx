import { useQuery } from '@apollo/client';
import { gql } from '@apollo/client';
import { NetworkStats } from '../types';
import { StatsCard } from '../components/StatsCard';
import { SearchBar } from '../components/SearchBar';

const NETWORK_STATS = gql`
  query NetworkStats {
    networkStats {
      blockHeight
      finalizedHeight
      blockCount
      totalSupply
      mindSupply
      peerCount
      syncing
    }
  }
`;

export function Home() {
  const { data, loading, error } = useQuery<{ networkStats: NetworkStats }>(
    NETWORK_STATS,
  );

  if (loading) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="flex justify-center py-20">
          <div className="h-4 w-4 rounded-full border-2 border-indigo-500 border-t-transparent animate-spin"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="card p-8 text-center">
          <p className="text-red-400">Failed to load network stats</p>
        </div>
      </div>
    );
  }

  const stats = data?.networkStats;

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-slate-100 mb-2">Network Overview</h1>
        <p className="text-slate-400">Real-time statistics from the Atoll Protocol network</p>
      </div>

      <SearchBar />

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6 mt-8">
        <StatsCard title="Block Height" value={stats?.block_height ?? '--'} />
        <StatsCard title="Finalized Height" value={stats?.finalized_height ?? '--'} />
        <StatsCard title="Block Count" value={stats?.block_count?.toLocaleString() ?? '--'} />
        <StatsCard title="Peer Count" value={stats?.peer_count?.toLocaleString() ?? '--'} />
        <StatsCard title="Total Supply (ATOLL)" value={stats?.total_supply ?? '--'} />
        <StatsCard title="Total Supply (MIND)" value={stats?.mind_supply ?? '--'} />
      </div>

      <div className="mt-8 card">
        <div className="flex items-center justify-between p-4 border-b border-slate-800">
          <h2 className="text-lg font-semibold">Network Status</h2>
          <span className={`px-2 py-1 rounded-full text-xs font-medium ${
            stats?.syncing ? 'bg-amber-500/10 text-amber-400 border border-amber-500/20' : 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20'
          }`}>
            {stats?.syncing ? 'Syncing' : 'Synced'}
          </span>
        </div>
      </div>
    </div>
  );
}
