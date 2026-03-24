import { useState } from "react";
import type { EntityKind, BuildStatus, Language } from "../types";

interface FilterBarProps {
  onSearch: (query: string) => void;
  onFilterKind: (kinds: EntityKind[] | undefined) => void;
  onFilterBuildStatus: (status: BuildStatus | undefined) => void;
  onFilterLanguage: (languages: Language[] | undefined) => void;
  availableLanguages: Language[];
  searchInputRef: React.RefObject<HTMLInputElement | null>;
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

const BUILD_STATUSES: { value: BuildStatus; label: string; color: string }[] = [
  { value: "built", label: "Built", color: "bg-green-600" },
  { value: "unbuilt", label: "Unbuilt", color: "bg-yellow-600" },
  { value: "error", label: "Error", color: "bg-red-600" },
  { value: "rebuild", label: "Rebuild", color: "bg-orange-600" },
];

export function FilterBar({
  onSearch,
  onFilterKind,
  onFilterBuildStatus,
  onFilterLanguage,
  availableLanguages,
  searchInputRef,
}: FilterBarProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [activeKinds, setActiveKinds] = useState<Set<EntityKind>>(new Set());
  const [activeStatus, setActiveStatus] = useState<BuildStatus | null>(null);
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

  const toggleStatus = (status: BuildStatus) => {
    const next = activeStatus === status ? null : status;
    setActiveStatus(next);
    onFilterBuildStatus(next ?? undefined);
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

      {/* Build status filters */}
      <div className="flex gap-1">
        {BUILD_STATUSES.map(({ value, label, color }) => (
          <button
            key={value}
            className={`px-1.5 py-0.5 text-[10px] rounded transition-colors ${
              activeStatus === value
                ? `${color} text-white`
                : "bg-gray-800 text-gray-500 hover:text-gray-300"
            }`}
            onClick={() => toggleStatus(value)}
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
