import { useQuery } from '@apollo/client';
import { Table, TableHead, TableBody, TableRow, TableHeaderCell, TableCell } from '@/components/Table';
import { Loading } from '@/components/Loading';
import { ErrorDisplay } from '@/components/ErrorDisplay';
import { AddressDisplay } from '@/components/AddressDisplay';
import { VALIDATORS_LIST_QUERY } from '@/graphql/queries';

interface ValidatorRow {
  address: { address: string };
  blocksProposed: number;
  lastBlockHeight: string;
}

export function ValidatorsList() {
  const { loading, error, data } = useQuery(VALIDATORS_LIST_QUERY, {
    variables: { first: '50' },
  });

  if (loading) return <Loading />;
  if (error) return <ErrorDisplay message={error.message} onRetry={() => window.location.reload()} />;

  const validators: ValidatorRow[] = data?.validators || [];

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-white">Validators</h1>
      </div>

      <Table>
        <TableHead>
          <TableRow>
            <TableHeaderCell>Address</TableHeaderCell>
            <TableHeaderCell>Blocks Proposed</TableHeaderCell>
            <TableHeaderCell>Last Block Height</TableHeaderCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {validators.map((v) => (
            <TableRow key={v.address.address}>
              <TableCell>
                <AddressDisplay address={v.address.address} />
              </TableCell>
              <TableCell className="font-mono text-slate-300">{v.blocksProposed}</TableCell>
              <TableCell className="font-mono text-slate-300">{v.lastBlockHeight}</TableCell>
            </TableRow>
          ))}
          {validators.length === 0 && (
            <TableRow>
              <TableCell colSpan={3} className="text-slate-500 text-center">
                No validators found
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </div>
  );
}
