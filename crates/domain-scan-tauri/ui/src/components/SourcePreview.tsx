import { useEffect, useRef } from "react";

interface SourcePreviewProps {
  source: string | null;
  startLine: number;
  language: string | null;
  file: string | null;
}

export function SourcePreview({
  source,
  startLine,
  language,
  file,
}: SourcePreviewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const highlightRef = useRef<HTMLDivElement>(null);

  // Scroll to the highlighted line when source changes
  useEffect(() => {
    if (highlightRef.current && containerRef.current) {
      highlightRef.current.scrollIntoView({
        behavior: "smooth",
        block: "center",
      });
    }
  }, [source, startLine]);

  if (!source) {
    return (
      <div className="flex items-center justify-center h-full text-gray-600 text-sm">
        Select an entity to view source
      </div>
    );
  }

  const lines = source.split("\n");

  return (
    <div className="h-full flex flex-col">
      {/* File path header */}
      {file && (
        <div className="px-3 py-1.5 bg-gray-800/50 border-b border-gray-700 text-xs text-gray-400 truncate flex-shrink-0">
          {file}
        </div>
      )}

      {/* Code display */}
      <div ref={containerRef} className="flex-1 overflow-auto">
        <pre className="text-xs leading-5 font-mono p-3">
          {lines.map((line, i) => {
            const lineNum = startLine + i;
            const isFirst = i === 0;
            return (
              <div
                key={lineNum}
                ref={isFirst ? highlightRef : undefined}
                className={`flex ${isFirst ? "bg-blue-900/30 -mx-3 px-3" : ""}`}
              >
                <span className="inline-block w-10 text-right mr-4 text-gray-600 select-none flex-shrink-0">
                  {lineNum}
                </span>
                <span className="text-gray-200 whitespace-pre">{line}</span>
              </div>
            );
          })}
        </pre>
      </div>

      {/* Language tag */}
      {language && (
        <div className="px-3 py-1 bg-gray-800/50 border-t border-gray-700 text-[10px] text-gray-500 flex-shrink-0">
          {language}
        </div>
      )}
    </div>
  );
}
