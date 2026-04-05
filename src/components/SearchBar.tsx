import { useState, type FormEvent } from "react";
import { Search, Loader2 } from "lucide-react";

interface Props {
  onSearch: (gameName: string, tagLine: string) => void;
  loading: boolean;
}

export function SearchBar({ onSearch, loading }: Props) {
  const [input, setInput] = useState("");

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    const parts = input.split("#");
    if (parts.length !== 2 || !parts[0].trim() || !parts[1].trim()) return;
    onSearch(parts[0].trim(), parts[1].trim());
  };

  const isValid = input.includes("#") && input.split("#").length === 2;

  return (
    <form onSubmit={handleSubmit} className="w-full max-w-lg mx-auto">
      <div className="relative">
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Riot ID (Name#Tag)"
          disabled={loading}
          className="w-full px-5 py-3.5 pr-14 rounded-xl bg-bg-card border border-border
                     text-text-primary placeholder:text-text-muted
                     focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent
                     transition-all text-base disabled:opacity-50"
        />
        <button
          type="submit"
          disabled={loading || !isValid}
          className="absolute right-2 top-1/2 -translate-y-1/2 p-2.5 rounded-lg
                     bg-accent hover:bg-accent-hover disabled:opacity-30
                     transition-colors text-white"
        >
          {loading ? (
            <Loader2 size={18} className="animate-spin" />
          ) : (
            <Search size={18} />
          )}
        </button>
      </div>
      {input && !isValid && (
        <p className="text-sm text-text-muted mt-2 text-center">
          Формат: Name#RU1
        </p>
      )}
    </form>
  );
}
