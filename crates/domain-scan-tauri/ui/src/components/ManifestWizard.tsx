import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { WizardStepDomains } from "./WizardStepDomains";
import { WizardStepSubsystems } from "./WizardStepSubsystems";
import { WizardStepConnections } from "./WizardStepConnections";
import { WizardStepReview } from "./WizardStepReview";
import { useToast } from "../hooks/useToast";
import type {
  SystemManifest,
  DomainDef,
  ManifestSubsystem,
  ManifestConnection,
} from "../types";

export type WizardStep = "domains" | "subsystems" | "connections" | "review";

const STEPS: WizardStep[] = ["domains", "subsystems", "connections", "review"];

const STEP_LABELS: Record<WizardStep, string> = {
  domains: "Domains",
  subsystems: "Subsystems",
  connections: "Connections",
  review: "Review & Save",
};

interface ManifestWizardProps {
  onComplete: (manifestPath: string) => void;
  onCancel: () => void;
}

export function ManifestWizard({ onComplete, onCancel }: ManifestWizardProps) {
  const [currentStep, setCurrentStep] = useState<WizardStep>("domains");
  const [bootstrapping, setBootstrapping] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Draft manifest state
  const [domains, setDomains] = useState<Record<string, DomainDef>>({});
  const [subsystems, setSubsystems] = useState<ManifestSubsystem[]>([]);
  const [connections, setConnections] = useState<ManifestConnection[]>([]);
  const [meta, setMeta] = useState({ name: "", version: "1.0.0", description: "" });

  const currentStepIndex = STEPS.indexOf(currentStep);

  const handleBootstrap = useCallback(async () => {
    setBootstrapping(true);
    setError(null);
    try {
      const manifest = await invoke<SystemManifest>("bootstrap_manifest", {
        projectName: meta.name || null,
      });
      setMeta(manifest.meta);
      setDomains(manifest.domains);
      setSubsystems(manifest.subsystems);
      setConnections(manifest.connections);
    } catch (e) {
      setError(String(e));
    } finally {
      setBootstrapping(false);
    }
  }, [meta.name]);

  const { addToast } = useToast();

  const handleSave = useCallback(async () => {
    const path = await save({
      defaultPath: "system.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;

    setSaving(true);
    setError(null);
    try {
      const manifest: SystemManifest = {
        meta,
        domains,
        subsystems,
        connections,
      };
      await invoke("save_manifest", { manifestJson: manifest, path });
      addToast(`Manifest saved to ${path.split("/").pop()}`, "success");
      onComplete(path);
    } catch (e) {
      setError(String(e));
      addToast(`Failed to save manifest: ${String(e)}`, "error");
    } finally {
      setSaving(false);
    }
  }, [meta, domains, subsystems, connections, addToast, onComplete]);

  const goNext = useCallback(() => {
    const idx = STEPS.indexOf(currentStep);
    if (idx < STEPS.length - 1) setCurrentStep(STEPS[idx + 1]);
  }, [currentStep]);

  const goPrev = useCallback(() => {
    const idx = STEPS.indexOf(currentStep);
    if (idx > 0) setCurrentStep(STEPS[idx - 1]);
  }, [currentStep]);

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* Header with progress */}
      <div className="flex items-center justify-between px-6 py-3 border-b border-slate-700/50 bg-slate-900/60 flex-shrink-0">
        <div className="flex items-center gap-4">
          <h2 className="text-sm font-semibold text-slate-200">
            Manifest Wizard
          </h2>
          {/* Step indicators */}
          <div className="flex items-center gap-1">
            {STEPS.map((step, i) => (
              <div key={step} className="flex items-center">
                {i > 0 && (
                  <div
                    className={`w-8 h-px mx-1 ${
                      i <= currentStepIndex
                        ? "bg-blue-500"
                        : "bg-slate-700"
                    }`}
                  />
                )}
                <button
                  onClick={() => setCurrentStep(step)}
                  className={`flex items-center gap-1.5 px-2.5 py-1 rounded text-xs transition-colors ${
                    step === currentStep
                      ? "bg-blue-600/20 text-blue-400 font-medium"
                      : i < currentStepIndex
                        ? "text-slate-300 hover:text-slate-100"
                        : "text-slate-500 hover:text-slate-400"
                  }`}
                >
                  <span
                    className={`w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold ${
                      step === currentStep
                        ? "bg-blue-600 text-white"
                        : i < currentStepIndex
                          ? "bg-slate-600 text-slate-200"
                          : "bg-slate-800 text-slate-500 border border-slate-700"
                    }`}
                  >
                    {i + 1}
                  </span>
                  {STEP_LABELS[step]}
                </button>
              </div>
            ))}
          </div>
        </div>
        <button
          onClick={onCancel}
          className="text-xs text-slate-500 hover:text-slate-300 transition-colors"
        >
          Cancel
        </button>
      </div>

      {/* Error display */}
      {error && (
        <div className="mx-6 mt-3 text-xs text-red-400 bg-red-950/50 border border-red-800/50 rounded-md px-3 py-2">
          {error}
          <button
            onClick={() => setError(null)}
            className="ml-2 text-red-500 hover:text-red-300"
          >
            dismiss
          </button>
        </div>
      )}

      {/* Step content */}
      <div className="flex-1 overflow-y-auto">
        {currentStep === "domains" && (
          <WizardStepDomains
            domains={domains}
            meta={meta}
            onDomainsChange={setDomains}
            onMetaChange={setMeta}
            onBootstrap={handleBootstrap}
            bootstrapping={bootstrapping}
            hasData={subsystems.length > 0}
          />
        )}
        {currentStep === "subsystems" && (
          <WizardStepSubsystems
            subsystems={subsystems}
            domains={domains}
            onSubsystemsChange={setSubsystems}
          />
        )}
        {currentStep === "connections" && (
          <WizardStepConnections
            connections={connections}
            subsystems={subsystems}
            onConnectionsChange={setConnections}
          />
        )}
        {currentStep === "review" && (
          <WizardStepReview
            meta={meta}
            domains={domains}
            subsystems={subsystems}
            connections={connections}
          />
        )}
      </div>

      {/* Footer: navigation buttons */}
      <div className="flex items-center justify-between px-6 py-3 border-t border-slate-700/50 bg-slate-900/60 flex-shrink-0">
        <button
          onClick={goPrev}
          disabled={currentStepIndex === 0}
          className="px-4 py-1.5 rounded text-xs text-slate-400 hover:text-slate-200 disabled:text-slate-600 disabled:cursor-not-allowed transition-colors"
        >
          Back
        </button>
        <div className="flex items-center gap-2">
          {currentStep === "review" ? (
            <button
              onClick={handleSave}
              disabled={saving || subsystems.length === 0}
              className="px-4 py-1.5 rounded text-xs font-medium bg-blue-600 hover:bg-blue-500 disabled:bg-blue-600/50 text-white transition-colors"
            >
              {saving ? "Saving..." : "Save Manifest"}
            </button>
          ) : (
            <button
              onClick={goNext}
              className="px-4 py-1.5 rounded text-xs font-medium bg-blue-600 hover:bg-blue-500 text-white transition-colors"
            >
              Next
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
