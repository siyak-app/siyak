//! Token estimates, `pricing.json` lookup and the `llm_usage` spending log
//! (spec §9.2, §9.4).
//!
//! Estimates are deliberately simple: a character count, with Arabic script
//! counted more densely because tokenizers split it into more pieces. They
//! only feed the "≈" next to a button; the real token counts the provider
//! reports are what is logged.
//!
//! `pricing.json` comes with the pack manifest (`pricing_url`), is cached in
//! `<data_dir>/cache/pricing.json` and refreshed once a day; without it every
//! cost is "unknown". Spending never blocks anything: over the monthly limit
//! the interface asks for confirmation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::Usage;
use crate::db::user::Settings;
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::net::{Net, Purpose};
use crate::packs::manifest::PackSource;
use crate::secrets::ProviderId;

/// Setting: the monthly spending limit in USD (JSON number), unset for none.
pub const MONTHLY_LIMIT_SETTING: &str = "ai.monthly_limit_usd";

/// How long a fetched `pricing.json` is used before it is fetched again.
const PRICING_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// Largest `pricing.json` read.
const MAX_PRICING_BYTES: usize = 1024 * 1024;

/// File name of `pricing.json`, in the cache and in a local pack directory.
const PRICING_FILE: &str = "pricing.json";

/// Tokens added per message for the provider's own formatting.
const MESSAGE_OVERHEAD: u32 = 4;

/// What a model costs per million tokens.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ModelPrice {
    /// USD per million input tokens.
    pub input_per_mtok: f64,
    /// USD per million output tokens.
    pub output_per_mtok: f64,
}

/// The prices of `pricing.json`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Pricing {
    /// Prices by `provider/model`.
    #[serde(default)]
    pub models: BTreeMap<String, ModelPrice>,
}

impl Pricing {
    /// Parses `pricing.json`; only USD prices are understood.
    pub fn parse(bytes: &[u8]) -> AppResult<Self> {
        #[derive(Deserialize)]
        struct Raw {
            currency: String,
            #[serde(default)]
            models: BTreeMap<String, ModelPrice>,
        }
        let raw: Raw = serde_json::from_slice(bytes).map_err(|err| {
            AppError::new(AppErrorCode::ManifestInvalid, format!("pricing: {err}"))
        })?;
        if raw.currency != "USD" {
            return Err(AppError::new(
                AppErrorCode::ManifestInvalid,
                format!("pricing: currency {}", raw.currency),
            ));
        }
        let models = raw
            .models
            .into_iter()
            .filter(|(_, p)| {
                p.input_per_mtok.is_finite()
                    && p.output_per_mtok.is_finite()
                    && p.input_per_mtok >= 0.0
                    && p.output_per_mtok >= 0.0
            })
            .collect();
        Ok(Self { models })
    }

    /// The price of `model`: its own entry, else the entry it extends with a
    /// date (snapshots such as `claude-x-20261101` or `gpt-x-2026-11-01`).
    pub fn price(&self, provider: ProviderId, model: &str) -> Option<ModelPrice> {
        let key = super::model_key(provider, model);
        if let Some(price) = self.models.get(&key) {
            return Some(*price);
        }
        self.models
            .iter()
            .find(|(name, _)| {
                key.strip_prefix(name.as_str())
                    .and_then(|rest| rest.strip_prefix('-'))
                    .is_some_and(|date| {
                        date.len() >= 4 && date.chars().all(|c| c.is_ascii_digit() || c == '-')
                    })
            })
            .map(|(_, price)| *price)
    }

    /// The cost of `usage` with `model`, if its price is known.
    pub fn cost(&self, provider: ProviderId, model: &str, usage: Usage) -> Option<f64> {
        let price = self.price(provider, model)?;
        Some(
            f64::from(usage.input_tokens) * price.input_per_mtok / 1e6
                + f64::from(usage.output_tokens) * price.output_per_mtok / 1e6,
        )
    }
}

/// Loads `pricing.json` and keeps it for a day.
pub struct PricingStore {
    cache_path: PathBuf,
    state: Mutex<Option<(Arc<Pricing>, SystemTime)>>,
}

impl PricingStore {
    /// A store caching under `<data_dir>/cache/`.
    pub fn new(data_dir: &Path) -> Self {
        Self {
            cache_path: data_dir.join("cache").join(PRICING_FILE),
            state: Mutex::default(),
        }
    }

