//! On-demand tafsir summaries and translations with the user's own model,
//! cached in `user.db` and always labelled as machine output (spec §9, ADR 0004).
//!
//! A **summary** covers a whole passage (`paragraph_idx = -1` in
//! `tafsir_translations`); a **full** translation covers one paragraph, and
//! the interface asks for the next one on "translate more". Cached results
//! are used only while the pack has the version they were made from.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio_util::sync::CancellationToken;

use super::cost::estimate_prompt;
use super::{Completion, LlmProvider, LlmRequest, Message, Role, TextSink, Usage};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::export::markdown::html_to_markdown;
use crate::net::Purpose;
use crate::secrets::ProviderId;

/// Longest passage summarized, in characters; longer ones are cut, marked
/// with "[…]". About 50,000 tokens of Arabic.
const MAX_SUMMARY_CHARS: usize = 100_000;

/// Output tokens estimated for a summary.
const SUMMARY_OUTPUT_TOKENS: u32 = 500;

/// Most output tokens a summary may use (thinking models count their
/// thinking here too).
const SUMMARY_MAX_TOKENS: u32 = 4096;

/// Most output tokens a paragraph translation may use.
const FULL_MAX_TOKENS: u32 = 16_384;

/// Interface languages a tafsir can be summarized or translated into.
const TARGET_LANGUAGES: &[(&str, &str)] = &[("tr", "Turkish"), ("en", "English")];

/// A summary of the passage or a translation of one paragraph (spec §9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TranslationMode {
    /// A short summary of the whole passage.
    Summary,
    /// A translation of one paragraph.
    Full,
}

impl TranslationMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Full => "full",
        }
    }
}

/// A saved summary or translation, shown next to its original.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct TafsirTranslation {
    /// The translated paragraph; `None` for a summary of the passage.
    pub paragraph_idx: Option<u32>,
    /// Summary or translation.
    pub mode: TranslationMode,
    /// The interface language it is in.
    pub target_lang: String,
    /// Plain text, untrusted model output.
    pub text: String,
    /// The provider that wrote it.
    pub provider: String,
    /// The model that wrote it.
    pub model: String,
    /// When it was made (UTC).
    pub created_at: String,
}

/// Everything a summary or translation needs from `content.db`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    /// The tafsir pack.
    pub pack_id: String,
    /// Its version: the cache key's guard.
    pub pack_version: String,
    /// The passage (`tafsir.id`).
    pub tafsir_id: u32,
    /// The paragraph; `None` for a summary.
    pub paragraph_idx: Option<u32>,
    /// Summary or translation.
    pub mode: TranslationMode,
    /// The language to write in.
    pub target_lang: String,
    /// The system prompt.
    pub system: String,
    /// The source text sent.
    pub source: String,
}

