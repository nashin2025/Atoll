import { ChangeEvent } from 'react';

interface SearchBarProps {
  onSearch: (query: string) => void;
  placeholder?: string;
}

export function SearchBar({ onSearch, placeholder = 'Search by hash, address, or height' }: SearchBarProps) {
  const handleSubmit = (e: ChangeEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const query = formData.get('query') as string;
    onSearch(query);
  };

  return (
    <form onSubmit={handleSubmit} className="relative max-w-md">
      <input
        type="text"
        name="query"
        placeholder={placeholder}
        className="input pl-10 pr-3"
      />
      <button type="submit" className="absolute left-2 top-1/2 -translate-y-1/2 text-xs text-slate-400">
        Search
      </button>
    </form>
  );
}
