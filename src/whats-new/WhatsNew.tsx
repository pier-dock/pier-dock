import { useEffect, useMemo, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import changelogEn from "../../CHANGELOG.md?raw";
import changelogPt from "../../CHANGELOG.pt-BR.md?raw";
import { entriesBetween, parseChangelog, type Block, type Inline } from "../lib/changelog";
import { ActionFooter } from "../ui/ActionFooter";
import { Logo } from "../ui/Logo";
import { useLang, useT } from "../i18n";

const RELEASES_URL = "https://github.com/pier-dock/pier-dock/releases";

function Text({ parts }: { parts: Inline[] }) {
  return (
    <>
      {parts.map((p, i) => (p.bold ? <strong key={i}>{p.text}</strong> : p.code ? <code key={i}>{p.text}</code> : p.text))}
    </>
  );
}

function BlockView({ block }: { block: Block }) {
  switch (block.type) {
    case "heading":
      return (
        <h3 className="whats-new-heading">
          <Text parts={block.text} />
        </h3>
      );
    case "paragraph":
      return (
        <p>
          <Text parts={block.text} />
        </p>
      );
    case "note":
      return (
        <p className="whats-new-note">
          <Text parts={block.text} />
        </p>
      );
    case "list":
      return (
        <ul>
          {block.items.map((item, i) => (
            <li key={i}>
              <Text parts={item} />
            </li>
          ))}
        </ul>
      );
  }
}

/** Novidades das versões depois de `since` até a atual; sem `since`, só a atual. */
export function WhatsNew({ since }: { since: string | null }) {
  const t = useT().whatsNew;
  const lang = useLang((s) => s.lang);
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    getVersion().then(setVersion).catch(() => setVersion(""));
  }, []);

  const entries = useMemo(
    () => (version === null ? [] : entriesBetween(parseChangelog(lang === "pt" ? changelogPt : changelogEn), since, version)),
    [lang, since, version],
  );
  const close = () => void getCurrentWindow().close();

  return (
    <div className="settings whats-new" data-glass>
      <header className="whats-new-header" data-tauri-drag-region>
        <span className="whats-new-logo" data-tauri-drag-region>
          <Logo size={26} />
        </span>
        <h1 className="whats-new-title" data-tauri-drag-region>
          {t.title}
        </h1>
        {version && (
          <span className="whats-new-sub" data-tauri-drag-region>
            {t.version(version)}
          </span>
        )}
      </header>
      <main className="whats-new-body">
        {version !== null && !entries.length && <p className="whats-new-empty">{t.empty}</p>}
        {entries.map((entry) => (
          <section key={entry.version} className="whats-new-version">
            {entries.length > 1 && <h2 className="eyebrow whats-new-eyebrow">{t.version(entry.version)}</h2>}
            {entry.blocks.map((block, i) => (
              <BlockView key={i} block={block} />
            ))}
          </section>
        ))}
      </main>
      <ActionFooter
        keyboard
        context={null}
        actions={[
          { label: t.close, keys: "↵", onClick: close },
          { label: t.allReleases, onClick: () => openUrl(RELEASES_URL).catch(console.error) },
        ]}
      />
    </div>
  );
}