impl Job {
    /// Reads the passage (or paragraph) of `tafsir_id` in `pack_id`.
    pub fn prepare(
        content: &Connection,
        pack_id: &str,
        tafsir_id: u32,
        paragraph_idx: Option<u32>,
        mode: TranslationMode,
        target_lang: &str,
    ) -> AppResult<Self> {
        let language = TARGET_LANGUAGES
            .iter()
            .find(|(code, _)| *code == target_lang)
            .map(|(_, name)| *name)
            .ok_or_else(|| {
                AppError::new(
                    AppErrorCode::InvalidInput,
                    format!("cannot translate into {target_lang:?}"),
                )
            })?;
        let (pack_version, title, author): (String, String, Option<String>) = content
            .query_row(
                "SELECT p.version, p.title, p.author FROM tafsir t JOIN packs p ON p.id = t.pack_id
                 WHERE t.id = ?1 AND t.pack_id = ?2 AND p.type = 'tafsir'",
                params![tafsir_id, pack_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| {
                AppError::new(
                    AppErrorCode::NotFound,
                    format!("tafsir passage {tafsir_id} of {pack_id}"),
                )
            })?;
        let mut stmt = content
            .prepare("SELECT idx, text FROM tafsir_paragraphs WHERE tafsir_id = ?1 ORDER BY idx")?;
        let paragraphs: Vec<(u32, String)> = stmt
            .query_map([tafsir_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let work = match &author {
            Some(author) => format!("{title} by {author}"),
            None => title,
        };
        let (paragraph_idx, source, system) = match mode {
            TranslationMode::Summary => {
                let mut source = paragraphs
                    .iter()
                    .map(|(_, text)| html_to_markdown(text).trim().to_owned())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                if let Some((cut, _)) = source.char_indices().nth(MAX_SUMMARY_CHARS) {
                    source.truncate(cut);
                    source.push_str(" […]");
                }
                (None, source, summary_prompt(&work, language))
            }
            TranslationMode::Full => {
                let idx = paragraph_idx.ok_or_else(|| {
                    AppError::new(
                        AppErrorCode::InvalidInput,
                        "a translation needs a paragraph",
                    )
                })?;
                let (_, text) = paragraphs.iter().find(|(i, _)| *i == idx).ok_or_else(|| {
                    AppError::new(AppErrorCode::NotFound, format!("paragraph {idx}"))
                })?;
                (
                    Some(idx),
                    html_to_markdown(text).trim().to_owned(),
                    translation_prompt(&work, language),
                )
            }
        };
        if source.is_empty() {
            return Err(AppError::new(
                AppErrorCode::NotFound,
                "the passage has no text",
            ));
        }
        Ok(Self {
            pack_id: pack_id.to_owned(),
            pack_version,
            tafsir_id,
            paragraph_idx,
            mode,
            target_lang: target_lang.to_owned(),
            system,
            source,
        })
    }

    /// The request for `model`.
    pub fn request(&self, model: &str) -> LlmRequest {
        LlmRequest {
            model: model.to_owned(),
            system: self.system.clone(),
            messages: vec![Message {
                role: Role::User,
                content: self.source.clone(),
            }],
            max_tokens: match self.mode {
                TranslationMode::Summary => SUMMARY_MAX_TOKENS,
                TranslationMode::Full => FULL_MAX_TOKENS,
            },
            purpose: Purpose::Translate,
        }
    }

    /// The tokens this is expected to use (spec §9.2).
    pub fn estimate(&self) -> Usage {
        let input_tokens = estimate_prompt(&self.system, &[&self.source]);
        let output_tokens = match self.mode {
            TranslationMode::Summary => SUMMARY_OUTPUT_TOKENS,
            // A translation is about as long as its source.
            TranslationMode::Full => {
                input_tokens.saturating_sub(estimate_prompt(&self.system, &[]))
            }
        };
        Usage {
            input_tokens,
            output_tokens,
        }
    }

    /// Streams the answer of `provider`'s `model`; the text is returned whole.
    pub async fn run(
        &self,
        provider: &dyn LlmProvider,
        model: &str,
        on_text: TextSink<'_>,
        cancel: &CancellationToken,
    ) -> AppResult<(String, Completion)> {
        let text = std::sync::Mutex::new(String::new());
        let collect = |piece: &str| {
            text.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push_str(piece);
            on_text(piece);
        };
        let request = self.request(model);
        let completion = provider.stream(&request, &collect, cancel).await?;
        let text = text
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok((text.trim().to_owned(), completion))
    }

    /// Saves `text` in the cache and returns it as shown.
    pub fn save(
        &self,
        user: &Connection,
        text: &str,
        provider: ProviderId,
        model: &str,
    ) -> AppResult<TafsirTranslation> {
        user.execute(
            &format!(
                "INSERT OR REPLACE INTO tafsir_translations
                   (pack_id, tafsir_id, paragraph_idx, mode, target_lang, pack_version, text,
                    provider, model, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, {})",
                crate::notes::NOW
            ),
            params![
                self.pack_id,
                self.tafsir_id,
                stored_idx(self.paragraph_idx),
                self.mode.as_str(),
                self.target_lang,
                self.pack_version,
                text,
                provider.as_str(),
                model
            ],
        )?;
        let found = cached(
            user,
            &self.pack_id,
            self.tafsir_id,
            &self.target_lang,
            &self.pack_version,
        )?
        .into_iter()
        .find(|t| t.mode == self.mode && t.paragraph_idx == self.paragraph_idx);
        found.ok_or_else(|| AppError::new(AppErrorCode::QueryFailed, "saved translation missing"))
    }
}

/// The saved summaries and translations of a passage in `target_lang` made
/// from `pack_version`; older ones are stale and left out (ADR 0004).
pub fn cached(
    user: &Connection,
    pack_id: &str,
    tafsir_id: u32,
    target_lang: &str,
    pack_version: &str,
) -> AppResult<Vec<TafsirTranslation>> {
    let mut stmt = user.prepare(
        "SELECT paragraph_idx, mode, target_lang, text, provider, model, created_at
         FROM tafsir_translations
         WHERE pack_id = ?1 AND tafsir_id = ?2 AND target_lang = ?3 AND pack_version = ?4
         ORDER BY mode DESC, paragraph_idx",
    )?;
    let rows = stmt.query_map(
        params![pack_id, tafsir_id, target_lang, pack_version],
        |row| {
            let idx: i64 = row.get(0)?;
            let mode: String = row.get(1)?;
            Ok(TafsirTranslation {
                paragraph_idx: u32::try_from(idx).ok(),
                mode: if mode == "full" {
                    TranslationMode::Full
                } else {
                    TranslationMode::Summary
                },
                target_lang: row.get(2)?,
                text: row.get(3)?,
                provider: row.get(4)?,
                model: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    )?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The installed version of a tafsir pack.
pub fn pack_version(content: &Connection, pack_id: &str) -> AppResult<String> {
    content
        .query_row(
            "SELECT version FROM packs WHERE id = ?1 AND type = 'tafsir'",
            [pack_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("tafsir {pack_id}")))
}

fn stored_idx(idx: Option<u32>) -> i64 {
    idx.map_or(-1, i64::from)
}

fn summary_prompt(work: &str, language: &str) -> String {
    format!(
        "You summarize a passage of classical Quran commentary ({work}) for a reader of {language}.\n\
         Write the summary in {language}, in plain text without Markdown, in 3 to 6 sentences.\n\
         Keep to what the passage says: add no commentary, opinion or ruling of your own.\n\
         When you use a technical term (legal, theological, grammatical), give the original Arabic \
         term in parentheses after it.\n\
         Write verse references as [surah:ayah].\n\
         If a phrase is unclear to you, say so briefly instead of guessing."
    )
}

fn translation_prompt(work: &str, language: &str) -> String {
    format!(
        "You translate a paragraph of classical Quran commentary ({work}) into {language}.\n\
         Translate faithfully and completely, in plain text without Markdown; keep line breaks.\n\
         Add no commentary, explanation or opinion of your own.\n\
         Keep technical terms (legal, theological, grammatical) with the original Arabic term in \
         parentheses after them.\n\
         Leave quoted Quran verses in Arabic and add their translation in parentheses.\n\
         Mark a phrase you are unsure of with [?]."
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::collections::test_support::{content, user};
    use crate::llm::Finish;
    use crate::llm::fake::FakeProvider;

    #[test]
    fn prepares_a_summary_of_the_whole_passage() {
        let job = Job::prepare(
            &content(),
            "ar.tafsir",
            7,
            None,
            TranslationMode::Summary,
            "tr",
        )
        .unwrap();
        assert_eq!(job.pack_version, "1.0.0");
        assert_eq!(job.paragraph_idx, None);
        assert_eq!(job.source, "Bu **ayet** & devamı  \nikinci satır\n\nikinci");
        assert!(job.system.contains("Tafsir Test by al-Author"));
        assert!(job.system.contains("in Turkish"));
        let estimate = job.estimate();
        assert!(estimate.input_tokens > 0);
        assert_eq!(estimate.output_tokens, SUMMARY_OUTPUT_TOKENS);
        assert_eq!(job.request("m").purpose, Purpose::Translate);
    }

    #[test]
    fn prepares_one_paragraph_and_refuses_what_does_not_exist() {
        let conn = content();
        let job =
            Job::prepare(&conn, "ar.tafsir", 7, Some(1), TranslationMode::Full, "en").unwrap();
        assert_eq!(job.source, "ikinci");
        assert!(job.estimate().output_tokens > 0);
        let code = |r: AppResult<Job>| r.unwrap_err().code;
        assert_eq!(
            code(Job::prepare(
                &conn,
                "ar.tafsir",
                7,
                Some(9),
                TranslationMode::Full,
                "en"
            )),
            AppErrorCode::NotFound
        );
        assert_eq!(
            code(Job::prepare(
                &conn,
                "ar.tafsir",
                7,
                None,
                TranslationMode::Full,
                "en"
            )),
            AppErrorCode::InvalidInput
        );
        assert_eq!(
            code(Job::prepare(
                &conn,
                "en.test",
                7,
                None,
                TranslationMode::Summary,
                "en"
            )),
            AppErrorCode::NotFound
        );
        assert_eq!(
            code(Job::prepare(
                &conn,
                "ar.tafsir",
                7,
                None,
                TranslationMode::Summary,
                "de"
            )),
            AppErrorCode::InvalidInput
        );
    }

    #[tokio::test]
    async fn streams_saves_and_serves_the_cache_for_its_version_only() {
        let content = content();
        let user = user();
        let job = Job::prepare(
            &content,
            "ar.tafsir",
            7,
            Some(0),
            TranslationMode::Full,
            "tr",
        )
        .unwrap();
        let provider = FakeProvider::answering(
            &["Bu ayet ", "(āyah) ve devamı. "],
            Usage {
                input_tokens: 50,
                output_tokens: 10,
            },
        );
        let pieces = Mutex::new(Vec::new());
        let on_text = |piece: &str| pieces.lock().unwrap().push(piece.to_owned());
        let (text, completion) = job
            .run(&*provider, "m-1", &on_text, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(text, "Bu ayet (āyah) ve devamı.");
        assert_eq!(pieces.lock().unwrap().len(), 2);
        assert_eq!(completion.finish, Finish::Complete);
        assert_eq!(provider.requests()[0].messages[0].content, job.source);

        let saved = job
            .save(&user, &text, ProviderId::Anthropic, "m-1")
            .unwrap();
        assert_eq!(saved.paragraph_idx, Some(0));
        assert_eq!(saved.provider, "anthropic");
        // Saving again replaces the cached text.
        job.save(&user, "yeni", ProviderId::Anthropic, "m-2")
            .unwrap();
        let found = cached(&user, "ar.tafsir", 7, "tr", "1.0.0").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "yeni");
        assert!(
            cached(&user, "ar.tafsir", 7, "tr", "1.1.0")
                .unwrap()
                .is_empty()
        );
        assert!(
            cached(&user, "ar.tafsir", 7, "en", "1.0.0")
                .unwrap()
                .is_empty()
        );
        assert_eq!(pack_version(&content, "ar.tafsir").unwrap(), "1.0.0");
    }
}
