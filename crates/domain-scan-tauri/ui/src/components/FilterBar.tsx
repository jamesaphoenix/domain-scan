import { useState } from "react";
import type { EntityKind, Language } from "../types";

interface FilterBarProps {
  onSearch: (query: string) => void;
  onFilterKind: (kinds: EntityKind[] | undefined) => void;
  onFilterLanguage: (languages: Language[] | undefined) => void;
  availableLanguages: Language[];
  searchInputRef: React.RefObject<HTMLInputElement | null>;
  pathScope: { prefix: string; label?: string } | null;
  onClearPathScope: () => void;
}

const ENTITY_KINDS: { value: EntityKind; label: string }[] = [
  { value: "interface", label: "Interfaces" },
  { value: "service", label: "Services" },
  { value: "class", label: "Classes" },
  { value: "function", label: "Functions" },
  { value: "schema", label: "Schemas" },
  { value: "impl", label: "Impls" },
  { value: "type_alias", label: "Types" },
];

export function FilterBar({
  onSearch,
  onFilterKind,
  onFilterLanguage,
  availableLanguages,
  searchInputRef,
  pathScope,
  onClearPathScope,
}: FilterBarProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [activeKinds, setActiveKinds] = useState<Set<EntityKind>>(new Set());
  const [activeLanguage, setActiveLanguage] = useState<Language | null>(null);

  const handleSearch = (value: string) => {
    setSearchQuery(value);
    onSearch(value);
  };

  const toggleKind = (kind: EntityKind) => {
    setActiveKinds((prev) => {
      const next = new Set(prev);
      if (next.has(kind)) {
        next.delete(kind);
      } else {
        next.add(kind);
      }
      const kinds = next.size > 0 ? Array.from(next) : undefined;
      onFilterKind(kinds);
      return next;
    });
  };

  const handleLanguageChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const val = e.target.value;
    if (val === "") {
      setActiveLanguage(null);
      onFilterLanguage(undefined);
    } else {
      const lang = val as Language;
      setActiveLanguage(lang);
      onFilterLanguage([lang]);
    }
  };

  return (
    <div className="space-y-2 p-2 border-t border-gray-700">
      {pathScope && (
        <div className="rounded border border-blue-500/30 bg-blue-500/10 px-2 py-1.5">
          <div className="flex items-start justify-between gap-2">
            <div className="min-w-0">
              <div className="text-[10px] font-medium uppercase tracking-wide text-blue-300">
                Tube Map Path Scope
              </div>
              <div className="truncate text-[11px] text-gray-200">
                {pathScope.label ?? pathScope.prefix}
              </div>
              <div className="truncate text-[10px] text-gray-400">
                {pathScope.prefix}
              </div>
            </div>
            <button
              onClick={onClearPathScope}
              className="text-[10px] text-blue-300 hover:text-blue-200 transition-colors"
            >
              Clear
            </button>
          </div>
        </div>
      )}

      {/* Search input */}
      <input
        ref={searchInputRef}
        type="text"
        placeholder="Search entities... (/)"
        className="w-full bg-gray-800 text-gray-200 text-xs border border-gray-700 rounded px-2 py-1 placeholder-gray-600 focus:outline-none focus:border-blue-500"
        value={searchQuery}
        onChange={(e) => handleSearch(e.target.value)}
      />

      {/* Kind filters */}
      <div className="flex flex-wrap gap-1">
        {ENTITY_KINDS.map(({ value, label }) => (
          <button
            key={value}
            className={`px-1.5 py-0.5 text-[10px] rounded transition-colors ${
              activeKinds.has(value)
                ? "bg-blue-600 text-white"
                : "bg-gray-800 text-gray-500 hover:text-gray-300"
            }`}
            onClick={() => toggleKind(value)}
          >
            {label}
          </button>
        ))}
      </div>

      {/* Language dropdown */}
      {availableLanguages.length > 1 && (
        <select
          className="w-full bg-gray-800 text-gray-300 text-xs border border-gray-700 rounded px-2 py-1"
          value={activeLanguage ?? ""}
          onChange={handleLanguageChange}
        >
          <option value="">All Languages</option>
          {availableLanguages.map((lang) => (
            <option key={lang} value={lang}>
              {lang}
            </option>
          ))}
        </select>
      )}
    </div>
  );
}
