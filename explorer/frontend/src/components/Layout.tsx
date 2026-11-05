import { Link } from 'react-router-dom';

export function Layout({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen bg-slate-950 text-slate-100">
      <header className="border-b border-slate-800 bg-slate-900/80 backdrop-blur sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <Link to="/" className="flex items-center gap-2">
              <div className="w-8 h-8 rounded bg-indigo-600 flex items-center justify-center text-white font-bold text-xs">
                A
              </div>
              <span className="font-semibold text-lg tracking-tight">Atoll Explorer</span>
            </Link>
            <nav className="flex items-center gap-6">
              <Link
                to="/"
                className="text-sm font-medium text-slate-300 hover:text-indigo-400 transition-colors"
              >
                Home
              </Link>
              <Link
                to="/blocks"
                className="text-sm font-medium text-slate-300 hover:text-indigo-400 transition-colors"
              >
                Blocks
              </Link>
              <Link
                to="/transactions"
                className="text-sm font-medium text-slate-300 hover:text-indigo-400 transition-colors"
              >
                Transactions
              </Link>
              <Link
                to="/validators"
                className="text-sm font-medium text-slate-300 hover:text-indigo-400 transition-colors"
              >
                Validators
              </Link>
            </nav>
          </div>
        </div>
      </header>
      <main>{children}</main>
    </div>
  );
}
