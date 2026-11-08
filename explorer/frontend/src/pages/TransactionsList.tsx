import { useQuery } from '@apollo/client';
import { useNavigate } from 'react-router-dom';
import { TRANSACTIONS_LIST_QUERY } from '@/graphql/queries';
import { SearchBar } from '@/components/SearchBar';
import { Table, TableHead, TableBody, TableRow, TableHeaderCell, TableCell } from '@/components/Table';
import { Loading } from '@/components/Loading';
import { ErrorDisplay } from '@/components/ErrorDisplay';
import { AddressDisplay } from '@/components/AddressDisplay';

interface TransactionRow {
  hash: string;
  version: string;
  from: { address: string };
  to: { address: string } | null;
  value: string;
  data: string | null;
  blockHeight: string;
}

export function TransactionsList() {
  const navigate = useNavigate();
  const { loading, error, data } = useQuery(TRANSACTIONS_LIST_QUERY, {
    variables: { first: '20', offset: '0' },
  });

  if (loading) return <Loading />;
  if (error) return <ErrorDisplay message={error.message} onRetry={() => window.location.reload()} />;

  const transactions: TransactionRow[] = data?.transactions || [];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-white">Transactions</h1>
        <SearchBar
          onSearch={(query) => {
            navigate(`/transactions/${query}`);
          }}
        />
      </div>

      <Table>
        <TableHead>
          <TableRow>
            <TableHeaderCell>Hash</TableHeaderCell>
            <TableHeaderCell>From</TableHeaderCell>
            <TableHeaderCell>To</TableHeaderCell>
            <TableHeaderCell>Value</TableHeaderCell>
            <TableHeaderCell>Block</TableHeaderCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {transactions.map((tx) => (
            <TableRow
              key={tx.hash}
              onClick={() => navigate(`/transactions/${tx.hash}`)}
              className="cursor-pointer hover:bg-slate-800/50"
            >
              <TableCell className="font-mono text-atoll-400 text-xs truncate max-w-xs">
                {tx.hash}
              </TableCell>
              <TableCell>
                <AddressDisplay address={tx.from.address} truncate />
              </TableCell>
              <TableCell>
                <AddressDisplay address={tx.to?.address || '–'} truncate />
              </TableCell>
              <TableCell className="font-mono text-xs text-slate-300">{tx.value}</TableCell>
              <TableCell className="font-mono text-xs text-slate-300">{tx.blockHeight}</TableCell>
            </TableRow>
          ))}
          {transactions.length === 0 && (
            <TableRow>
              <TableCell colSpan={5} className="text-slate-500 text-center">
                No transactions found
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </div>
  );
}
