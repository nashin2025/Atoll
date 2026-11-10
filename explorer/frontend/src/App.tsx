import { Routes, Route } from 'react-router-dom';
import { Layout } from './components/Layout';
import { Home } from './pages/Home';
import { BlocksList, BlockDetail } from './pages/Blocks';
import { TransactionsList, TransactionDetail } from './pages/Transactions';
import { ValidatorsList, ValidatorDetail } from './pages/Validators';
import { ApolloProvider } from '@apollo/client';
import { client } from './graphql/client';

function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Home />}>
      </Route>
      <Route path="/blocks" element={<BlocksList />} />
      <Route path="/blocks/:hash" element={<BlockDetail />} />
      <Route path="/transactions" element={<TransactionsList />} />
      <Route path="/transactions/:hash" element={<TransactionDetail />} />
      <Route path="/validators" element={<ValidatorsList />} />
      <Route path="/validators/:address" element={<ValidatorDetail />} />
    </Routes>
  );
}

export default function App() {
  return (
    <ApolloProvider client={client}>
      <Layout>
        <AppRoutes />
      </Layout>
    </ApolloProvider>
  );
}