    /// The prices: kept, cached or fetched from `url` (the manifest's
    /// `pricing_url`). Never fails: without prices every cost is unknown.
    pub async fn load(&self, net: &Net, source: &PackSource, url: Option<&str>) -> Arc<Pricing> {
        if let PackSource::Local(dir) = source {
            return Arc::new(
                std::fs::read(dir.join(PRICING_FILE))
                    .ok()
                    .and_then(|bytes| Pricing::parse(&bytes).ok())
                    .unwrap_or_default(),
            );
        }
        if let Some((pricing, at)) = self.kept().or_else(|| self.read_cache())
            && at.elapsed().is_ok_and(|age| age < PRICING_MAX_AGE)
        {
            return pricing;
        }
        let fetched = match url {
            Some(url) => fetch(net, url).await,
            None => Err(AppError::new(
                AppErrorCode::ManifestUnavailable,
                "no pricing_url",
            )),
        };
        match fetched {
            Ok((pricing, bytes)) => {
                if let Err(err) = self.save_cache(&bytes) {
                    eprintln!("caching pricing.json failed: {err}");
                }
                let pricing = Arc::new(pricing);
                *self.lock() = Some((Arc::clone(&pricing), SystemTime::now()));
                pricing
            }
            Err(err) => {
                eprintln!("fetching pricing.json failed: {err}");
                self.kept()
                    .or_else(|| self.read_cache())
                    .map(|(pricing, _)| pricing)
                    .unwrap_or_default()
            }
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<(Arc<Pricing>, SystemTime)>> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn kept(&self) -> Option<(Arc<Pricing>, SystemTime)> {
        self.lock().clone()
    }

    fn read_cache(&self) -> Option<(Arc<Pricing>, SystemTime)> {
        let bytes = std::fs::read(&self.cache_path).ok()?;
        let pricing = Pricing::parse(&bytes).ok()?;
        let at = std::fs::metadata(&self.cache_path)
            .and_then(|m| m.modified())
            .ok()?;
        Some((Arc::new(pricing), at))
    }

    fn save_cache(&self, bytes: &[u8]) -> std::io::Result<()> {
        if let Some(dir) = self.cache_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.cache_path.with_extension("json.tmp");
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(tmp, &self.cache_path)
    }
}

async fn fetch(net: &Net, url: &str) -> AppResult<(Pricing, Vec<u8>)> {
    let mut response = net.get(url, Purpose::Manifest, None).await?;
    if !response.status().is_success() {
        return Err(AppError::new(
            AppErrorCode::DownloadFailed,
            format!("{}: {}", response.host(), response.status()),
        ));
    }
    let bytes = response.bytes_limited(MAX_PRICING_BYTES).await?;
    Ok((Pricing::parse(&bytes)?, bytes))
}

/// Estimates the tokens of `text`: about one per 3.5 characters, one per 2
/// for Arabic script.
pub fn estimate_tokens(text: &str) -> u32 {
    let (arabic, other) = text.chars().fold((0_u32, 0_u32), |(a, o), c| {
        if is_arabic(c) {
            (a.saturating_add(1), o)
        } else {
            (a, o.saturating_add(1))
        }
    });
    let tokens = f64::from(arabic) / 2.0 + f64::from(other) / 3.5;
    // Saturates: estimates never exceed u32.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "rounded up, non-negative and saturating"
    )]
    let tokens = tokens.ceil() as u32;
    tokens
}

/// Estimated input tokens of a prompt.
pub fn estimate_prompt(system: &str, messages: &[&str]) -> u32 {
    let mut tokens = estimate_tokens(system).saturating_add(MESSAGE_OVERHEAD);
    for message in messages {
        tokens = tokens
            .saturating_add(estimate_tokens(message))
            .saturating_add(MESSAGE_OVERHEAD);
    }
    tokens
}

fn is_arabic(c: char) -> bool {
    matches!(c as u32,
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
}

/// What an action is estimated to cost before the user confirms it.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct CostEstimate {
    /// Estimated input tokens.
    pub input_tokens: u32,
    /// Estimated output tokens.
    pub output_tokens: u32,
    /// Estimated cost in USD; `None` if the model's price is unknown.
    pub cost_usd: Option<f64>,
    /// The provider of the model.
    pub provider: ProviderId,
    /// The model.
    pub model: String,
    /// Whether this would pass the monthly limit (spec §9.4): the interface
    /// asks for confirmation, it does not block.
    pub over_limit: bool,
}

