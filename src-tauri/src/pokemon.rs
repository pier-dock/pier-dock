//! Sprites de time (menu de party) dos Pokémon para o mascote da dock. A PokeAPI só tem esse ícone
//! até a geração VII (#1 a #809); o download é feito aqui e cacheado em disco, como os ícones de apps.

use std::time::Duration;

use base64::Engine;
use serde::Serialize;
use serde_json::Value;

use crate::util::{data_dir, ErrStr};

pub const MAX_ID: u32 = 809;
const SPRITE_URL: &str = "https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/versions/generation-vii/icons";
const LIST_URL: &str = "https://pokeapi.co/api/v2/pokemon?limit=809";

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct PokemonEntry {
    pub id: u32,
    pub name: String,
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().user_agent("pier").timeout(Duration::from_secs(10)).build().err_str()
}

fn cache_dir() -> std::path::PathBuf {
    let dir = data_dir().join("pokemon");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn parse_list(json: &Value) -> Vec<PokemonEntry> {
    json.get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let name = p.get("name")?.as_str()?.to_string();
            let id: u32 = p.get("url")?.as_str()?.trim_end_matches('/').rsplit('/').next()?.parse().ok()?;
            (1..=MAX_ID).contains(&id).then_some(PokemonEntry { id, name })
        })
        .collect()
}

#[tauri::command]
pub async fn pokemon_list() -> Result<Vec<PokemonEntry>, String> {
    let path = cache_dir().join("list.json");
    if let Some(cached) = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) {
        let list = parse_list(&cached);
        if !list.is_empty() {
            return Ok(list);
        }
    }
    let json: Value = client()?.get(LIST_URL).send().await.err_str()?.error_for_status().err_str()?.json().await.err_str()?;
    let _ = std::fs::write(&path, json.to_string());
    Ok(parse_list(&json))
}

#[tauri::command]
pub async fn pokemon_sprite(id: u32) -> Result<String, String> {
    if !(1..=MAX_ID).contains(&id) {
        return Err(format!("no party sprite for #{id}"));
    }
    let path = cache_dir().join(format!("{id}.png"));
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => {
            let res = client()?.get(format!("{SPRITE_URL}/{id}.png")).send().await.err_str()?.error_for_status().err_str()?;
            let b = res.bytes().await.err_str()?.to_vec();
            let _ = std::fs::write(&path, &b);
            b
        }
    };
    Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_keeps_only_ids_with_party_sprite() {
        let json = serde_json::json!({ "results": [
            { "name": "bulbasaur", "url": "https://pokeapi.co/api/v2/pokemon/1/" },
            { "name": "melmetal", "url": "https://pokeapi.co/api/v2/pokemon/809/" },
            { "name": "grookey", "url": "https://pokeapi.co/api/v2/pokemon/810/" },
            { "name": "broken", "url": "nope" },
        ]});
        let list = parse_list(&json);
        assert_eq!(list.iter().map(|p| p.id).collect::<Vec<_>>(), vec![1, 809]);
        assert_eq!(list[0].name, "bulbasaur");
    }
}
