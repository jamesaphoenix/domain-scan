import { useToast, type ToastType } from "../hooks/useToast";

const typeStyles: Record<ToastType, string> = {
  success:
    "bg-emerald-900/90 border-emerald-700 text-emerald-200",
  error:
    "bg-red-900/90 border-red-700 text-red-200",
  info:
    "bg-slate-800/90 border-slate-600 text-slate-200",
};

const typeIcons: Record<ToastType, string> = {
  success: "\u2713",
  error: "\u2717",
  info: "\u2139",
};

export function ToastContainer() {
  const { toasts, removeToast } = useToast();

  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-12 right-4 z-50 flex flex-col gap-2 pointer-events-none">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`pointer-events-auto flex items-center gap-2 px-3 py-2 rounded-md border text-xs shadow-lg animate-slide-in ${typeStyles[toast.type]}`}
          role="status"
        >
          <span className="font-bold text-sm">{typeIcons[toast.type]}</span>
          <span className="flex-1">{toast.message}</span>
          <button
            onClick={() => removeToast(toast.id)}
            className="ml-2 opacity-60 hover:opacity-100 transition-opacity"
          >
            &times;
          </button>
        </div>
      ))}
    </div>
  );
}
