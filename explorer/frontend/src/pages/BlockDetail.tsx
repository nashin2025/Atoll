import { useQuery } from '@apollo/client';
import { useParams } from 'react-router-dom';
import { BLOCK_DETAIL_QUERY } from '@/graphql/queries';
import { Loading } from '@/components/Loading';
import { ErrorDisplay } from '@/components/ErrorDisplay';
import { AddressDisplay } from '@/components/AddressDisplay';
import { Table, TableHead, TableBody, TableRow, TableHeaderCell, TableCell } from '@/components/Table';

export function BlockDetail() {
  const { hash } = useParams<{ hash: string }>();
  const { loading, error, data } = useQuery(BLOCK_DETAIL_QUERY, {
    variables: { hash: hash || '' },
  });

  if (loading) return <Loading />;
  if (error) return <ErrorDisplay message={error.message} />;
  if (!data?.block) return <ErrorDisplay message="Block not found" />;

  const block = data.block;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-white">Block #{block.height}</h1>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div className="card">
          <div className="text-slate-400 text-xs font-semibold uppercase mb-4">Block Information</div>
          <div className="space-y-3">
            <div className="flex justify-between border-b border-slate-800 pb-2">
              <span className="text-slate-400 text-sm">Hash</span>
              <span className="font-mono text-atoll-400 truncate max-w-xs">{block.hash}</span>
            </div>
            <div className="flex justify-between border-b border-slate-800 pb-2">
              <span className="text-slate-400 text-sm">Height</span>
              <span className="font-mono text-slate-300">{block.height}</span>
            </div>
            <div className="flex justify-between border-b border-slate-800 pb-2">
              <span className="text-slate-400 text-sm">Timestamp</span>
              <span className="font-mono text-slate-300">
                {new Date(Number(block.timestamp)).toLocaleString()}
              </span>
            </div>
            <div className="flex justify-between border-b border-slate-800 pb-2">
              <span className="text-slate-400 text-sm">Shard ID</span>
              <span className="font-mono text-slate-300">{block.shardId}</span>
            </div>
            <div className="flex justify-between border-b border-slate-800 pb-2">
              <span className="text-slate-400 text-sm">Version</span>
              <span className="font-mono text-slate-300">{block.version}</span>
            </div>
            <div className="flex justify-between border-b border-slate-800 pb-2">
              <span className="text-slate-400 text-sm">Difficulty</span>
              <span className="font-mono text-slate-300">{block.difficulty}</span>
            </div>
            <div className="flex justify-between border-b border-slate-800 pb-2">
              <span className="text-slate-400 text-sm">Proposer</span>
              <span>
                <AddressDisplay address={block.proposer.address} />
              </span>
            </div>
            <div className="flex justify-between border-b border-slate-800 pb-2">
              <span className="text-slate-400 text-sm">Transaction Count</span>
              <span className="font-mono text-slate-300">{block.transactionCount}</span>
            </div>
            <div className="flex justify-between border-b border-slate-800 pb-2">
              <span className="text-slate-400 text-sm">State Root</span>
              <span className="font-mono text-slate-300 truncate max-w-xs">{block.stateRoot}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-slate-400 text-sm">Transaction Root</span>
              <span className="font-mono text-slate-300 truncate max-w-xs">{block.transactionRoot}</span>
            </div>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <span className="text-slate-400 text-xs font-semibold uppercase">
              Transactions ({block.transactionCount})
            </span>
          </div>
          <Table>
            <TableHead>
              <TableRow>
                <TableHeaderCell>Hash</TableHeaderCell>
                <TableHeaderCell>From</TableHeaderCell>
                <TableHeaderCell>To</TableHeaderCell>
                <TableHeaderCell>Value</TableHeaderCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {block.transactions.map((tx: { hash: string; from: { address: string }; to: { address: string } | null; value: string }) => (
                <TableRow key={tx.hash}>
                  <TableCell className="font-mono text-atoll-400 text-xs truncate">
                    {tx.hash}
                  </TableCell>
                  <TableCell>
                    <AddressDisplay address={tx.from.address} truncate />
                  </TableCell>
                  <TableCell>
                    <AddressDisplay address={tx.to?.address || '–'} truncate />
                  </TableCell>
                  <TableCell className="font-mono text-xs text-slate-300">{tx.value}</TableCell>
                </TableRow>
              ))}
              {block.transactionCount === 0 && (
                <TableRow>
                  <TableCell colSpan={4} className="text-slate-500 text-center">
                    No transactions
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </div>
      </div>
    </div>
  );
}
