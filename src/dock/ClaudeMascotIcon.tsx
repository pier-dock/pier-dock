interface Props {
  size?: number;
}

/** Mascote do Claude ("clawd"), a partir da arte pixel-art em `clawd.svg`. Preenchido com
 * `currentColor` (sem fundo) para herdar a cor de status já aplicada em `.claude-sprite`
 * (ver `widgets.css`). Substitui o ícone genérico `Bot` do lucide-react. */
export function ClaudeMascotIcon({ size = 18 }: Props) {
  return (
    <svg width={size} height={size} viewBox="0 0 400 400" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden>
      <path
        d="M 12,152 L 12,214 L 13,215 L 14,214 L 71,214 L 72,215 L 72,343 L 106,343 L 107,342 L 107,278 L 106,277 L 107,276 L 135,276 L 136,277 L 136,343 L 169,343 L 169,277 L 170,276 L 230,276 L 231,277 L 231,343 L 263,343 L 263,277 L 264,276 L 293,276 L 294,277 L 294,343 L 330,343 L 330,215 L 331,214 L 389,214 L 389,152 L 331,152 L 330,151 L 330,91 L 72,91 L 72,151 L 71,152 Z M 260,119 L 261,118 L 294,118 L 295,119 L 295,151 L 294,152 L 261,152 L 260,151 Z M 106,119 L 107,118 L 140,118 L 141,119 L 141,151 L 140,152 L 107,152 L 106,151 Z"
        fill="currentColor"
        fillRule="evenodd"
      />
    </svg>
  );
}
