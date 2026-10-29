import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement> & { size?: number };

/** Dock artwork from pier-dock.vercel.app. Keep its geometry independent of Lucide releases. */
function Outline({ size = 17, children, ...props }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden {...props}>{children}</svg>;
}

export function Play({ size = 15, ...props }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden {...props}><path d="M5 2.5l9 5.5-9 5.5z" /></svg>;
}

export function Pause({ size = 15, ...props }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden {...props}><path d="M4 2.5h3v11H4z M9 2.5h3v11H9z" /></svg>;
}

export function SkipForward({ size = 15, ...props }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden {...props}><path d="M16.5 5H19v14h-2.5z M4 5v14l11-7z" /></svg>;
}

export function Battery({ percent, charging = false, ...props }: IconProps & { percent: number; charging?: boolean }) {
  const level = Number.isFinite(percent) ? Math.min(100, Math.max(0, percent)) : 0;
  return (
    <Outline width={20} height={17} viewBox="0 0 26 24" {...props}>
      <rect x={2} y={7} width={19} height={10} rx={2.5} />
      <line x1={24} x2={24} y1={11} y2={13} />
      <rect x={4.5} y={9.5} width={14 * level / 100} height={5} rx={1} fill="currentColor" stroke="none" opacity={charging ? 0.3 : 1} />
      {charging && <path d="m13 8-4 5h4l-1 3 5-6h-4z" fill="currentColor" stroke="none" />}
    </Outline>
  );
}

function Volume({ waves, muted = false, ...props }: IconProps & { waves: number; muted?: boolean }) {
  return <Outline {...props}>
    <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5" />
    {waves > 0 && <path d="M15.54 8.46a5 5 0 0 1 0 7.07" />}
    {waves > 1 && <path d="M19.07 4.93a10 10 0 0 1 0 14.14" />}
    {muted && <path d="m17 9 6 6m0-6-6 6" />}
  </Outline>;
}
export function Volume2(props: IconProps) { return <Volume waves={2} {...props} />; }
export function Volume1(props: IconProps) { return <Volume waves={1} {...props} />; }
export function VolumeX(props: IconProps) { return <Volume waves={0} muted {...props} />; }

export function Wifi(props: IconProps) {
  return <Outline {...props}><path d="M12 20h.01" /><path d="M2 8.82a15 15 0 0 1 20 0" /><path d="M5 12.86a10 10 0 0 1 14 0" /><path d="M8.5 16.43a5 5 0 0 1 7 0" /></Outline>;
}

export function Sun(props: IconProps) {
  return <Outline {...props}><circle cx={12} cy={12} r={4} /><path d="M12 2v2 M12 20v2 M4.9 4.9l1.4 1.4 M17.7 17.7l1.4 1.4 M2 12h2 M20 12h2 M6.3 17.7l-1.4 1.4 M19.1 4.9l-1.4 1.4" /></Outline>;
}

export function Sparkles(props: IconProps) {
  return <Outline {...props}><path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z" /><path d="M20 3v4M22 5h-4M4 17v2M5 18H3" /></Outline>;
}

export function Video(props: IconProps) {
  return <Outline {...props}><path d="m16 13 5.2 3.5a.5.5 0 0 0 .8-.4V7.9a.5.5 0 0 0-.8-.4L16 11" /><rect x={2} y={6} width={14} height={12} rx={2} /></Outline>;
}

export function GitPullRequest({ size = 18, ...props }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden {...props}><path d="M1.5 3.25a2.25 2.25 0 1 1 3 2.122v5.256a2.251 2.251 0 1 1-1.5 0V5.372A2.25 2.25 0 0 1 1.5 3.25Zm5.677-.177L9.573.677A.25.25 0 0 1 10 .854V2.5h1A2.5 2.5 0 0 1 13.5 5v5.628a2.251 2.251 0 1 1-1.5 0V5a1 1 0 0 0-1-1h-1v1.646a.25.25 0 0 1-.427.177L7.177 3.427a.25.25 0 0 1 0-.354ZM3.75 2.5a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Zm0 9.5a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Zm8.25.75a.75.75 0 1 0 1.5 0 .75.75 0 0 0-1.5 0Z" /></svg>;
}

export function GitCommitHorizontal({ size = 18, ...props }: IconProps) {
  return <Outline size={size} {...props}><circle cx={12} cy={12} r={3} /><path d="M3 12h6M15 12h6" /></Outline>;
}
