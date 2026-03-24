import { useState } from "react";
import type { ScanStats } from "./types";

function App() {
  const [stats, setStats] = useState<ScanStats | null>(null);
  const [scanning, setScanning] = useState(false);

  return (
    <div className="h-screen flex flex-col bg-gray-900 text-gray-100">
      {/* Status bar */}
      <div className="flex items-center justify-between px-4 py-2 bg-gray-800 border-b border-gray-700 text-sm">
        <span className="font-semibold">domain-scan</span>
        {scanning && <span className="text-yellow-400">Scanning...</span>}
        {stats && (
          <span className="text-gray-400">
            {stats.total_files} files | {stats.total_interfaces} interfaces |{" "}
            {stats.total_services} services | {stats.total_schemas} schemas
          </span>
        )}
      </div>

      {/* Three-panel layout */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left: Entity Tree */}
        <div className="w-72 border-r border-gray-700 overflow-y-auto p-2">
          <p className="text-gray-500 text-sm">Entity Tree</p>
          <p className="text-gray-600 text-xs mt-2">
            Open a directory to scan
          </p>
        </div>

        {/* Center: Source Preview */}
        <div className="flex-1 overflow-y-auto p-4">
          <p className="text-gray-500 text-sm">Source Preview</p>
          <p className="text-gray-600 text-xs mt-2">
            Select an entity to view source
          </p>
        </div>

        {/* Right: Details Panel */}
        <div className="w-80 border-l border-gray-700 overflow-y-auto p-4">
          <p className="text-gray-500 text-sm">Details</p>
          <p className="text-gray-600 text-xs mt-2">
            Select an entity to view details
          </p>
        </div>
      </div>
    </div>
  );
}

export default App;
