import { useQuery } from '@apollo/client';
import { useNavigate } from 'react-router-dom';
import { BLOCKS_LIST_QUERY } from '@/graphql/queries';
import { SearchBar } from '@/components/SearchBar';
import { Table, TableHead, TableBody, TableRow, TableHeaderCell, TableCell } from '@/components/Table';
import { Loading } from '@/components/Loading';
import { ErrorDisplay } from '@/components/ErrorDisplay';
import { AddressDisplay } from '@/components/AddressDisplay';

interface BlockRow {
  hash: string;
  height: string;
  timestamp: string;
  shardId: string;
  proposer: { address: string };
  transactionCount: number;
}

export function BlocksList() {
  const navigate = useNavigate();
  const { loading, error, data } = useQuery(BLOCKS_LIST_QUERY, {
    variables: { first: '20', offset: '0' },
  });

  if (loading) return <Loading />;
  if (error) return <ErrorDisplay message={error.message} onRetry={() => window.location.reload()} />;

  const blocks: BlockRow[] = data?.blocks || [];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-white">Blocks</h1>
        <SearchBar
          onSearch={(query) => {
            navigate(`/blocks/${query}`);
          }}
        />
      </div>

      <Table>
        <TableHead>
          <TableRow>
            <TableHeaderCell>Height</TableHeaderCell>
            <TableHeaderCell>Hash</TableHeaderCell>
            <TableHeaderCell>Timestamp</TableHeaderCell>
            <TableHeaderCell>Shard</TableHeaderCell>
            <TableHeaderCell>Proposer</TableHeaderCell>
            <TableHeaderCell>Txs</TableHeaderCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {blocks.map((block) => (
            <TableRow
              key={block.hash}
              onClick={() => navigate(`/blocks/${block.hash}`)}
              className="cursor-pointer hover:bg-slate-800/50"
            >
              <TableCell className="font-mono text-slate-300">{block.height}</TableCell>
              <TableCell className="font-mono text-atoll-400 truncate max-w-xs">{block.hash}</TableCell>
              <TableCell className="font-mono text-slate-300">
                {new Date(Number(block.timestamp)).toLocaleString()}
              </TableCell>
              <TableCell className="font-mono text-slate-300">{block.shardId}</TableCell>
              <TableCell>
                <AddressDisplay address={block.proposer.address} />
              </TableCell>
              <TableCell className="font-mono text-slate-300">{block.transactionCount}</TableCell>
            </TableRow>
          ))}
          {blocks.length === 0 && (
            <TableRow>
              <TableCell colSpan={6} className="text-slate-500 text-center">
                No blocks found
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </div>
  );
}
