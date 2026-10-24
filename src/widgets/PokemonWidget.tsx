import { Tile } from "./Tile";
import { usePokemonSprite } from "../hooks/usePokemonSprite";
import { useConfig } from "../store/config";
import { useT } from "../i18n";

export function PokemonWidget() {
  const src = usePokemonSprite(useConfig((s) => s.config?.pokemonPetIds[0] ?? 25));
  const t = useT().widgets.info.pokemon;
  return (
    <Tile kind="pokemon" label={t.name} panel={false} className="pokemon-tile">
      {src && <img className="pokemon-tile-sprite" src={src} alt="" draggable={false} />}
    </Tile>
  );
}
