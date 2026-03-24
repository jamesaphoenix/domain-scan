import type { BreadcrumbItem } from "../hooks/useTubeMapState";

interface BreadcrumbsProps {
  items: BreadcrumbItem[];
  onNavigate: (index: number) => void;
}

export function Breadcrumbs({ items, onNavigate }: BreadcrumbsProps) {
  return (
    <nav className="flex items-center gap-1 text-sm">
      {items.map((item, index) => {
        const isLast = index === items.length - 1;
        return (
          <span key={item.id} className="flex items-center gap-1">
            {index > 0 && <span className="text-slate-600 mx-0.5">/</span>}
            {isLast ? (
              <span className="text-slate-200 font-medium">{item.name}</span>
            ) : (
              <button
                onClick={() => onNavigate(index)}
                className="text-slate-400 hover:text-slate-200 transition-colors cursor-pointer"
              >
                {item.name}
              </button>
            )}
          </span>
        );
      })}
    </nav>
  );
}
