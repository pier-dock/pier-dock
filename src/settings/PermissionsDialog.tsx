import { useEffect, useRef } from "react";
import { X } from "lucide-react";
import { ipc, type Permission } from "../lib/ipc";
import { labels } from "../lib/platform";
import { Card, Row } from "../ui/Card";
import { useT } from "../i18n";

const PILL: Record<Permission["status"], string> = { granted: "ok", denied: "warn", "not-determined": "warn", unknown: "off" };

/** Uma linha por permissão, com o estado e o que fazer: pedir ao sistema ou abrir os Ajustes. */
export function PermissionsDialog({ permissions, refresh, onClose }: { permissions: Permission[]; refresh: () => void; onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null);
  const t = useT().permissions;
  const openSettings = labels().openSettings;

  useEffect(() => {
    if (!ref.current?.open) ref.current?.showModal();
    // O usuário muda a permissão nos Ajustes e volta: relê ao focar a janela de novo.
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, [refresh]);

  const act = (id: Permission["id"]) => {
    ipc.permissionAction(id).catch(console.error);
    setTimeout(refresh, 1_000);
  };
  const close = () => ref.current?.close();

  return (
    <dialog
      ref={ref}
      className="widget-dialog permissions-dialog"
      aria-labelledby="permissions-dialog-title"
      onClose={onClose}
      onClick={(e) => e.target === ref.current && close()}
    >
      <div className="widget-dialog-content">
        <header className="widget-dialog-header">
          <span id="permissions-dialog-title" className="widget-dialog-name">
            {t.title}
          </span>
          <button className="icon-button widget-dialog-close" aria-label={t.close} onClick={close}>
            <X size={16} />
          </button>
        </header>
        <div className="widget-dialog-body">
          <p className="widget-dialog-description">{t.intro}</p>
          <Card>
            {permissions.map((p) => (
              <Row key={p.id} label={t.names[p.id]} description={t.why[p.id]}>
                <span className={`status-pill ${PILL[p.status]}`}>{t.status[p.status]}</span>
                {p.status !== "granted" && (
                  <button className="btn" onClick={() => act(p.id)}>
                    {p.status === "not-determined" ? t.grant : openSettings}
                  </button>
                )}
              </Row>
            ))}
          </Card>
        </div>
      </div>
    </dialog>
  );
}
