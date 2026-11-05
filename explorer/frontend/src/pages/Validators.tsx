import { useQuery } from '@apollo/client';
import { gql } from '@apollo/client';
import { useParams } from 'react-router-dom';
import { Validator } from '../types';

const VALIDATORS_LIST = gql`
  query Validators($first: String) {
    validators(first: $first) {
      address { address }
      blocksProposed
      lastBlockHeight
    }
  }
`;

export function ValidatorsList() {
  const { data, loading, error } = useQuery<{ validators: Validator[] }>(
    VALIDATORS_LIST,
    { variables: { first: '50' } },
  );

  if (loading) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-8">
        <div className="flex justify-center py-20">
          <div className="h-4 w-4 rounded-full border-2 border-indigo-500 border-t-transparent animate-spin"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-8">
        <div className="card p-8 text-center">
          <p className="text-red-400">Failed to load validators</p>
        </div>
      </div>
    );
  }

  const validators = data?.validators ?? [];

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="mb-6">
        <h1 className="text-3xl font-bold text-slate-100">Validators</h1>
      </div>

      <div className="card overflow-hidden">
        <table className="w-full text-left border-collapse">
          <thead>
            <tr className="border-b border-slate-800">
              <th className="table-header">Address</th>
              <th className="table-header">Blocks Proposed</th>
              <th className="table-header">Last Block Height</th>
            </tr>
          </thead>
          <tbody>
            {validators.map((v) => (
              <tr key={v.address.address} className="hover:bg-slate-800/50 transition-colors">
                <td className="table-cell mono">{v.address.address}</td>
                <td className="table-cell">{v.blocks_proposed}</td>
                <td className="table-cell mono">{v.last_block_height}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export function ValidatorDetail() {
  const { address } = useParams<{ address: string }>();
  const { data, loading, error } = useQuery<{ validators: Validator[] }>(
    VALIDATORS_LIST,
    { variables: { first: '500' } },
  );

  if (loading) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-8">
        <div className="flex justify-center py-20">
          <div className="h-4 w-4 rounded-full border-2 border-indigo-500 border-t-transparent animate-spin"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-8">
        <div className="card p-8 text-center">
          <p className="text-red-400">Failed to load validator details</p>
        </div>
      </div>
    );
  }

  const validator = data?.validators.find((v) => v.address.address === address);

  if (!validator) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="card p-8 text-center">
          <h1 className="text-3xl font-bold text-slate-100">Validator Detail</h1>
          <p className="text-slate-400">
            Validator not found: {address ?? 'unknown'}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="mb-6">
        <h1 className="text-3xl font-bold text-slate-100">Validator Detail</h1>
      </div>

      <div className="card p-8">
        <dl className="grid grid-cols-2 gap-4">
          <div>
            <dt className="text-slate-400 text-sm">Address</dt>
            <dd className="mono">{validator.address.address}</dd>
          </div>
          <div>
            <dt className="text-slate-400 text-sm">Blocks Proposed</dt>
            <dd className="text-slate-100">{validator.blocks_proposed}</dd>
          </div>
          <div>
            <dt className="text-slate-400 text-sm">Last Block Height</dt>
            <dd className="mono text-slate-100">{validator.last_block_height}</dd>
          </div>
        </dl>
      </div>
    </div>
  );
}
