//! Widget de mercado, pela brapi (https://brapi.dev). Ações e FIIs vêm da cotação em lote; as
//! criptomoedas, do endpoint de cripto. Sem chave, a brapi só responde os tickers de teste, então
//! os outros ficam de fora sem derrubar a lista. A chave opcional fica no cofre de senhas do
//! sistema e nunca chega ao WebView, como o token do GitHub.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

use crate::config::{ConfigState, FREE_MARKET_TICKERS};
use crate::util::ErrStr;

const KEYRING_SERVICE: &str = "pier";
const KEYRING_USER: &str = "brapi";

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketAsset {
    pub symbol: String,
    pub name: String,
    /// "ticker" (ação ou FII) ou "crypto".
    pub kind: String,
    pub price: f64,
    pub change: f64,
    pub change_percent: f64,
    pub currency: String,
    pub logo_url: Option<String>,
    pub day_low: Option<f64>,
    pub day_high: Option<f64>,
    pub week_low: Option<f64>,
    pub week_high: Option<f64>,
    pub volume: Option<f64>,
    pub market_cap: Option<f64>,
    /// Horário da cotação, em ISO 8601.
    pub updated_at: String,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketData {
    pub assets: Vec<MarketAsset>,
    /// Símbolos ignorados por falta de chave (fora da lista gratuita da brapi).
    pub unavailable: Vec<String>,
    /// Símbolos pedidos que a brapi não devolveu.
    pub missing: Vec<String>,
    pub fetched_at: f64,
    pub error: Option<String>,
}

static CACHE: Mutex<Option<MarketData>> = Mutex::new(None);

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().user_agent("pier").timeout(Duration::from_secs(10)).build().err_str()
}

fn now_ms() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0)
}

fn stored_token() -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()?.get_password().ok().filter(|t| !t.is_empty())
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(|n| n.as_f64().or_else(|| n.as_i64().map(|i| i as f64)))
}

/// Volume e market cap zero não são um dado: a brapi devolve 0 quando não tem o valor.
fn positive(value: &Value, key: &str) -> Option<f64> {
    number(value, key).filter(|n| *n > 0.0)
}

