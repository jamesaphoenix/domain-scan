export type Tab = "entities" | "tube-map";

interface TabBarProps {
  activeTab: Tab;
  onTabChange: (tab: Tab) => void;
}

const tabs: { id: Tab; label: string }[] = [
  { id: "tube-map", label: "Subsystem Tube Map" },
  { id: "entities", label: "Entities/Types" },
];

export function TabBar({ activeTab, onTabChange }: TabBarProps) {
  return (
    <div className="flex items-center gap-1 bg-gray-800 border-b border-gray-700 px-4">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          className={
            activeTab === tab.id
              ? "bg-gray-700 text-white font-medium rounded-t-md px-4 py-2 text-sm"
              : "text-gray-400 hover:text-gray-200 px-4 py-2 text-sm"
          }
          onClick={() => onTabChange(tab.id)}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