impl CostEstimate {
    /// The estimate for `usage` with `model`, checked against the month's
    /// spending so far.
    pub fn new(
        pricing: &Pricing,
        provider: ProviderId,
        model: &str,
        usage: Usage,
        conn: &Connection,
        settings: &Settings,
    ) -> AppResult<Self> {
        let cost_usd = pricing.cost(provider, model, usage);
        let over_limit = match (monthly_limit(settings), cost_usd) {
            (Some(limit), Some(cost)) => month_spent(conn)? + cost > limit,
            (Some(limit), None) => month_spent(conn)? > limit,
            (None, _) => false,
        };
        Ok(Self {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cost_usd,
            provider,
            model: model.to_owned(),
            over_limit,
        })
    }
}

/// The monthly spending limit, if one is set.
pub fn monthly_limit(settings: &Settings) -> Option<f64> {
    settings
        .get(MONTHLY_LIMIT_SETTING)
        .and_then(|json| serde_json::from_str::<f64>(json).ok())
        .filter(|limit| limit.is_finite() && *limit >= 0.0)
}

/// What a logged request was for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    /// A chat answer.
    Chat,
    /// A tafsir summary or translation.
    Translate,
}

impl Feature {
    fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Translate => "translate",
        }
    }
}

/// Logs the tokens and cost of one request in `llm_usage`.
pub fn record_usage(
    conn: &Connection,
    feature: Feature,
    provider: ProviderId,
    model: &str,
    usage: Usage,
    cost_usd: Option<f64>,
) -> AppResult<()> {
    conn.execute(
        &format!(
            "INSERT INTO llm_usage (feature, provider, model, input_tokens, output_tokens, cost_usd, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, {})",
            crate::notes::NOW
        ),
        params![
            feature.as_str(),
            provider.as_str(),
            model,
            usage.input_tokens,
            usage.output_tokens,
            cost_usd
        ],
    )?;
    Ok(())
}

/// The known spending of the current calendar month (UTC), in USD.
pub fn month_spent(conn: &Connection) -> AppResult<f64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(cost_usd), 0) FROM llm_usage
         WHERE substr(created_at, 1, 7) = strftime('%Y-%m', 'now')",
        [],
        |row| row.get(0),
    )?)
}

/// The period of a spending summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum UsagePeriod {
    /// The current calendar month (UTC).
    Month,
    /// Everything logged.
    Total,
}

/// Spending on one feature with one model.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct UsageRow {
    /// The feature.
    pub feature: Feature,
    /// The provider, as stored.
    pub provider: String,
    /// The model.
    pub model: String,
    /// Number of requests.
    pub requests: u32,
    /// Input tokens.
    pub input_tokens: u32,
    /// Output tokens.
    pub output_tokens: u32,
    /// Known cost in USD; `None` if no request had a known price.
    pub cost_usd: Option<f64>,
    /// Requests whose price was unknown.
    pub unpriced: u32,
}

/// Spending in a period (Settings → AI → Spending).
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct UsageSummary {
    /// The known cost of all requests, in USD.
    pub total_usd: f64,
    /// Per feature and model, most expensive first.
    pub rows: Vec<UsageRow>,
    /// The monthly limit, if one is set.
    pub monthly_limit_usd: Option<f64>,
}

