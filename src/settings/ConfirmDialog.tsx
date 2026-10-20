import { useEffect, useRef } from "react";
import { X } from "lucide-react";
import { useT } from "../i18n";

/** Confirmação de uma ação que não dá para desfazer, dentro da própria janela. */
export function ConfirmDialog({
  title,
  description,
  confirmLabel,
  onConfirm,
  onClose,
}: {
  title: string;
  description: string;
  confirmLabel: string;
  onConfirm: () => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const t = useT().confirm;

  useEffect(() => {
    if (!ref.current?.open) ref.current?.showModal();
  }, []);

  const close = () => ref.current?.close();

  return (
    <dialog
      ref={ref}
      className="widget-dialog confirm-dialog"
      aria-labelledby="confirm-dialog-title"
      aria-describedby="confirm-dialog-description"
      onClose={onClose}
      onClick={(e) => e.target === ref.current && close()}
    >
      <div className="widget-dialog-content">
        <header className="widget-dialog-header">
          <span id="confirm-dialog-title" className="widget-dialog-name">
            {title}
          </span>
          <button className="icon-button widget-dialog-close" aria-label={t.close} onClick={close}>
            <X size={16} />
          </button>
        </header>
        <p id="confirm-dialog-description" className="widget-dialog-description">
          {description}
        </p>
        <footer className="widget-dialog-footer">
          <button className="btn-text" onClick={close} autoFocus>
            {t.cancel}
          </button>
          <button
            className="btn btn-danger"
            onClick={() => {
              onConfirm();
              close();
            }}
          >
            {confirmLabel}
          </button>
        </footer>
      </div>
    </dialog>
  );
}
