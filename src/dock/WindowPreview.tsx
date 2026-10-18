import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { ipc, type WindowInfo } from "../lib/ipc";
import { useIcon } from "../hooks/useIcon";
import { useT } from "../i18n";
import { PREVIEW_HEIGHT, previewLeft } from "./layout";
import type { DockApp } from "./apps";

/** Acima disso a fileira fica longa demais para caber na tela. */
const MAX_WINDOWS = 8;
/** Largura estimada de um card + folga, só para o primeiro paint (antes de medir a faixa). */
const CARD_ESTIMATE = 200;

interface Props {
  app: DockApp;
  centerX: number;
  onClose: () => void;
}

interface CardProps {
  info: WindowInfo;
  thumbnail: string | null;
  icon: string | null;
  onActivate: () => void;
  onClose: () => void;
}

/** Card de uma janela: prévia (ou ícone do app) e título, com o botão de fechar no canto. */
function PreviewCard({ info, thumbnail, icon, onActivate, onClose }: CardProps) {
  const t = useT().preview;
  return (
    <div className="preview-card">
      {/* O fechar é irmão do card: um botão dentro de outro botão não é HTML válido. */}
      <button className="preview-card-body" title={info.title} onClick={onActivate}>
        <span className="preview-thumb">
          {thumbnail ? (
            <img src={thumbnail} alt="" draggable={false} />
          ) : (
            <span className="preview-fallback">
              {icon ? <img src={icon} alt="" draggable={false} /> : <span className="preview-fallback-text">{t.fallback}</span>}
              {info.minimized && <span className="preview-minimized">{t.minimized}</span>}
            </span>
          )}
        </span>
        <span className="preview-title">{info.title}</span>
      </button>
      <button
        className="preview-close"
        aria-label={t.close}
        onClick={(e) => {
          e.stopPropagation();
          onClose();
        }}
      >
        <X size={11} strokeWidth={2.5} aria-hidden />
      </button>
    </div>
  );
}

/** Fileira de prévias das janelas de um app, acima da barra da dock. */
export function WindowPreview({ app, centerX, onClose }: Props) {
  const t = useT().preview;
  const icon = useIcon(app.iconKey);
  const stripRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [viewport, setViewport] = useState(() => window.innerWidth);
  const [thumbs, setThumbs] = useState<Record<number, string | null>>({});
  const windows = app.windows.slice(0, MAX_WINDOWS);
  // A lista de janelas é remontada a cada snapshot: o efeito de captura só pode depender dos ids,
  // senão as capturas recomeçariam a cada render.
  const ids = windows.map((w) => w.id).join(",");
  // Antes de medir a faixa, o palpite evita ela "pular" para o lugar no primeiro paint.
  const estimate = windows.length * CARD_ESTIMATE + 16;

  // Uma captura custa dezenas de ms: em série, para não travar a composição do sistema. Nada
  // bloqueia a renderização; o ícone do app aparece na hora e a imagem entra quando chega.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      for (const w of windows) {
        // Minimizada não tem captura possível no Linux; nem vale a ida ao Rust.
        if (w.minimized) continue;
        const src = await ipc.windowThumbnail(w.id).catch(() => null);
        if (cancelled) return;
        setThumbs((prev) => ({ ...prev, [w.id]: src }));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [ids]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  useEffect(() => {
    const onResize = () => setViewport(window.innerWidth);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  useLayoutEffect(() => {
    const strip = stripRef.current;
    if (!strip) return;
    const measure = () => setWidth(strip.offsetWidth);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(strip);
    return () => observer.disconnect();
  }, []);

  return (
    <div
      ref={stripRef}
      className="preview-strip"
      role="dialog"
      aria-label={t.label(app.name)}
      style={{ bottom: "var(--preview-bottom)", height: PREVIEW_HEIGHT, left: previewLeft(centerX, viewport, width || estimate) }}
    >
      {windows.map((w) => (
        <PreviewCard
          key={w.id}
          info={w}
          thumbnail={thumbs[w.id] ?? null}
          icon={icon}
          onActivate={() => {
            ipc.activateWindow(w.id).catch(console.error);
            onClose();
          }}
          onClose={() => ipc.closeWindows([w.id]).catch(console.error)}
        />
      ))}
    </div>
  );
}