/// Sums `llm_usage` over `period`.
pub fn usage_summary(
    conn: &Connection,
    period: UsagePeriod,
    settings: &Settings,
) -> AppResult<UsageSummary> {
    let filter = match period {
        UsagePeriod::Month => "WHERE substr(created_at, 1, 7) = strftime('%Y-%m', 'now')",
        UsagePeriod::Total => "",
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT feature, provider, model, COUNT(*), SUM(input_tokens), SUM(output_tokens),
                SUM(cost_usd), SUM(cost_usd IS NULL)
         FROM llm_usage {filter}
         GROUP BY feature, provider, model
         ORDER BY COALESCE(SUM(cost_usd), 0) DESC, feature, provider, model"
    ))?;
    let clamp = |n: i64| u32::try_from(n.max(0)).unwrap_or(u32::MAX);
    let rows = stmt
        .query_map([], |row| {
            let feature: String = row.get(0)?;
            Ok(UsageRow {
                feature: if feature == "chat" {
                    Feature::Chat
                } else {
                    Feature::Translate
                },
                provider: row.get(1)?,
                model: row.get(2)?,
                requests: clamp(row.get(3)?),
                input_tokens: clamp(row.get(4)?),
                output_tokens: clamp(row.get(5)?),
                cost_usd: row.get(6)?,
                unpriced: clamp(row.get(7)?),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(UsageSummary {
        total_usd: rows.iter().filter_map(|r| r.cost_usd).sum(),
        rows,
        monthly_limit_usd: monthly_limit(settings),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pricing() -> Pricing {
        Pricing::parse(
            br#"{"updated_at":"2026-11-07","currency":"USD","models":{
                "anthropic/claude-sonnet-5-5":{"input_per_mtok":2.0,"output_per_mtok":10.0},
                "anthropic/claude":{"input_per_mtok":99.0,"output_per_mtok":99.0},
                "openai/bad":{"input_per_mtok":-1.0,"output_per_mtok":1.0}}}"#,
        )
        .unwrap()
    }

    fn user_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate(&mut conn, crate::db::user::MIGRATIONS).unwrap();
        conn
    }

    #[test]
    fn prices_match_exact_names_then_dated_snapshots() {
        let pricing = pricing();
        let usage = Usage {
            input_tokens: 1_000_000,
            output_tokens: 100_000,
        };
        let cost = pricing.cost(ProviderId::Anthropic, "claude-sonnet-5-5", usage);
        assert!((cost.unwrap() - 3.0).abs() < 1e-9);
        let dated = pricing.price(ProviderId::Anthropic, "claude-sonnet-5-5-20261101");
        assert_eq!(dated.unwrap().input_per_mtok, 2.0);
        // `claude-sonnet-5-5x` does not extend a name with a dash.
        assert_eq!(
            pricing.price(ProviderId::Anthropic, "claude-sonnet-5-5x"),
            None
        );
        assert_eq!(pricing.price(ProviderId::OpenAi, "bad"), None);
        assert_eq!(pricing.price(ProviderId::Gemini, "claude-sonnet-5-5"), None);
        assert!(Pricing::parse(br#"{"currency":"EUR","models":{}}"#).is_err());
    }

    #[test]
    fn arabic_counts_more_tokens_per_character() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("patience"), 3);
        assert_eq!(estimate_tokens("الصبر"), 3);
        assert!(estimate_prompt("system", &["a", "b"]) > estimate_tokens("systemab"));
    }

    #[test]
    fn usage_is_summed_per_period_and_checked_against_the_limit() {
        let conn = user_db();
        let usage = Usage {
            input_tokens: 1000,
            output_tokens: 100,
        };
        record_usage(
            &conn,
            Feature::Chat,
            ProviderId::Anthropic,
            "m",
            usage,
            Some(0.5),
        )
        .unwrap();
        record_usage(
            &conn,
            Feature::Chat,
            ProviderId::Anthropic,
            "m",
            usage,
            Some(0.25),
        )
        .unwrap();
        record_usage(
            &conn,
            Feature::Translate,
            ProviderId::OpenAiCompatible,
            "local",
            usage,
            None,
        )
        .unwrap();
        conn.execute(
            "INSERT INTO llm_usage (feature, provider, model, input_tokens, output_tokens, cost_usd, created_at)
             VALUES ('chat', 'anthropic', 'm', 1, 1, 10.0, '2020-01-01T00:00:00Z')",
            [],
        )
        .unwrap();

        let settings: Settings = [(MONTHLY_LIMIT_SETTING.to_owned(), "1.0".to_owned())].into();
        let month = usage_summary(&conn, UsagePeriod::Month, &settings).unwrap();
        assert!((month.total_usd - 0.75).abs() < 1e-9);
        assert_eq!(month.monthly_limit_usd, Some(1.0));
        assert_eq!(month.rows[0].requests, 2);
        assert_eq!(month.rows[0].input_tokens, 2000);
        assert_eq!(month.rows[1].cost_usd, None);
        assert_eq!(month.rows[1].unpriced, 1);
        let total = usage_summary(&conn, UsagePeriod::Total, &Settings::new()).unwrap();
        assert!((total.total_usd - 10.75).abs() < 1e-9);

        let pricing = pricing();
        let small = Usage {
            input_tokens: 1000,
            output_tokens: 1000,
        };
        let estimate = CostEstimate::new(
            &pricing,
            ProviderId::Anthropic,
            "claude-sonnet-5-5",
            small,
            &conn,
            &settings,
        )
        .unwrap();
        assert!(!estimate.over_limit);
        let big = Usage {
            input_tokens: 100_000,
            output_tokens: 10_000,
        };
        let estimate = CostEstimate::new(
            &pricing,
            ProviderId::Anthropic,
            "claude-sonnet-5-5",
            big,
            &conn,
            &settings,
        )
        .unwrap();
        assert!(estimate.over_limit);
        let estimate = CostEstimate::new(
            &pricing,
            ProviderId::Anthropic,
            "claude-sonnet-5-5",
            big,
            &conn,
            &Settings::new(),
        )
        .unwrap();
        assert!(!estimate.over_limit);
    }
}
