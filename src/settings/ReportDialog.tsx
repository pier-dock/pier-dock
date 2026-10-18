import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ipc, type ReportEnvironment, type ReportKind, type ReportResult } from "../lib/ipc";
import { Card, Row } from "../ui/Card";
import { Switch } from "../ui/Switch";
import { useT } from "../i18n";

type Step = { step: "editing" } | { step: "sending" } | { step: "done"; result: ReportResult } | { step: "error"; message: string };

function environmentLines(env: ReportEnvironment) {
  return [`Pier ${env.version}`, `${env.os} (${env.arch})`, env.desktop].filter(Boolean).join("\n");
}

/** Monta a issue com título, descrição, ambiente e logs revisados, e a envia (ou abre no GitHub). */
export function ReportDialog({ kind, onClose }: { kind: ReportKind; onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null);
  const t = useT().report;
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [env, setEnv] = useState<ReportEnvironment | null>(null);
  const [attachLogs, setAttachLogs] = useState(false);
  const [logs, setLogs] = useState<string | null>(null);
  const [connected, setConnected] = useState(false);
  const [step, setStep] = useState<Step>({ step: "editing" });

  useEffect(() => {
    if (!ref.current?.open) ref.current?.showModal();
    ipc
      .githubAuthStatus()
      .then((s) => setConnected(s.source !== null))
      .catch(() => setConnected(false));
    if (kind === "bug") ipc.reportEnvironment().then(setEnv).catch(console.error);
  }, []);

  const toggleLogs = (on: boolean) => {
    setAttachLogs(on);
    if (on && logs === null) ipc.recentLogs().then(setLogs).catch(() => setLogs(""));
  };

  const send = () => {
    setStep({ step: "sending" });
    ipc
      .reportIssue({ kind, title, description, logs: attachLogs ? logs : null })
      .then((result) => {
        setStep({ step: "done", result });
        if (result.kind === "fallback") openUrl(result.url).catch(console.error);
      })
      .catch((e) => setStep({ step: "error", message: String(e) }));
  };

  const close = () => ref.current?.close();
  const name = kind === "bug" ? t.reportProblem : t.suggestFeature;
  const editable = step.step === "editing" || step.step === "error";

  return (
    <dialog
      ref={ref}
      className="widget-dialog report-dialog"
      aria-labelledby="report-dialog-title"
      onClose={onClose}
      onClick={(e) => e.target === ref.current && close()}
    >
      <div className="widget-dialog-content">
        <header className="widget-dialog-header">
          <span id="report-dialog-title" className="widget-dialog-name">
            {name}
          </span>
          <button className="icon-button widget-dialog-close" aria-label={t.close} onClick={close}>
            <X size={16} />
          </button>
        </header>

        {step.step === "done" ? (
          <div className="widget-dialog-body report-done">
            {step.result.kind === "created" ? (
              <>
                <p className="report-done-title">{t.created}</p>
                <button className="btn" onClick={() => openUrl(step.result.url).catch(console.error)} autoFocus>
                  {t.openIssue}
                </button>
              </>
            ) : (
              <>
                <p className="report-done-title">{t.openedInBrowser}</p>
                {step.result.reason && <p className="field-hint">{t.apiFailed(step.result.reason)}</p>}
              </>
            )}
          </div>
        ) : (
          <div className="widget-dialog-body report-body">
            <label className="report-field">
              <span className="field-label">{t.title}</span>
              <input
                className="text-input"
                value={title}
                placeholder={t.titlePlaceholder[kind]}
                disabled={!editable}
                autoFocus
                onChange={(e) => setTitle(e.target.value)}
              />
            </label>
            <label className="report-field">
              <span className="field-label">{t.description}</span>
              <textarea
                className="text-input report-textarea"
                rows={5}
                value={description}
                placeholder={t.descriptionPlaceholder[kind]}
                disabled={!editable}
                onChange={(e) => setDescription(e.target.value)}
              />
            </label>
            {kind === "bug" && (
              <Card>
                <Row label={t.environment} description={env ? <span className="report-env">{environmentLines(env)}</span> : undefined} />
                <Row label={t.attachLogs} description={t.logsHint}>
                  <Switch checked={attachLogs} label={t.attachLogs} disabled={!editable} onChange={toggleLogs} />
                </Row>
              </Card>
            )}
            {kind === "bug" && attachLogs && logs !== null && (
              <textarea
                className="text-input report-textarea report-logs"
                rows={8}
                value={logs}
                placeholder={t.noLogs}
                spellCheck={false}
                disabled={!editable}
                aria-label={t.attachLogs}
                onChange={(e) => setLogs(e.target.value)}
              />
            )}
            {step.step === "error" && <p className="field-hint report-error">{t.failed(step.message)}</p>}
          </div>
        )}

        <footer className="widget-dialog-footer">
          {step.step === "done" ? (
            <button className="btn btn-primary" onClick={close}>
              {t.done}
            </button>
          ) : (
            <>
              <span className="field-hint">{connected ? t.viaAccount : t.viaBrowser}</span>
              <button className="btn btn-primary" disabled={!title.trim() || !editable} onClick={send}>
                {step.step === "sending" ? t.sending : connected ? t.create : t.continueOnGithub}
              </button>
            </>
          )}
        </footer>
      </div>
    </dialog>
  );
}
