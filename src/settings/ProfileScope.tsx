import type { Config } from "../lib/ipc";
import { useT } from "../i18n";
import { Segmented } from "../ui/Segmented";

/** Qual perfil as abas de conteúdo estão editando. Com um perfil só não há o que escolher. */
export function ProfileScope({ config, editingId, onChange }: { config: Config; editingId: string; onChange: (id: string) => void }) {
  const t = useT().profiles;
  if (config.dockProfiles.length < 2) return null;
  return (
    <div className="profile-scope">
      <span>{t.editing}</span>
      <Segmented label={t.editing} value={editingId} onChange={onChange} options={config.dockProfiles.map((p) => ({ value: p.id, label: p.name }))} />
      {editingId !== config.activeProfile && <span className="profile-scope-hint">{t.notActive}</span>}
    </div>
  );
}
