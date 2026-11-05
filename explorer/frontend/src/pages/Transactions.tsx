import React from 'react';
import { useQuery } from '@apollo/client';
import { gql } from '@apollo/client';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { Transaction } from '../types';

const TRANSACTIONS_LIST = gql`
  query Transactions($first: String, $offset: String) {
    transactions(first: $first, offset: $offset) {
      hash
      from { address }
      to { address }
      value
      block_height
      version
    }
  }
`;

const TRANSACTION_BY_HASH = gql`
  query TransactionByHash($hash: String!) {
    transaction(hash: $hash) {
      hash
      version
      from { address }
      to { address }
      value
      data
      block_height
    }
  }
`;

export function TransactionsList() {
  const navigate = useNavigate();
  const [offset, setOffset] = React.useState(0);

  const { data, loading, error } = useQuery<{ transactions: Transaction[] }>(
    TRANSACTIONS_LIST,
    {
      variables: { first: '20', offset: offset.toString() },
    },
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
          <p className="text-red-400">Failed to load transactions</p>
        </div>
      </div>
    );
  }

  const transactions = data?.transactions ?? [];

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="mb-6">
        <h1 className="text-3xl font-bold text-slate-100">Transactions</h1>
      </div>

      <div className="card overflow-hidden">
        <table className="w-full text-left border-collapse">
          <thead>
            <tr className="border-b border-slate-800">
              <th className="table-header">Hash</th>
              <th className="table-header">From</th>
              <th className="table-header">To</th>
              <th className="table-header">Value</th>
              <th className="table-header">Block</th>
            </tr>
          </thead>
          <tbody>
            {transactions.map((tx) => (
              <tr
                key={tx.hash}
                onClick={() => navigate(`/transactions/${tx.hash}`)}
                className="hover:bg-slate-800/50 cursor-pointer transition-colors"
              >
                <td className="table-cell mono">{tx.hash}</td>
                <td className="table-cell mono">{tx.from.address}</td>
                <td className="table-cell mono">{tx.to?.address ?? '—'}</td>
                <td className="table-cell mono">{tx.value}</td>
                <td className="table-cell mono">{tx.block_height}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="mt-6 flex items-center justify-between">
        <button
          onClick={() => setOffset(Math.max(0, offset - 20))}
          disabled={offset === 0}
          className="btn-secondary disabled:opacity-50 disabled:pointer-events-none"
        >
          Previous
        </button>
        <button
          onClick={() => setOffset(offset + 20)}
          className="btn-secondary"
        >
          Next
        </button>
      </div>
    </div>
  );
}

export function TransactionDetail() {
  const { hash } = useParams<{ hash: string }>();
  const navigate = useNavigate();

  const { data, loading, error } = useQuery<{ transaction: Transaction | null }>(
    TRANSACTION_BY_HASH,
    {
      variables: { hash: hash ?? '' },
      skip: !hash,
    },
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

  if (error || !data?.transaction) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-8">
        <div className="card p-8 text-center">
          <h2 className="text-xl font-semibold text-red-400 mb-2">Transaction not found</h2>
          <button
            onClick={() => navigate('/transactions')}
            className="btn-primary mt-4"
          >
            Back to Transactions
          </button>
        </div>
      </div>
    );
  }

  const tx = data.transaction;

  return (
    <div className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="flex items-center justify-between mb-6">
        <Link to="/transactions" className="text-sm text-indigo-400 hover:text-indigo-300">
          &larr; Back to Transactions
        </Link>
        <h1 className="text-3xl font-bold text-slate-100">Transaction</h1>
      </div>

      <div className="card">
        <h2 className="text-lg font-semibold mb-4">Transaction Details</h2>
        <div className="space-y-3">
          <div className="flex justify-between">
            <span className="text-slate-400 text-sm">Hash</span>
            <span className="mono">{tx.hash}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400 text-sm">From</span>
            <span className="mono">{tx.from.address}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400 text-sm">To</span>
            <span className="mono">{tx.to?.address ?? 'Contract creation'}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400 text-sm">Value</span>
            <span className="mono">{tx.value}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400 text-sm">Block Height</span>
            <span className="mono">{tx.block_height}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400 text-sm">Version</span>
            <span className="mono">{tx.version}</span>
          </div>
          {tx.data && (
            <div className="flex justify-between">
              <span className="text-slate-400 text-sm">Data</span>
              <span className="mono">{tx.data}</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
