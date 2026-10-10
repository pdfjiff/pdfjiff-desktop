import {
  BadgeCheck,
  Check,
  ChevronDown,
  ChevronUp,
  CircleAlert,
  Clock3,
  Combine,
  Copy,
  FileOutput,
  FileText,
  FolderOpen,
  HardDrive,
  Info,
  ListChecks,
  LoaderCircle,
  PackageMinus,
  RotateCcw,
  ScanSearch,
  ShieldCheck,
  Sparkles,
  WifiOff,
  X,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import {
  chooseOutput,
  choosePdf,
  choosePdfs,
  compressPdf,
  getCapabilities,
  inspectFile,
  mergePdfs,
} from "./api";
import type {
  Capabilities,
  Inspection,
  OperationResult,
  Preset,
  Tool,
} from "./types";

const tools: { id: Tool; label: string; icon: typeof PackageMinus }[] = [
  { id: "compress", label: "Compress", icon: PackageMinus },
  { id: "merge", label: "Merge", icon: Combine },
  { id: "inspect", label: "Inspect", icon: ScanSearch },
  { id: "capabilities", label: "Capabilities", icon: ListChecks },
];

function App() {
  const [tool, setTool] = useState<Tool>("compress");
  const [capabilities, setCapabilities] = useState<Capabilities | null>(null);
  const [capabilityError, setCapabilityError] = useState("");

  useEffect(() => {
    void getCapabilities()
      .then(setCapabilities)
      .catch((error: unknown) => setCapabilityError(messageOf(error)));
  }, []);

  return (
    <div className="app-shell">
      <header className="titlebar">
        <img src="/pdfjiff-header.png" alt="PDFJiff" className="brand" />
        <span className="desktop-label">Desktop</span>
        <span className="privacy-state"><ShieldCheck size={15} /> Local · Offline</span>
      </header>

      <main className="app-main">
        <nav className="tool-switcher" aria-label="PDF operations">
          {tools.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              type="button"
              className={tool === id ? "tool-tab active" : "tool-tab"}
              aria-pressed={tool === id}
              onClick={() => setTool(id)}
            >
              <Icon size={16} />
              {label}
            </button>
          ))}
        </nav>

        <section className="workspace">
          {tool === "compress" && <CompressPanel />}
          {tool === "merge" && <MergePanel />}
          {tool === "inspect" && <InspectPanel />}
          {tool === "capabilities" && (
            <CapabilitiesPanel data={capabilities} error={capabilityError} />
          )}
        </section>
      </main>

      <footer className="statusbar">
        <span><WifiOff size={13} /> No uploads, accounts, or telemetry</span>
        <span>PDFJiff processes selected files on this device</span>
      </footer>
    </div>
  );
}

