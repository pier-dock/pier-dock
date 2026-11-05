import { openUrl } from "@tauri-apps/plugin-opener";
import { useGithub } from "../hooks/useGithub";
import { timeAgo } from "../lib/format";
import { keys } from "../lib/platform";
import { ActionFooter } from "../ui/ActionFooter";
import { contributionLevel, contributionsThisYear } from "../widgets/GithubWidgets";
import { GithubError, refreshGithub } from "./PullRequests";
import { locale, useT } from "../i18n";

type Cell = { date: string; level: number } | "future" | null;

const iso = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;

/**
 * O ano civil em colunas de semana (domingo a sábado, como no GitHub): dias de outros anos ficam
 * vazios e os que ainda não chegaram, apagados. Devolve também a coluna em que cada mês começa.
 */
function yearCells(counts: Map<string, number>, max: number, year: number): { cells: Cell[]; monthCols: number[] } {
  const today = iso(new Date());
  const day = new Date(year, 0, 1);
  day.setDate(day.getDate() - day.getDay());
  const end = new Date(year, 11, 31);
  end.setDate(end.getDate() + (6 - end.getDay()));
  const cells: Cell[] = [];
  const monthCols: number[] = [];
  for (let i = 0; day <= end; i++, day.setDate(day.getDate() + 1)) {
    const date = iso(day);
    if (day.getFullYear() !== year) cells.push(null);
    else if (date > today) cells.push("future");
    else cells.push({ date, level: contributionLevel(counts.get(date) ?? 0, max) });
    if (day.getFullYear() === year && day.getDate() === 1) monthCols.push(Math.floor(i / 7));
  }
  return { cells, monthCols };
}

export function Contributions() {
  const data = useGithub();
  const { github: t, common } = useT();
  const all = data?.weeks ?? [];
  const counts = new Map(all.flat().map((d) => [d.date, d.count]));
  const max = Math.max(1, ...counts.values());
  const year = new Date().getFullYear();
  const { cells, monthCols } = yearCells(counts, max, year);
  const columns = { gridTemplateColumns: `repeat(${Math.ceil(cells.length / 7)}, 1fr)` };
  const profile = data?.login ? `https://github.com/${data.login}` : "https://github.com";

  return (
    <>
      <header className="contrib-header">
        <span className="contrib-heading">
          <span className="eyebrow">{t.contributionsIn(year)}</span>
          <span className="contrib-big">{contributionsThisYear(all).toLocaleString(locale())}</span>
        </span>
        <span className="heat-legend" aria-hidden>
          {t.less}
          {[0, 1, 2, 3, 4].map((l) => (
            <span key={l} className={`heat l${l}`} />
          ))}
          {t.more}
        </span>
      </header>
      <GithubError data={data} />
      <div className="year-map">
        <div className="year-grid" style={columns} role="img" aria-label={t.contributionsMap(year)}>
          {cells.map((cell, i) =>
            cell === null ? (
              <span key={i} className="heat blank" />
            ) : cell === "future" ? (
              <span key={i} className="heat" />
            ) : (
              <span
                key={i}
                className={`heat l${cell.level}`}
                title={t.contributionsOn(counts.get(cell.date) ?? 0, new Date(cell.date + "T12:00").toLocaleDateString(locale()))}
              />
            ),
          )}
        </div>
        <div className="year-months" style={columns} aria-hidden>
          {monthCols.map((col, m) => (
            <span key={m} style={{ gridColumn: `${col + 1} / span 4` }}>
              {t.months[m]}
            </span>
          ))}
        </div>
      </div>
      <ActionFooter
        context={data ? common.updatedAgo(timeAgo(data.fetchedAt)) : ""}
        actions={[
          { label: common.viewProfile, keys: "↵", onClick: () => openUrl(profile) },
          { label: common.refresh, loadingLabel: common.refreshing, keys: `${keys.command} R`, onClick: refreshGithub },
        ]}
      />
    </>
  );
}
