import { useQuery } from '@apollo/client';
import { useParams } from 'react-router-dom';
import { TRANSACTION_BY_HASH_QUERY } from '@/graphql/queries';
import { Loading } from '@/components/Loading';
import { ErrorDisplay } from '@/components/ErrorDisplay';
import { AddressDisplay } from '@/components/AddressDisplay';

export function TransactionDetail() {
  const { hash } = useParams<{ hash: string }>();
  const { loading, error, data } = useQuery(TRANSACTION_BY_HASH_QUERY, {
    variables: { hash: hash || '' },
  });

  if (loading) return <Loading />;
  if (error) return <ErrorDisplay message={error.message} />;
  if (!data?.transaction) return <ErrorDisplay message="Transaction not found" />;

  const tx = data.transaction;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-white">Transaction</h1>
      </div>

      <div className="card">
        <div className="text-slate-400 text-xs font-semibold uppercase mb-4">Transaction Details</div>
        <div className="space-y-3">
          <div className="flex justify-between border-b border-slate-800 pb-2">
            <span className="text-slate-400 text-sm">Hash</span>
            <span className="font-mono text-atoll-400 truncate max-w-xs">{tx.hash}</span>
          </div>
          <div className="flex justify-between border-b border-slate-800 pb-2">
            <span className="text-slate-400 text-sm">Version</span>
            <span className="font-mono text-slate-300">{tx.version}</span>
          </div>
          <div className="flex justify-between border-b border-slate-800 pb-2">
            <span className="text-slate-400 text-sm">From</span>
            <span>
              <AddressDisplay address={tx.from.address} />
            </span>
          </div>
          <div className="flex justify-between border-b border-slate-800 pb-2">
            <span className="text-slate-400 text-sm">To</span>
            <span>
              <AddressDisplay address={tx.to?.address || '–'} />
            </span>
          </div>
          <div className="flex justify-between border-b border-slate-800 pb-2">
            <span className="text-slate-400 text-sm">Value</span>
            <span className="font-mono text-slate-300">{tx.value}</span>
          </div>
          <div className="flex justify-between border-b border-slate-800 pb-2">
            <span className="text-slate-400 text-sm">Block Height</span>
            <span className="font-mono text-slate-300">{tx.blockHeight}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400 text-sm">Data</span>
            <span className="font-mono text-slate-300 truncate max-w-xs">{tx.data || '–'}</span>
          </div>
        </div>
      </div>
    </div>
  );
}
