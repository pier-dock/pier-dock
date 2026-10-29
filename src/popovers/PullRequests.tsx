import { useState } from "react";
import { CheckCircle2, CircleDashed, XCircle } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ipc, type GithubData, type PullRequest } from "../lib/ipc";
import { useGithub } from "../hooks/useGithub";
import { timeAgo } from "../lib/format";
import { keys } from "../lib/platform";
import { ActionFooter } from "../ui/ActionFooter";
import { GitPullRequest } from "../ui/DockIcons";
import { Segmented } from "../ui/Segmented";
import { dict, useT } from "../i18n";

export const refreshGithub = () => ipc.githubFetch().then(() => {});

function ChecksIcon({ state }: { state?: PullRequest["checks"] }) {
  const t = useT().github;
  if (state === "SUCCESS") return <CheckCircle2 size={16} className="ok" aria-label={t.checksPassed} />;
  if (state === "FAILURE" || state === "ERROR") return <XCircle size={16} className="fail" aria-label={t.checksFailed} />;
  if (state === "PENDING" || state === "EXPECTED") return <CircleDashed size={16} className="pending" aria-label={t.checksRunning} />;
  return null;
}

function reviewLabel(decision: string): string | undefined {
  const t = dict().github;
  return ({ APPROVED: t.approved, CHANGES_REQUESTED: t.changesRequested, REVIEW_REQUIRED: t.reviewRequired } as Record<string, string>)[decision];
}

export function EmptyState({ title, sub }: { title: string; sub: string }) {
  return (
    <div className="popover-empty">
      <span className="empty-icon">
        <GitPullRequest size={22} />
      </span>
      <span className="empty-title">{title}</span>
      <span className="empty-sub">{sub}</span>
    </div>
  );
}

export function GithubError({ data }: { data: GithubData | null }) {
  const t = useT();
  if (!data) return <div className="popover-loading">{t.common.loading}</div>;
  if (!data.error) return null;
  return (
    <div className="error-box" role="alert">
      {data.error}
    </div>
  );
}

function PrList({ prs, showAuthor }: { prs: PullRequest[]; showAuthor: boolean }) {
  const t = useT();
  return (
    <ul className="list">
      {prs.map((pr) => (
        <li key={pr.url}>
          <button className="list-row" onClick={() => openUrl(pr.url)}>
            {!showAuthor ? (
              <ChecksIcon state={pr.checks} />
            ) : pr.authorAvatar ? (
              <img className="avatar" src={pr.authorAvatar} alt="" />
            ) : (
              <span className="avatar avatar-fallback" aria-hidden>
                {pr.author.slice(0, 1).toUpperCase()}
              </span>
            )}
            <span className="list-main">
              <span className="list-title">
                {pr.isDraft && <span className="pill">{t.github.draft}</span>}
                {pr.title}
              </span>
              <span className="list-sub">
                {pr.repo}#{pr.number} · {showAuthor ? `${pr.author} · ` : ""}
                {timeAgo(pr.updatedAt)}
                {pr.reviewDecision && reviewLabel(pr.reviewDecision) ? ` · ${reviewLabel(pr.reviewDecision)}` : ""}
              </span>
            </span>
            {showAuthor && <ChecksIcon state={pr.checks} />}
          </button>
        </li>
      ))}
    </ul>
  );
}

type Tab = "mine" | "review";

/** Seus PRs e os que esperam o seu review, no mesmo popover; o widget clicado escolhe a aba. */
export function PullRequests({ initial }: { initial: Tab }) {
  const data = useGithub();
  const [tab, setTab] = useState<Tab>(initial);
  const { github: t, common } = useT();
  const prs = (tab === "mine" ? data?.myPrs : data?.reviewRequests) ?? [];
  const link = tab === "mine" ? "https://github.com/pulls" : "https://github.com/pulls/review-requested";
  const updated = data ? t.updatedSentence(timeAgo(data.fetchedAt)) : "";

  return (
    <>
      <header className="popover-header">
        <span className="popover-heading">{t.pullRequests}</span>
        <Segmented
          label={t.list}
          value={tab}
          onChange={setTab}
          options={[
            { value: "mine", label: t.openCount(data?.myPrCount ?? 0) },
            { value: "review", label: t.reviewsCount(data?.reviewCount ?? 0) },
          ]}
        />
      </header>
      <div className="popover-body">
        <GithubError data={data} />
        {data &&
          !data.error &&
          (prs.length ? (
            <PrList prs={prs} showAuthor={tab === "review"} />
          ) : (
            <EmptyState
              title={tab === "mine" ? t.noOpenPrs : t.noReviews}
              sub={t.allClear(updated)}
            />
          ))}
      </div>
      <ActionFooter
        context={
          <span className="footer-source">
            <GitPullRequest size={14} aria-hidden />
            GitHub
          </span>
        }
        actions={[
          { label: t.openOnGithub, keys: "↵", onClick: () => openUrl(link) },
          { label: common.refresh, keys: `${keys.command} R`, onClick: refreshGithub },
        ]}
      />
    </>
  );
}