fn text(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

pub fn parse_quotes(json: &Value) -> Vec<MarketAsset> {
    json.get("results")
        .and_then(Value::as_array)
        .map(|results| {
            results
                .iter()
                .filter_map(|item| {
                    let symbol = text(item, "symbol");
                    if symbol.is_empty() {
                        return None;
                    }
                    let long_name = text(item, "longName");
                    let short_name = text(item, "shortName");
                    let currency = text(item, "currency");
                    Some(MarketAsset {
                        name: if long_name.is_empty() { short_name } else { long_name },
                        kind: "ticker".into(),
                        price: number(item, "regularMarketPrice").unwrap_or(0.0),
                        change: number(item, "regularMarketChange").unwrap_or(0.0),
                        change_percent: number(item, "regularMarketChangePercent").unwrap_or(0.0),
                        currency: if currency.is_empty() { "BRL".into() } else { currency },
                        logo_url: Some(text(item, "logourl")).filter(|url| !url.is_empty()),
                        day_low: number(item, "regularMarketDayLow"),
                        day_high: number(item, "regularMarketDayHigh"),
                        week_low: number(item, "fiftyTwoWeekLow"),
                        week_high: number(item, "fiftyTwoWeekHigh"),
                        volume: positive(item, "regularMarketVolume"),
                        market_cap: positive(item, "marketCap"),
                        updated_at: text(item, "regularMarketTime"),
                        symbol,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_crypto(json: &Value) -> Vec<MarketAsset> {
    json.get("coins")
        .and_then(Value::as_array)
        .map(|coins| {
            coins
                .iter()
                .filter_map(|item| {
                    let symbol = text(item, "coin");
                    if symbol.is_empty() {
                        return None;
                    }
                    let currency = text(item, "currency");
                    Some(MarketAsset {
                        name: text(item, "coinName"),
                        kind: "crypto".into(),
                        price: number(item, "regularMarketPrice").unwrap_or(0.0),
                        change: number(item, "regularMarketChange").unwrap_or(0.0),
                        change_percent: number(item, "regularMarketChangePercent").unwrap_or(0.0),
                        currency: if currency.is_empty() { "BRL".into() } else { currency },
                        logo_url: Some(text(item, "coinImageUrl")).filter(|url| !url.is_empty()),
                        day_low: number(item, "regularMarketDayLow"),
                        day_high: number(item, "regularMarketDayHigh"),
                        // Cripto não tem faixa de 52 semanas na resposta.
                        week_low: None,
                        week_high: None,
                        volume: positive(item, "regularMarketVolume"),
                        market_cap: positive(item, "marketCap"),
                        updated_at: text(item, "regularMarketTime"),
                        symbol,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A ordem configurada manda na exibição; o que a brapi não devolveu vira `missing`.
fn arrange(mut assets: Vec<MarketAsset>, requested: &[String]) -> (Vec<MarketAsset>, Vec<String>) {
    let rank = |symbol: &str| requested.iter().position(|s| s.as_str() == symbol).unwrap_or(usize::MAX);
    assets.sort_by_key(|asset| rank(&asset.symbol));
    let missing = requested.iter().filter(|symbol| !assets.iter().any(|asset| &asset.symbol == *symbol)).cloned().collect();
    (assets, missing)
}

fn split_free(tickers: Vec<String>) -> (Vec<String>, Vec<String>) {
    tickers.into_iter().fold((Vec::new(), Vec::new()), |(mut free, mut paid), symbol| {
        if FREE_MARKET_TICKERS.contains(&symbol.as_str()) {
            free.push(symbol);
        } else {
            paid.push(symbol);
        }
        (free, paid)
    })
}

struct SendError {
    code: Option<String>,
    message: String,
}

impl From<String> for SendError {
    fn from(message: String) -> Self {
        Self { code: None, message }
    }
}

/// A brapi devolve `{"error":true,"code":"..."}` nos erros de plano; o código diz o que fazer.
fn error_code(body: &str) -> Option<String> {
    serde_json::from_str::<Value>(body).ok()?.get("code")?.as_str().map(str::to_string)
}

fn status_message(status: u16, code: Option<&str>) -> String {
    match (code, status) {
        (Some("FEATURE_NOT_AVAILABLE"), _) => crate::i18n::tr(
            "Criptomoedas exigem o plano Startup da brapi.",
            "Crypto requires the brapi Startup plan.",
        )
        .to_string(),
        (Some("QUOTES_PER_REQUEST_EXCEEDED"), _) => crate::i18n::tr(
            "O plano da brapi limita os ativos por requisição.",
            "Your brapi plan limits assets per request.",
        )
        .to_string(),
        (_, 401) => crate::i18n::tr(
            "A brapi pediu uma chave. Confira em Configurações > Widgets > Mercado.",
            "brapi asked for an API key. Check it in Settings > Widgets > Market.",
        )
        .to_string(),
        (_, 403) => crate::i18n::tr("O plano da brapi não inclui este dado.", "Your brapi plan doesn't include this data.").to_string(),
        (_, 404) => crate::i18n::tr("Ativo não encontrado na brapi.", "Asset not found on brapi.").to_string(),
        (_, 429) => crate::i18n::tr(
            "Limite da brapi atingido. Tente de novo em alguns minutos.",
            "brapi rate limit reached. Try again in a few minutes.",
        )
        .to_string(),
        _ => format!("brapi: HTTP {status}"),
    }
}

async fn send(request: reqwest::RequestBuilder) -> Result<Value, SendError> {
    let response = request.send().await.err_str()?;
    let status = response.status();
    if status.is_success() {
        return Ok(response.json().await.err_str()?);
    }
    let code = error_code(&response.text().await.unwrap_or_default());
    Err(SendError { message: status_message(status.as_u16(), code.as_deref()), code })
}

async fn fetch_one_quote(client: &reqwest::Client, tickers: &[String], token: Option<&str>) -> Result<Vec<MarketAsset>, SendError> {
    let mut request = client.get(format!("https://brapi.dev/api/quote/{}", tickers.join(",")));
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    send(request).await.map(|json| parse_quotes(&json))
}

/// Em lote; se o plano só aceita um ativo por requisição, repete um a um e mantém o que deu certo.
async fn fetch_quotes(client: &reqwest::Client, tickers: &[String], token: Option<&str>) -> Result<Vec<MarketAsset>, String> {
    if tickers.is_empty() {
        return Ok(Vec::new());
    }
    match fetch_one_quote(client, tickers, token).await {
        Ok(quotes) => Ok(quotes),
        Err(error) if tickers.len() > 1 && error.code.as_deref() == Some("QUOTES_PER_REQUEST_EXCEEDED") => {
            let mut quotes = Vec::new();
            let mut first_error = None;
            for ticker in tickers {
                match fetch_one_quote(client, std::slice::from_ref(ticker), token).await {
                    Ok(found) => quotes.extend(found),
                    Err(error) => {
                        first_error.get_or_insert(error.message);
                    }
                }
            }
            match first_error {
                Some(message) if quotes.is_empty() => Err(message),
                _ => Ok(quotes),
            }
        }
        Err(error) => Err(error.message),
    }
}

async fn fetch_crypto(client: &reqwest::Client, coins: &[String], token: Option<&str>) -> Result<Vec<MarketAsset>, String> {
    if coins.is_empty() {
        return Ok(Vec::new());
    }
    let mut request = client
        .get("https://brapi.dev/api/v2/crypto")
        .query(&[("coin", coins.join(",")), ("currency", "BRL".to_string())]);
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    send(request).await.map(|json| parse_crypto(&json)).map_err(|error| error.message)
}

async fn fetch(tickers: &[String], coins: &[String], token: Option<&str>) -> (Vec<MarketAsset>, Vec<String>, Option<String>) {
    let client = match client() {
        Ok(client) => client,
        Err(error) => return (Vec::new(), Vec::new(), Some(error)),
    };
    let mut errors = Vec::new();
    let mut assets = Vec::new();
    match fetch_quotes(&client, tickers, token).await {
        Ok(quotes) => assets.extend(quotes),
        Err(error) => errors.push(error),
    }
    match fetch_crypto(&client, coins, token).await {
        Ok(crypto) => assets.extend(crypto),
        Err(error) => errors.push(error),
    }
    let requested: Vec<String> = tickers.iter().chain(coins).cloned().collect();
    let (assets, missing) = arrange(assets, &requested);
    (assets, missing, (!errors.is_empty()).then(|| errors.join(" ")))
}

#[tauri::command]
pub async fn market_fetch(app: AppHandle, state: State<'_, ConfigState>) -> Result<MarketData, String> {
    let (tickers, coins) = {
        let config = state.0.lock().unwrap();
        (config.market_tickers.clone(), config.market_coins.clone())
    };
    let token = tauri::async_runtime::spawn_blocking(stored_token).await.err_str()?.filter(|t| !t.is_empty());
    let (tickers, mut unavailable) = if token.is_some() { (tickers, Vec::new()) } else { split_free(tickers) };
    let coins = if token.is_some() {
        coins
    } else {
        unavailable.extend(coins);
        Vec::new()
    };
    let (mut assets, missing, error) = fetch(&tickers, &coins, token.as_deref()).await;
    // Se a requisição falhou, a última cotação dos mesmos ativos continua na tela.
    if assets.is_empty() {
        let requested: Vec<String> = tickers.iter().chain(coins.iter()).cloned().collect();
        assets = CACHE
            .lock()
            .unwrap()
            .clone()
            .map(|data| data.assets)
            .unwrap_or_default()
            .into_iter()
            .filter(|asset| requested.contains(&asset.symbol))
            .collect();
    }
    let data = MarketData { assets, unavailable, missing, fetched_at: now_ms(), error };
    *CACHE.lock().unwrap() = Some(data.clone());
    let _ = app.emit("market-updated", &data);
    Ok(data)
}

#[tauri::command]
pub fn market_cached() -> Option<MarketData> {
    CACHE.lock().unwrap().clone()
}

#[tauri::command]
pub async fn market_token_status() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(|| stored_token().is_some()).await.err_str()
}

/// Salva a chave no cofre de senhas do sistema; `None` ou vazio apaga e volta ao modo sem chave.
#[tauri::command]
pub fn market_set_token(token: Option<String>) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).err_str()?;
    let result = match token.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) {
        Some(token) => entry.set_password(&token).err_str(),
        None => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        },
    };
    // A próxima leitura parte do zero: os ativos disponíveis mudaram junto com a chave.
    if result.is_ok() {
        *CACHE.lock().unwrap() = None;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_stock_and_fii_quotes() {
        let json = json!({ "results": [
            {
                "symbol": "PETR4", "shortName": "PETR4", "longName": "Petroleo Brasileiro SA Pfd",
                "currency": "BRL", "regularMarketPrice": 52.71, "regularMarketDayHigh": 53.66,
                "regularMarketDayLow": 52.36, "regularMarketChange": 0.51,
                "regularMarketChangePercent": 0.98, "regularMarketTime": "2026-11-01T22:15:46.000Z",
                "marketCap": 726564863632.0, "regularMarketVolume": 61743200,
                "fiftyTwoWeekRange": "29.53 - 59.79", "fiftyTwoWeekLow": 29.53,
                "fiftyTwoWeekHigh": 59.79, "logourl": "https://icons.brapi.dev/icons/PETR4.svg"
            },
            {
                "symbol": "MXRF11", "shortName": "MXRF11", "currency": "BRL",
                "regularMarketPrice": 10.1, "regularMarketChange": -0.02,
                "regularMarketChangePercent": -0.2, "regularMarketTime": "2026-11-01T22:15:46.000Z",
                "marketCap": 0, "regularMarketVolume": 0
            }
        ]});
        let assets = parse_quotes(&json);
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0].name, "Petroleo Brasileiro SA Pfd");
        assert_eq!(assets[0].week_high, Some(59.79));
        assert_eq!(assets[0].logo_url.as_deref(), Some("https://icons.brapi.dev/icons/PETR4.svg"));
        // Sem market cap nem volume, o zero não vira dado.
        assert_eq!(assets[1].market_cap, None);
        assert_eq!(assets[1].volume, None);
        assert_eq!(assets[1].kind, "ticker");
    }

    #[test]
    fn parses_crypto_coins() {
        let json = json!({ "coins": [
            {
                "currency": "BRL", "coin": "BTC", "coinName": "Bitcoin",
                "coinImageUrl": "https://icons.brapi.dev/icons/BTC.svg",
                "regularMarketPrice": 380000.0, "regularMarketChange": -2000.0,
                "regularMarketChangePercent": -0.52, "regularMarketDayLow": 375000.0,
                "regularMarketDayHigh": 385000.0, "regularMarketVolume": 1200.0,
                "marketCap": 0, "regularMarketTime": "2026-11-01T22:15:46.000Z"
            }
        ]});
        let assets = parse_crypto(&json);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].symbol, "BTC");
        assert_eq!(assets[0].name, "Bitcoin");
        assert_eq!(assets[0].kind, "crypto");
        assert_eq!(assets[0].week_low, None);
        assert_eq!(assets[0].market_cap, None);
    }

    #[test]
    fn arranges_by_the_configured_order_and_reports_missing() {
        let asset = |symbol: &str| MarketAsset { symbol: symbol.into(), ..Default::default() };
        let (assets, missing) = arrange(
            vec![asset("VALE3"), asset("PETR4"), asset("BTC")],
            &["PETR4".to_string(), "BTC".to_string(), "VALE3".to_string(), "MXRF11".to_string()],
        );
        assert_eq!(assets.iter().map(|a| a.symbol.as_str()).collect::<Vec<_>>(), vec!["PETR4", "BTC", "VALE3"]);
        assert_eq!(missing, vec!["MXRF11"]);
    }

    #[test]
    fn reads_the_brapi_error_code_and_explains_plan_limits() {
        let body = r#"{"error":true,"code":"FEATURE_NOT_AVAILABLE","message":"x"}"#;
        assert_eq!(error_code(body).as_deref(), Some("FEATURE_NOT_AVAILABLE"));
        assert_eq!(error_code("not json"), None);
        assert!(status_message(403, Some("FEATURE_NOT_AVAILABLE")).contains("Startup"));
        assert!(!status_message(403, None).contains("Startup"));
    }

    #[test]
    fn without_a_key_only_the_free_tickers_are_requested() {
        let (free, paid) = split_free(["PETR4".into(), "MXRF11".into(), "vale3".into(), "BTC".into()].to_vec());
        assert_eq!(free, vec!["PETR4"]);
        // A normalização da config sobe para maiúsculas antes de chegar aqui.
        assert_eq!(paid, vec!["MXRF11", "vale3", "BTC"]);
    }
}

#[cfg(test)]
mod live_tests {
    #[test]
    #[ignore = "usa a rede e a chave em BRAPI_TEST_TOKEN"]
    fn falls_back_to_one_asset_per_request() {
        let Ok(token) = std::env::var("BRAPI_TEST_TOKEN") else { return };
        let tickers: Vec<String> = ["PETR4", "BBAS3", "HGLG11"].map(String::from).to_vec();
        let found = tauri::async_runtime::block_on(async {
            let client = super::client().unwrap();
            super::fetch_quotes(&client, &tickers, Some(&token)).await
        })
        .unwrap();
        println!("{:?}", found.iter().map(|a| &a.symbol).collect::<Vec<_>>());
        assert_eq!(found.len(), 3);
    }

    #[test]
    #[ignore = "smoke test: chama a brapi real sem chave"]
    fn fetches_the_free_tickers() {
        let rt = tauri::async_runtime::block_on(async {
            let client = super::client().unwrap();
            super::fetch_quotes(&client, &["PETR4".to_string(), "VALE3".to_string()], None).await
        })
        .unwrap();
        println!("{} ativos: {:?}", rt.len(), rt.iter().map(|a| &a.symbol).collect::<Vec<_>>());
        assert_eq!(rt.len(), 2);
        assert!(rt[0].price > 0.0);
    }
}
