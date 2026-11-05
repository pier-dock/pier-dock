import { GitCommitHorizontal } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useGithub } from "../hooks/useGithub";
import { timeAgo } from "../lib/format";
import { keys } from "../lib/platform";
import { ActionFooter } from "../ui/ActionFooter";
import { GithubError, refreshGithub } from "./PullRequests";
import { useT } from "../i18n";

export function Commits() {
  const data = useGithub();
  const { github: t, common } = useT();
  const profile = data?.login ? `https://github.com/${data.login}` : "https://github.com";
  return (
    <>
      <header className="popover-header">
        <span className="popover-heading">{t.recentCommits}</span>
        {data?.login && <span className="popover-meta">@{data.login}</span>}
      </header>
      <div className="popover-body">
        <GithubError data={data} />
        {data && !data.error && !data.commits.length && (
          <div className="popover-empty">
            <span className="empty-icon">
              <GitCommitHorizontal size={22} strokeWidth={1.8} />
            </span>
            <span className="empty-title">{t.noCommits}</span>
          </div>
        )}
        <ul className="list">
          {data?.commits.map((c) => (
            <li key={c.sha + c.repo}>
              <button className="list-row" onClick={() => openUrl(c.url)}>
                <code className="sha">{c.sha.slice(0, 7)}</code>
                <span className="list-main">
                  <span className="list-title">{c.message}</span>
                  <span className="list-sub">
                    {c.repo} · {timeAgo(c.date)}
                  </span>
                </span>
              </button>
            </li>
          ))}
        </ul>
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
