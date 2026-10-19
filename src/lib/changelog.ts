/**
 * Lê o `CHANGELOG.md` (e o `CHANGELOG.pt-BR.md`) num subconjunto de Markdown: `## <versão>` abre
 * uma versão, `###` um título, `- ` uma lista, `> ` uma nota e o resto vira parágrafo; dentro do
 * texto, só `**negrito**` e `` `código` ``. Seções `##` que não são versão (como "Unreleased") e o
 * que vem antes da primeira versão ficam de fora.
 */

export interface Inline {
  text: string;
  bold?: boolean;
  code?: boolean;
}

export type Block =
  | { type: "heading"; text: Inline[] }
  | { type: "paragraph"; text: Inline[] }
  | { type: "note"; text: Inline[] }
  | { type: "list"; items: Inline[][] };

export interface ChangelogEntry {
  version: string;
  blocks: Block[];
}

const VERSION = /^##\s+v?(\d+\.\d+\.\d+)\b/;

export function parseInline(text: string): Inline[] {
  return text
    .split(/(\*\*[^*]+\*\*|`[^`]+`)/)
    .filter(Boolean)
    .map((part) => {
      if (part.startsWith("**") && part.endsWith("**") && part.length > 4) return { text: part.slice(2, -2), bold: true };
      if (part.startsWith("`") && part.endsWith("`") && part.length > 2) return { text: part.slice(1, -1), code: true };
      return { text: part };
    });
}

export function parseChangelog(markdown: string): ChangelogEntry[] {
  const entries: ChangelogEntry[] = [];
  let entry: ChangelogEntry | null = null;
  // Linhas do parágrafo ou da nota em andamento, juntadas com espaço quando o bloco termina.
  let pending: { type: "paragraph" | "note"; lines: string[] } | null = null;
  let list: Inline[][] | null = null;

  const flush = () => {
    if (entry && pending) entry.blocks.push({ type: pending.type, text: parseInline(pending.lines.join(" ")) });
    if (entry && list) entry.blocks.push({ type: "list", items: list });
    pending = null;
    list = null;
  };

  for (const raw of markdown.split(/\r?\n/)) {
    const line = raw.trim();
    if (line.startsWith("## ")) {
      flush();
      const version = VERSION.exec(line)?.[1];
      entry = version ? { version, blocks: [] } : null;
      if (entry) entries.push(entry);
      continue;
    }
    if (!entry) continue;
    if (!line) {
      flush();
    } else if (line.startsWith("### ")) {
      flush();
      entry.blocks.push({ type: "heading", text: parseInline(line.slice(4)) });
    } else if (line.startsWith("- ")) {
      if (pending) flush();
      (list ??= []).push(parseInline(line.slice(2)));
    } else if (line.startsWith(">")) {
      if (pending?.type !== "note") flush();
      (pending ??= { type: "note", lines: [] }).lines.push(line.replace(/^>\s?/, ""));
    } else {
      if (list || pending?.type === "note") flush();
      (pending ??= { type: "paragraph", lines: [] }).lines.push(line);
    }
  }
  flush();
  return entries;
}

export function compareVersions(a: string, b: string): number {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    const diff = (pa[i] || 0) - (pb[i] || 0);
    if (diff) return diff;
  }
  return 0;
}

/**
 * As versões depois de `since` até a `current`, da mais nova para a mais antiga. Sem `since`
 * (aberta pelas configurações ou vinda de uma versão sem este recurso), só a `current`.
 */
export function entriesBetween(entries: ChangelogEntry[], since: string | null, current: string): ChangelogEntry[] {
  return entries
    .filter((e) => (since ? compareVersions(e.version, since) > 0 : e.version === current) && compareVersions(e.version, current) <= 0)
    .sort((a, b) => compareVersions(b.version, a.version));
}