function CompressPanel() {
  const [input, setInput] = useState<Inspection | null>(null);
  const [output, setOutput] = useState("");
  const [preset, setPreset] = useState<Preset>("balanced");
  const [targetMib, setTargetMib] = useState("");
  const [overwrite, setOverwrite] = useState(false);
  const [signatureAck, setSignatureAck] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [result, setResult] = useState<OperationResult | null>(null);

  async function selectInput() {
    const path = await choosePdf();
    if (!path) return;
    setError("");
    setResult(null);
    setBusy(true);
    try {
      const inspected = await inspectFile(path);
      setInput(inspected);
      setOutput(suggestOutput(path, "-compressed.pdf"));
      setSignatureAck(false);
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  async function selectOutput() {
    const path = await chooseOutput(output || "compressed.pdf");
    if (path) setOutput(path);
  }

  async function run() {
    if (!input || !output) return;
    setBusy(true);
    setError("");
    setResult(null);
    try {
      const parsedTarget = targetMib.trim() === "" ? null : Number(targetMib);
      if (parsedTarget !== null && (!Number.isFinite(parsedTarget) || parsedTarget <= 0)) {
        throw new Error("Target size must be a positive number of MiB.");
      }
      setResult(
        await compressPdf({
          input: input.path,
          output,
          preset,
          targetBytes: parsedTarget === null ? null : Math.round(parsedTarget * 1024 * 1024),
          overwrite,
          allowSignatureInvalidation: signatureAck,
        }),
      );
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  if (result) {
    return (
      <ResultView
        title="Compression complete"
        result={result}
        onReset={() => setResult(null)}
      />
    );
  }

  return (
    <TaskFrame
      eyebrow="Local compression"
      title="Make PDFs smaller"
      description="Choose a preset or an exact target. Text and vector content stay crisp."
    >
      {!input ? (
        <EmptyDrop
          icon={PackageMinus}
          title="Choose a PDF to compress"
          detail="One file at a time · maximum 128 MiB"
          action="Choose PDF"
          onClick={() => void selectInput()}
          busy={busy}
        />
      ) : (
        <div className="task-card">
          <FileRow inspection={input} onRemove={() => setInput(null)} />

          <div className="settings-grid">
            <label className="field">
              <span>Compression</span>
              <select value={preset} onChange={(event) => setPreset(event.target.value as Preset)}>
                <option value="quality">Quality · lighter reduction</option>
                <option value="balanced">Balanced · recommended</option>
                <option value="small">Small · stronger reduction</option>
                <option value="lossless">Lossless · preserve image pixels</option>
              </select>
            </label>
            <label className="field">
              <span>Target size, MiB <em>optional</em></span>
              <input
                inputMode="decimal"
                value={targetMib}
                onChange={(event) => setTargetMib(event.target.value)}
                placeholder="For example, 2"
              />
            </label>
          </div>

          <OutputPicker value={output} onClick={() => void selectOutput()} />

          {input.signaturesDetected && (
            <Acknowledgement
              checked={signatureAck}
              onChange={setSignatureAck}
              title="Signature detected"
              detail="Rewriting this PDF invalidates its existing signature."
            />
          )}

          <label className="check-row">
            <input type="checkbox" checked={overwrite} onChange={(event) => setOverwrite(event.target.checked)} />
            Replace an existing output with this exact name
          </label>

          <ErrorBanner message={error} />
          <div className="action-row">
            <span className="supporting"><Sparkles size={15} /> Output is validated before publication</span>
            <PrimaryButton
              busy={busy}
              disabled={!output || (input.signaturesDetected && !signatureAck)}
              onClick={() => void run()}
            >
              Compress PDF
            </PrimaryButton>
          </div>
        </div>
      )}
    </TaskFrame>
  );
}

function MergePanel() {
  const [inputs, setInputs] = useState<Inspection[]>([]);
  const [output, setOutput] = useState("");
  const [overwrite, setOverwrite] = useState(false);
  const [signatureAck, setSignatureAck] = useState(false);
  const [structureAck, setStructureAck] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [result, setResult] = useState<OperationResult | null>(null);

  const signaturesDetected = inputs.some((item) => item.signaturesDetected);
  const sensitiveFeatures = useMemo(
    () => [...new Set(inputs.flatMap((item) => item.assemblySensitiveFeatures))],
    [inputs],
  );
  const pageCount = inputs.reduce((sum, item) => sum + (item.pageCount ?? 0), 0);

  async function addFiles() {
    const paths = await choosePdfs();
    if (paths.length === 0) return;
    setBusy(true);
    setError("");
    setResult(null);
    try {
      const inspected = await Promise.all(paths.map(inspectFile));
      const known = new Set(inputs.map((item) => item.path));
      const next = [...inputs, ...inspected.filter((item) => !known.has(item.path))];
      setInputs(next);
      if (!output && next[0]) setOutput(suggestOutput(next[0].path, "-merged.pdf"));
      setSignatureAck(false);
      setStructureAck(false);
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  function move(index: number, direction: -1 | 1) {
    const destination = index + direction;
    if (destination < 0 || destination >= inputs.length) return;
    const next = [...inputs];
    [next[index], next[destination]] = [next[destination], next[index]];
    setInputs(next);
  }

  async function run() {
    setBusy(true);
    setError("");
    setResult(null);
    try {
      setResult(
        await mergePdfs({
          inputs: inputs.map((item) => item.path),
          output,
          overwrite,
          allowSignatureInvalidation: signatureAck,
          allowStructureLoss: structureAck,
        }),
      );
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  if (result) {
    return <ResultView title="Merge complete" result={result} onReset={() => setResult(null)} />;
  }

  return (
    <TaskFrame
      eyebrow="Ordered assembly"
      title="Combine PDFs"
      description="Arrange files in the exact order they should appear in the result."
    >
      {inputs.length === 0 ? (
        <EmptyDrop
          icon={Combine}
          title="Choose PDFs to merge"
          detail="Select two or more files · maximum 256 MiB total"
          action="Choose PDFs"
          onClick={() => void addFiles()}
          busy={busy}
        />
      ) : (
        <div className="task-card">
          <div className="merge-summary">
            <span>{inputs.length} files</span><span>{pageCount} pages</span>
            <button type="button" className="text-button" onClick={() => void addFiles()}>Add PDFs</button>
          </div>
          <ol className="merge-list">
            {inputs.map((item, index) => (
              <li key={item.path}>
                <span className="order-number">{index + 1}</span>
                <FileText size={19} />
                <span className="merge-file"><strong>{item.fileName}</strong><small>{formatBytes(item.inputBytes)} · {item.pageCount ?? "?"} pages</small></span>
                <button type="button" aria-label={`Move ${item.fileName} up`} disabled={index === 0} onClick={() => move(index, -1)}><ChevronUp size={16} /></button>
                <button type="button" aria-label={`Move ${item.fileName} down`} disabled={index === inputs.length - 1} onClick={() => move(index, 1)}><ChevronDown size={16} /></button>
                <button type="button" aria-label={`Remove ${item.fileName}`} onClick={() => setInputs(inputs.filter((_, itemIndex) => itemIndex !== index))}><X size={16} /></button>
              </li>
            ))}
          </ol>

          <OutputPicker value={output} onClick={async () => {
            const selected = await chooseOutput(output || "merged.pdf");
            if (selected) setOutput(selected);
          }} />

          {sensitiveFeatures.length > 0 && (
            <Acknowledgement
              checked={structureAck}
              onChange={setStructureAck}
              title="Some structures cannot be preserved"
              detail={`The merged result omits ${sensitiveFeatures.join(", ")}.`}
            />
          )}
          {signaturesDetected && (
            <Acknowledgement
              checked={signatureAck}
              onChange={setSignatureAck}
              title="Signature detected"
              detail="Signatures from source PDFs are not valid in the merged document."
            />
          )}
          <label className="check-row">
            <input type="checkbox" checked={overwrite} onChange={(event) => setOverwrite(event.target.checked)} />
            Replace an existing output with this exact name
          </label>

          <ErrorBanner message={error} />
          <div className="action-row">
            <span className="supporting"><HardDrive size={15} /> Written atomically beside your files</span>
            <PrimaryButton
              busy={busy}
              disabled={inputs.length < 2 || !output || (signaturesDetected && !signatureAck) || (sensitiveFeatures.length > 0 && !structureAck)}
              onClick={() => void run()}
            >
              Merge {inputs.length} PDFs
            </PrimaryButton>
          </div>
        </div>
      )}
    </TaskFrame>
  );
}

function InspectPanel() {
  const [inspection, setInspection] = useState<Inspection | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);

  async function selectFile() {
    const path = await choosePdf();
    if (!path) return;
    setBusy(true);
    setError("");
    try {
      setInspection(await inspectFile(path));
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  async function copyReport() {
    if (!inspection) return;
    await navigator.clipboard.writeText(JSON.stringify({ schemaVersion: 1, operation: "document.inspect", result: inspection }, null, 2));
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1600);
  }

  return (
    <TaskFrame
      eyebrow="Read-only analysis"
      title="Inspect a PDF"
      description="See geometry, encryption, signatures and document structures without rewriting the file."
    >
      {!inspection ? (
        <>
          <EmptyDrop icon={ScanSearch} title="Choose a PDF to inspect" detail="Structural inspection · no visual validation" action="Choose PDF" onClick={() => void selectFile()} busy={busy} />
          <ErrorBanner message={error} />
        </>
      ) : (
        <div className="task-card inspection-card">
          <FileRow inspection={inspection} onRemove={() => setInspection(null)} />
          <div className="metric-grid">
            <Metric value={String(inspection.pageCount ?? "Unknown")} label="Pages" />
            <Metric value={formatBytes(inspection.inputBytes)} label="File size" />
            <Metric value={inspection.encrypted ? "Yes" : "No"} label="Encrypted" />
            <Metric value={inspection.signaturesDetected ? "Detected" : "None"} label="Signature" />
          </div>

          {inspection.assemblySensitiveFeatures.length > 0 && (
            <div className="structure-panel">
              <h3><Info size={16} /> Detected structures</h3>
              <div className="tag-list">{inspection.assemblySensitiveFeatures.map((feature) => <span key={feature}>{feature}</span>)}</div>
            </div>
          )}

          {inspection.pages.length > 0 && (
            <div className="table-wrap">
              <table>
                <thead><tr><th>Page</th><th>Width</th><th>Height</th><th>Rotation</th></tr></thead>
                <tbody>
                  {inspection.pages.slice(0, 80).map((page) => (
                    <tr key={page.number}><td>{page.number}</td><td>{page.widthPt.toFixed(1)} pt</td><td>{page.heightPt.toFixed(1)} pt</td><td>{page.rotation}°</td></tr>
                  ))}
                </tbody>
              </table>
              {inspection.pages.length > 80 && <p className="table-note">Showing the first 80 of {inspection.pages.length} pages.</p>}
            </div>
          )}

          <div className="action-row">
            <span className="supporting"><CircleAlert size={15} /> Detection does not validate signatures or PDF/A</span>
            <button type="button" className="primary-button" onClick={() => void copyReport()}>{copied ? <Check size={17} /> : <Copy size={17} />}{copied ? "Copied" : "Copy JSON report"}</button>
          </div>
        </div>
      )}
    </TaskFrame>
  );
}

function CapabilitiesPanel({ data, error }: { data: Capabilities | null; error: string }) {
  return (
    <TaskFrame
      eyebrow="Packaged runtime"
      title="Engine capabilities"
      description="This list comes from the app you are running, so unavailable tools are never implied."
    >
      <div className="task-card capability-card">
        {error && <ErrorBanner message={error} />}
        {!data && !error && <div className="loading-line"><LoaderCircle className="spin" size={18} /> Reading the native engine…</div>}
        {data && (
          <>
            <div className="runtime-strip">
              <span><BadgeCheck size={16} /> PDFJiff {data.appVersion}</span>
              <span>Schema {data.schemaVersion}</span>
              <span>{data.offline ? "Offline" : "Network enabled"}</span>
            </div>
            <h3 className="section-label">Available</h3>
            <div className="capability-list">
              {data.available.map((item) => <CapabilityRow key={item.id} available label={item.label} description={item.description} />)}
            </div>
            <h3 className="section-label">Planned</h3>
            <div className="capability-list muted-list">
              {data.planned.map((item) => <CapabilityRow key={item.id} label={item.label} description={item.description} />)}
            </div>
          </>
        )}
      </div>
    </TaskFrame>
  );
}

function TaskFrame({ eyebrow, title, description, children }: { eyebrow: string; title: string; description: string; children: React.ReactNode }) {
  return (
    <div className="task-frame">
      <div className="task-heading">
        <span>{eyebrow}</span>
        <h1>{title}</h1>
        <p>{description}</p>
      </div>
      {children}
    </div>
  );
}

function EmptyDrop({ icon: Icon, title, detail, action, onClick, busy }: { icon: typeof PackageMinus; title: string; detail: string; action: string; onClick: () => void; busy: boolean }) {
  return (
    <div className="empty-drop">
      <div className="empty-icon"><Icon size={25} /></div>
      <h2>{title}</h2>
      <p>{detail}</p>
      <PrimaryButton busy={busy} onClick={onClick}>{action}</PrimaryButton>
    </div>
  );
}

function FileRow({ inspection, onRemove }: { inspection: Inspection; onRemove: () => void }) {
  return (
    <div className="file-row">
      <div className="file-icon"><FileText size={20} /></div>
      <div className="file-copy"><strong>{inspection.fileName}</strong><span>{formatBytes(inspection.inputBytes)} · {inspection.pageCount ?? "Unknown"} pages</span></div>
      <span className="ready-state"><Check size={14} /> Ready</span>
      <button type="button" className="icon-button" aria-label={`Remove ${inspection.fileName}`} onClick={onRemove}><X size={16} /></button>
    </div>
  );
}

function OutputPicker({ value, onClick }: { value: string; onClick: () => void }) {
  return (
    <button type="button" className="output-picker" onClick={onClick}>
      <FileOutput size={18} />
      <span><small>Output file</small><strong>{value || "Choose where to save"}</strong></span>
      <FolderOpen size={17} />
    </button>
  );
}

function Acknowledgement({ checked, onChange, title, detail }: { checked: boolean; onChange: (checked: boolean) => void; title: string; detail: string }) {
  return (
    <label className="acknowledgement">
      <input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
      <span><strong>{title}</strong><small>{detail}</small></span>
    </label>
  );
}

function PrimaryButton({ children, busy = false, disabled = false, onClick }: { children: React.ReactNode; busy?: boolean; disabled?: boolean; onClick: () => void }) {
  return (
    <button type="button" className="primary-button" disabled={disabled || busy} onClick={onClick}>
      {busy && <LoaderCircle className="spin" size={17} />}
      {children}
    </button>
  );
}

function ResultView({ title, result, onReset }: { title: string; result: OperationResult; onReset: () => void }) {
  return (
    <TaskFrame eyebrow="Finished safely" title={title} description="The output was validated and published atomically.">
      <div className="task-card result-card">
        <div className="result-mark"><Check size={28} /></div>
        <h2>{fileName(result.output)}</h2>
        <p className="result-path">{result.output}</p>
        <div className="metric-grid">
          <Metric value={formatBytes(result.outputBytes)} label="Output" />
          <Metric value={String(result.pageCount)} label="Pages" />
          <Metric value={result.savedRatio === null ? "—" : `${Math.round(result.savedRatio * 100)}%`} label="Saved" />
          <Metric value={`${result.elapsedMs} ms`} label="Elapsed" />
        </div>
        {result.warnings.map((warning) => <div className="warning-row" key={warning}><CircleAlert size={16} />{warning}</div>)}
        <button type="button" className="secondary-button" onClick={onReset}><RotateCcw size={16} /> Process another</button>
      </div>
    </TaskFrame>
  );
}

function Metric({ value, label }: { value: string; label: string }) {
  return <div className="metric"><strong>{value}</strong><span>{label}</span></div>;
}

function CapabilityRow({ available = false, label, description }: { available?: boolean; label: string; description: string }) {
  return (
    <div className="capability-row">
      {available ? <Check size={18} /> : <Clock3 size={18} />}
      <span><strong>{label}</strong><small>{description}</small></span>
    </div>
  );
}

function ErrorBanner({ message }: { message: string }) {
  if (!message) return null;
  return <div className="error-banner" role="alert"><CircleAlert size={17} />{message}</div>;
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function formatBytes(value: number): string {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KiB`;
  return `${(value / (1024 * 1024)).toFixed(1)} MiB`;
}

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

function suggestOutput(path: string, suffix: string): string {
  return path.replace(/\.pdf$/i, "") + suffix;
}

export default App;
