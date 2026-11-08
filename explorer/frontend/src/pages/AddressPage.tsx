import { useQuery } from '@apollo/client';
import { useParams } from 'react-router-dom';
import { ADDRESS_QUERY } from '@/graphql/queries';
import { Loading } from '@/components/Loading';
import { ErrorDisplay } from '@/components/ErrorDisplay';

export function AddressPage() {
  const { address } = useParams<{ address: string }>();
  const { loading, error, data } = useQuery(ADDRESS_QUERY, {
    variables: { address: address || '' },
  });

  if (loading) return <Loading />;
  if (error) return <ErrorDisplay message={error.message} />;
  if (!data?.address) return <ErrorDisplay message="Address not found" />;

  const acct = data.address;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-white">Address</h1>
      </div>

      <div className="card">
        <div className="text-slate-400 text-xs font-semibold uppercase mb-4">Account Information</div>
        <div className="space-y-3">
          <div className="flex justify-between border-b border-slate-800 pb-2">
            <span className="text-slate-400 text-sm">Address</span>
            <span className="font-mono text-atoll-400 break-all">{acct.address}</span>
          </div>
          <div className="flex justify-between border-b border-slate-800 pb-2">
            <span className="text-slate-400 text-sm">Balance (ATOLL)</span>
            <span className="font-mono text-slate-300">{acct.balance}</span>
          </div>
          <div className="flex justify-between border-b border-slate-800 pb-2">
            <span className="text-slate-400 text-sm">Nonce</span>
            <span className="font-mono text-slate-300">{acct.nonce}</span>
          </div>
          <div className="flex justify-between border-b border-slate-800 pb-2">
            <span className="text-slate-400 text-sm">Transaction Count</span>
            <span className="font-mono text-slate-300">{acct.transactionCount}</span>
          </div>
          <div className="flex justify-between border-b border-slate-800 pb-2">
            <span className="text-slate-400 text-sm">Code Hash</span>
            <span className="font-mono text-slate-300 truncate max-w-xs">
              {acct.codeHash || '–'}
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400 text-sm">Storage Root</span>
            <span className="font-mono text-slate-300 truncate max-w-xs">
              {acct.storageRoot || '–'}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
