import { createBrowserRouter, RouterProvider, RouteObject } from 'react-router-dom';
import { Home } from '@/pages/Home';
import { BlocksList } from '@/pages/BlocksList';
import { BlockDetail } from '@/pages/BlockDetail';
import { TransactionsList } from '@/pages/TransactionsList';
import { TransactionDetail } from '@/pages/TransactionDetail';
import { AddressPage } from '@/pages/AddressPage';
import { ValidatorsList } from '@/pages/ValidatorsList';

const routes: RouteObject[] = [
  { path: '/', element: <Home /> },
  { path: '/blocks', element: <BlocksList /> },
  { path: '/blocks/:hash', element: <BlockDetail /> },
  { path: '/transactions', element: <TransactionsList /> },
  { path: '/transactions/:hash', element: <TransactionDetail /> },
  { path: '/address/:address', element: <AddressPage /> },
  { path: '/validators', element: <ValidatorsList /> },
];

const router = createBrowserRouter(routes);

export default function App() {
  return <RouterProvider router={router} />;
}
