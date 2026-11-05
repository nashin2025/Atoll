import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery } from '@apollo/client';
import { gql } from '@apollo/client';

const BLOCK_BY_HASH = gql`
  query Block($hash: String!) {
    block(hash: $hash) {
      hash
      height
      timestamp
      proposer { address }
      difficulty
      transaction_count
    }
  }
`;

const TRANSACTION_BY_HASH = gql`
  query Transaction($hash: String!) {
    transaction(hash: $hash) {
      hash
      from { address }
      to { address }
      value
      block_height
    }
  }
`;

const ADDRESS_QUERY = gql`
  query Address($address: String!) {
    address(address: $address) {
      address
      balance
      nonce
    }
  }
`;

export function SearchBar() {
  const [query, setQuery] = useState('');
  const [type, setType] = useState<'block' | 'address' | 'transaction'>('block');
  const navigate = useNavigate();

  const { data: blockData, error: blockError } = useQuery(BLOCK_BY_HASH, {
    variables: { hash: query },
    skip: type !== 'block' || !query,
  });

  const { data: txData, error: txError } = useQuery(TRANSACTION_BY_HASH, {
    variables: { hash: query },
    skip: type !== 'transaction' || !query,
  });

  const { data: addrData, error: addrError } = useQuery(ADDRESS_QUERY, {
    variables: { address: query },
    skip: type !== 'address' || !query,
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!query.trim()) return;

    if (type === 'block' && blockData?.block) {
      navigate(`/blocks/${blockData.block.hash}`);
    } else if (type === 'transaction' && txData?.transaction) {
      navigate(`/transactions/${txData.transaction.hash}`);
    } else if (type === 'address' && addrData?.address) {
      navigate(`/validators/${addrData.address.address}`);
    }
  };

  const hasError = blockError || txError || addrError;

  return (
    <div className="max-w-2xl mx-auto px-4 sm:px-6 lg:px-8 mb-8">
      <div className="bg-slate-900 border border-slate-800 rounded-xl p-6">
        <h2 className="text-lg font-semibold mb-4">Global Search</h2>
        <form onSubmit={handleSubmit} className="flex flex-col sm:flex-row gap-3">
          <div className="flex-1 min-w-0">
            <input
              type="text"
              className="input"
              placeholder="Search by block hash, transaction hash, or address"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <select
            className="input"
            value={type}
            onChange={(e) => setType(e.target.value as 'block' | 'address' | 'transaction')}
          >
            <option value="block">Block hash</option>
            <option value="transaction">Transaction hash</option>
            <option value="address">Address</option>
          </select>
          <button type="submit" className="btn-primary">
            Search
          </button>
        </form>
        {hasError && query && (
          <div className="mt-3 p-3 bg-red-500/10 border border-red-500/20 rounded text-sm text-red-400">
            No results found for &ldquo;{query}&rdquo;
          </div>
        )}
      </div>
    </div>
  );
}
