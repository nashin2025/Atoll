import { HTMLAttributes, ReactNode } from 'react';

interface TableProps extends HTMLAttributes<HTMLTableElement> {
  children: ReactNode;
}

export function Table({ children, className }: TableProps) {
  return (
    <div className="overflow-x-auto">
      <table className={`w-full text-left border-collapse ${className || ''}`}>{children}</table>
    </div>
  );
}

export function TableHead({ children }: { children: ReactNode }) {
  return <thead>{children}</thead>;
}

export function TableBody({ children }: { children: ReactNode }) {
  return <tbody>{children}</tbody>;
}

export function TableRow({
  children,
  className,
  onClick,
}: {
  children: ReactNode;
  className?: string;
  onClick?: () => void;
}) {
  return (
    <tr className={`border-b border-slate-800 ${className || ''}`} onClick={onClick}>
      {children}
    </tr>
  );
}

export function TableHeaderCell({
  children,
  className,
  colSpan,
}: {
  children: ReactNode;
  className?: string;
  colSpan?: number;
}) {
  return (
    <th className={`px-3 py-2 text-left text-xs font-semibold text-slate-400 uppercase border-b border-slate-700 ${className || ''}`} colSpan={colSpan}>
      {children}
    </th>
  );
}

export function TableCell({
  children,
  className,
  colSpan,
}: {
  children: ReactNode;
  className?: string;
  colSpan?: number;
}) {
  return (
    <td className={`px-3 py-2 text-sm text-slate-100 border-b border-slate-800 ${className || ''}`} colSpan={colSpan}>
      {children}
    </td>
  );
}
