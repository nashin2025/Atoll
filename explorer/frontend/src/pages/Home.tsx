import { useQuery } from '@apollo/client';
import { useNavigate } from 'react-router-dom';
import { NETWORK_STATS_QUERY } from '@/graphql/queries';
import { SearchBar } from '@/components/SearchBar';
import { Table, TableHead, TableBody, TableRow, TableHeaderCell, TableCell } from '@/components/Table';
import { Loading } from '@/components/Loading';
import { ErrorDisplay } from '@/components/ErrorDisplay';

export function Home() {
  const { loading, error, data } = useQuery(NETWORK_STATS_QUERY);
  const navigate = useNavigate();

  if (loading) return <Loading />;
  if (error) return <ErrorDisplay message={error.message} />;

  const stats = data?.networkStats;
  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold text-white">Atoll Explorer</h1>
        <p className="text-slate-400 mt-1">Blockchain explorer for the Atoll Protocol</p>
      </div>

      <SearchBar
        onSearch={(query) => {
          if (query.startsWith('0x') && query.length === 66) {
            navigate(`/blocks/${query}`);
          } else if (query.startsWith('0x')) {
            navigate(`/address/${query}`);
          } else {
            navigate(`/blocks/${query}`);
          }
        }}
      />

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="card">
          <div className="text-slate-400 text-xs font-semibold uppercase mb-2">Block Height</div>
          <div className="text-2xl font-mono text-white">{stats?.blockHeight || '–'}</div>
        </div>
        <div className="card">
          <div className="text-slate-400 text-xs font-semibold uppercase mb-2">Finalized Height</div>
          <div className="text-2xl font-mono text-white">{stats?.finalizedHeight || '–'}</div>
        </div>
        <div className="card">
          <div className="text-slate-400 text-xs font-semibold uppercase mb-2">Peer Count</div>
          <div className="text-2xl font-mono text-white">{stats?.peerCount || '–'}</div>
        </div>
      </div>

      <div className="card">
        <Table>
          <TableHead>
            <TableRow>
              <TableHeaderCell>Recent Blocks</TableHeaderCell>
            </TableRow>
          </TableHead>
          <TableBody>
            <TableRow>
              <TableCell className="text-slate-500">No blocks yet</TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
