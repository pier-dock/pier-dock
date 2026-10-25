/** Repositório e número de uma issue a partir da URL que o GitHub devolve; `null` se não for uma issue. */
export function issueRef(url: string): { repo: string; number: number } | null {
  const m = /^https:\/\/github\.com\/([^/]+\/[^/]+)\/issues\/(\d+)\/?(?:[?#].*)?$/.exec(url);
  return m ? { repo: m[1], number: Number(m[2]) } : null;
}
