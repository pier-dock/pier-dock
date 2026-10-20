import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { disable as disableAutostart, enable as enableAutostart, isEnabled as autostartEnabled } from "@tauri-apps/plugin-autostart";
import { Bug, Lightbulb, Sparkles } from "lucide-react";
import { ipc, type ReportKind, type UpdateInfo } from "../../lib/ipc";
import { labels, os } from "../../lib/platform";
import { Card, Row, SectionLabel } from "../../ui/Card";
import { Logo } from "../../ui/Logo";
import { Segmented } from "../../ui/Segmented";
import { Switch } from "../../ui/Switch";
import type { TabProps } from "../shared";
import { ReportDialog } from "../ReportDialog";
import { useT, type Dict, type LanguageSetting } from "../../i18n";

const LANGUAGES: { value: LanguageSetting; label: string }[] = [
  { value: "pt", label: "Português" },
  { value: "en", label: "English" },
];

type UpdateStatus =
  | { kind: "checking" }
  | { kind: "latest" }
  | { kind: "available"; version: string }
  | { kind: "installing"; version: string }
  | { kind: "error"; message: string };

function updateText(status: UpdateStatus, t: Dict["general"]) {
  switch (status.kind) {
    case "checking":
      return t.checking;
    case "latest":
      return t.latest;
    case "available":
      return t.available(status.version);
    case "installing":
      return t.installing(status.version);
    case "error":
      return status.message;
  }
}

export function GeneralTab({ config, update, platform }: TabProps) {
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [version, setVersion] = useState("");
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>({ kind: "checking" });
  const [report, setReport] = useState<ReportKind | null>(null);
  const [transfer, setTransfer] = useState<string | null>(null);
  const { general: t, report: reportT } = useT();
  const platformLabels = labels();

  const checkUpdate = () => {
    setUpdateStatus({ kind: "checking" });
    ipc
      .checkUpdate()
      .then((info) => setUpdateStatus(info ? { kind: "available", version: info.version } : { kind: "latest" }))
      .catch(() => setUpdateStatus({ kind: "error", message: t.checkFailed }));
  };

  const installUpdate = (version: string) => {
    setUpdateStatus({ kind: "installing", version });
    ipc.installUpdate().catch(() => setUpdateStatus({ kind: "error", message: t.updateFailed }));
  };

  useEffect(() => {
    autostartEnabled().then(setAutostart).catch(() => setAutostart(false));
    getVersion().then(setVersion).catch(() => {});
    checkUpdate();
    const unlisten = listen<UpdateInfo>("update-available", (e) =>
      setUpdateStatus((s) => (s.kind === "installing" ? s : { kind: "available", version: e.payload.version })),
    );
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const transferConfig = (run: () => Promise<boolean>, done: string) => {
    setTransfer(null);
    run()
      .then((changed) => changed && setTransfer(done))
      .catch((e) => {
        console.error(e);
        setTransfer(t.transferFailed);
      });
  };

  const toggleAutostart = async (on: boolean) => {
    await (on ? enableAutostart() : disableAutostart());
    setAutostart(await autostartEnabled());
  };

  return (
    <div className="column">
      <div className="field">
        <SectionLabel>{t.behavior}</SectionLabel>
        <Card>
          <Row label={t.language}>
            <Segmented
              label={t.language}
              value={config.language}
              onChange={(language) => update({ language })}
              options={[{ value: "system", label: t.languageSystem }, ...LANGUAGES]}
            />
          </Row>
          <Row label={platformLabels.autostart}>
            <Switch checked={!!autostart} disabled={autostart === null} label={platformLabels.autostart} onChange={toggleAutostart} />
          </Row>
          <Row label={t.autoUpdate} description={t.autoUpdateHint}>
            <Switch checked={config.autoUpdate} label={t.autoUpdate} onChange={(autoUpdate) => update({ autoUpdate })} />
          </Row>
          {platform?.hideNative && (
            <Row label={t.replaceBar} description={platformLabels.nativeBar}>
              <Switch checked={config.hideTaskbar} label={t.replaceBar} onChange={(hideTaskbar) => update({ hideTaskbar })} />
            </Row>
          )}
        </Card>
      </div>

      {platform && os !== "windows" && (
        <div className="field">
          <SectionLabel>{t.system}</SectionLabel>
          <Card>
            <Row label={t.openWindows} description={t.permission[platform.windowList]} />
          </Card>
        </div>
      )}

      <div className="field">
        <SectionLabel>{t.settingsFile}</SectionLabel>
        <Card>
          <Row label={t.transfer} description={transfer ?? t.transferHint}>
            <button className="btn" onClick={() => transferConfig(ipc.importConfig, t.imported)}>
              {t.importConfig}
            </button>
            <button className="btn" onClick={() => transferConfig(ipc.exportConfig, t.exported)}>
              {t.exportConfig}
            </button>
          </Row>
        </Card>
      </div>

      <div className="version-card">
        <span className="version-logo">
          <Logo size={30} />
        </span>
        <div className="account-text">
          <span className="version-name">Pier {version}</span>
          <span className="account-sub">{updateText(updateStatus, t)}</span>
        </div>
        {updateStatus.kind === "available" ? (
          <button className="btn btn-primary" onClick={() => installUpdate(updateStatus.version)}>
            {t.installRestart}
          </button>
        ) : (
          <button className="btn" disabled={updateStatus.kind === "checking" || updateStatus.kind === "installing"} onClick={checkUpdate}>
            {t.checkUpdates}
          </button>
        )}
      </div>

      <div className="report-actions">
        <button className="btn" onClick={() => setReport("bug")}>
          <Bug size={14} />
          {reportT.reportProblem}
        </button>
        <button className="btn-text" onClick={() => setReport("feature")}>
          <Lightbulb size={14} />
          {reportT.suggestFeature}
        </button>
        <button className="btn-text" onClick={() => ipc.openWhatsNew().catch(console.error)}>
          <Sparkles size={14} />
          {t.whatsNew}
        </button>
      </div>
      {report && <ReportDialog kind={report} onClose={() => setReport(null)} />}

      <p className="field-hint">{t.restoreHint[os]}</p>
    </div>
  );
}
