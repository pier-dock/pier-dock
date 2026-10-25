import { useEffect, useRef, useState } from "react";
import { ArrowUpRight, Check, CircleAlert, ExternalLink, X } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ipc, type ReportEnvironment, type ReportKind, type ReportResult } from "../lib/ipc";
import { issueRef } from "../lib/report";
import { Card, Row } from "../ui/Card";
import { Switch } from "../ui/Switch";
import { useT } from "../i18n";

type Step = { step: "editing" } | { step: "sending" } | { step: "done"; result: ReportResult } | { step: "error"; message: string };

function environmentLines(env: ReportEnvironment) {
  return [`Pier ${env.version}`, `${env.os} (${env.arch})`, env.desktop].filter(Boolean).join("\n");
}

/** Confirmação do envio: a issue criada (com link) ou o aviso de que o GitHub abriu no navegador. */
function Done({ result, title }: { result: ReportResult; title: string }) {
  const t = useT().report;
  const created = result.kind === "created";
  const ref = created ? issueRef(result.url) : null;
  return (
    <div className="report-done" role="status">
      <span className={`report-done-icon ${created ? "" : "browser"}`} aria-hidden>
        {created ? <Check size={26} strokeWidth={2.6} /> : <ExternalLink size={24} />}
      </span>
      <h2 id="report-dialog-title" className="report-done-title">
        {created ? t.created : t.openedTitle}
      </h2>
      <p className="report-done-text">{created ? t.createdBody : t.openedInBrowser}</p>
      {!created && result.reason && <p className="field-hint">{t.apiFailed(result.reason)}</p>}
      {created && (
        <div className="report-issue">
          <span className="report-issue-text">
            <span className="report-issue-title">{title}</span>
            <span className="report-issue-meta">{ref ? `${ref.repo} #${ref.number} · ${t.justNow}` : t.justNow}</span>
          </span>
          <button className="btn" onClick={() => openUrl(result.url).catch(console.error)}>
            {t.openIssue}
            <ArrowUpRight size={13} aria-hidden />
          </button>
        </div>
      )}
    </div>
  );
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
    // Só ao abrir: o `kind` não muda enquanto o diálogo está aberto.
    // eslint-disable-next-line react-hooks/exhaustive-deps
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

  const again = () => {
    setTitle("");
    setDescription("");
    setAttachLogs(false);
    setLogs(null);
    setStep({ step: "editing" });
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
        {step.step === "done" ? (
          <>
            <button className="icon-button widget-dialog-close report-close" aria-label={t.close} onClick={close}>
              <X size={16} />
            </button>
            <Done result={step.result} title={title.trim()} />
            <footer className="widget-dialog-footer">
              <button className="btn-text report-again" onClick={again}>
                {t.sendAnother}
              </button>
              <button className="btn btn-primary" onClick={close} autoFocus>
                {t.done}
              </button>
            </footer>
          </>
        ) : (
          <>
            <header className="widget-dialog-header">
              <span id="report-dialog-title" className="widget-dialog-name">
                {name}
              </span>
              <button className="icon-button widget-dialog-close" aria-label={t.close} onClick={close}>
                <X size={16} />
              </button>
            </header>
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
              <p className="report-warning">
                <CircleAlert size={15} aria-hidden />
                {t.publicWarning}
              </p>
              {step.step === "error" && <p className="field-hint report-error">{t.failed(step.message)}</p>}
            </div>
            <footer className="widget-dialog-footer">
              <span className="field-hint">{connected ? t.viaAccount : t.viaBrowser}</span>
              <button className="btn-text" onClick={close}>
                {t.cancel}
              </button>
              <button className="btn btn-primary" disabled={!title.trim() || !editable} onClick={send}>
                {step.step === "sending" && <span className="report-spinner" aria-hidden />}
                {step.step === "sending" ? t.sending : connected ? t.create : t.send}
              </button>
            </footer>
          </>
        )}
      </div>
    </dialog>
  );
}
