import { GitCommitHorizontal, GitCompare, GitPullRequest } from "lucide-react";
import type { GithubData, PullRequest, WidgetSize } from "../lib/ipc";
import { Tile } from "./Tile";
import { locale, useT } from "../i18n";

export function checksState(prs: PullRequest[]): "failure" | "pending" | "success" | "none" {
  if (prs.some((p) => p.checks === "FAILURE" || p.checks === "ERROR")) return "failure";
  if (prs.some((p) => p.checks === "PENDING" || p.checks === "EXPECTED")) return "pending";
  if (prs.some((p) => p.checks === "SUCCESS")) return "success";
  return "none";
}

/** Contagem ao lado do ícone; com `highlight` vira um selo na cor de destaque. */
function Count({ value, highlight = false }: { value: number; highlight?: boolean }) {
  const text = value > 99 ? "99+" : String(value);
  return <span className={highlight && value > 0 ? "tile-badge" : "tile-count"}>{text}</span>;
}

export function GithubReviewWidget({ data }: { data: GithubData | null }) {
  const count = data?.reviewCount ?? 0;
  const t = useT().github;
  return (
    <Tile kind="github-review" label={data?.error ?? t.reviewsTile(count)}>
      <GitCompare size={18} strokeWidth={1.8} />
      <Count value={count} highlight />
      {data?.error && <span className="status-dot failure" />}
    </Tile>
  );
}

export function GithubMineWidget({ data }: { data: GithubData | null }) {
  const count = data?.myPrCount ?? 0;
  const state = checksState(data?.myPrs ?? []);
  const t = useT().github;
  return (
    <Tile kind="github-mine" label={t.mineTile(count)}>
      <GitPullRequest size={18} strokeWidth={1.8} />
      <Count value={count} />
      {state !== "none" && <span className={`status-dot ${state}`} />}
    </Tile>
  );
}

function isToday(iso: string): boolean {
  return new Date(iso).toDateString() === new Date().toDateString();
}

export function GithubCommitsWidget({ data }: { data: GithubData | null }) {
  const todayCount = data?.commits.filter((c) => isToday(c.date)).length ?? 0;
  const t = useT().github;
  return (
    <Tile kind="github-commits" label={t.commitsTile(todayCount)}>
      <GitCommitHorizontal size={18} strokeWidth={1.8} />
      <Count value={todayCount} />
    </Tile>
  );
}

export function contributionLevel(count: number, max: number): number {
  if (count === 0) return 0;
  return Math.min(4, Math.ceil((count / Math.max(1, max)) * 4));
}

export type ContributionDay = GithubData["weeks"][number][number];

/**
 * Completa as semanas para 7 dias: a primeira semana do calendário começa no meio (vazios no
 * topo) e a atual ainda não terminou (vazios embaixo). Sem isso as colunas esticadas ficariam
 * desalinhadas.
 */
export function padWeeks(weeks: ContributionDay[][]): (ContributionDay | null)[][] {
  return weeks.map((week, i) => {
    const missing = Array<null>(Math.max(0, 7 - week.length)).fill(null);
    return i === 0 && weeks.length > 1 ? [...missing, ...week] : [...week, ...missing];
  });
}

/** Contribuições do ano corrente (o calendário do GitHub cobre os últimos 12 meses). */
export function contributionsThisYear(weeks: ContributionDay[][]): number {
  const year = String(new Date().getFullYear());
  return weeks.flat().reduce((sum, d) => (d.date.startsWith(year) ? sum + d.count : sum), 0);
}

/** Mapa pequeno da dock: células de tamanho fixo, uma coluna por semana. */
function MiniHeatmap({ weeks }: { weeks: ContributionDay[][] }) {
  const max = Math.max(1, ...weeks.flat().map((d) => d.count));
  return (
    <span className="mini-heatmap" aria-hidden>
      {padWeeks(weeks)
        .flat()
        .map((day, i) => (
          <span key={day?.date ?? `blank-${i}`} className={`heat ${day ? `l${contributionLevel(day.count, max)}` : "blank"}`} />
        ))}
    </span>
  );
}

const DOCK_WEEKS = 20;

export function ContributionsWidget({ data, size }: { data: GithubData | null; size: WidgetSize }) {
  const all = data?.weeks ?? [];
  const year = contributionsThisYear(all).toLocaleString(locale());
  const t = useT().github;
  return (
    <Tile kind="github-contrib" label={t.contribTile(year)} className="contrib-tile">
      {size !== "small" && (
        <span className="contrib-summary">
          <span className="contrib-total">{year}</span>
          <span className="contrib-caption">{t.thisYear}</span>
        </span>
      )}
      {size !== "count" && <MiniHeatmap weeks={all.slice(-DOCK_WEEKS)} />}
    </Tile>
  );
}
