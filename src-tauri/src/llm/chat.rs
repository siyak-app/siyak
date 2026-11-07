//! RAG chat: retrieve verses, add translations and budgeted tafsir context,
//! stream the answer and record cited verses and sources (spec §10).
//!
//! The context of each answer is built afresh from the new message:
//! 1. the chat model rewrites the message, with the last few messages of the
//!    session, into one descriptive search query ([`search_query_request`]):
//!    a short or follow-up question ("İslam'da miras", "peki kızların payı?")
//!    finds the right verses only when it names its topic the way a
//!    translation would;
//! 2. the [`TOP_K`] verses closest to that query in the active vector pack,
//!    each with its Arabic text and the reader's translations;
//! 3. the chat tafsirs (`chat.tafsirs`, at most [`MAX_CHAT_TAFSIRS`]) of the
//!    [`TAFSIR_VERSES`] best verses, within [`TAFSIR_BUDGET`] tokens: a passage
//!    longer than its share keeps its opening paragraphs, in order, with "[…]"
//!    for the rest. Tafsir is not embedded. Arabic tafsirs go only to a model
//!    that may read Arabic sources (spec §8).
//!
//! The sources are stored with the answer (`sources_json`) and the verses it
//! cites (`[surah:ayah]`) in `cited_verses`.

use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::cost::estimate_tokens;
use super::{LlmRequest, Message, Role};
use crate::db::content::{self, VerseRef};
use crate::db::user::Settings;
use crate::embed::index::Neighbor;
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::export::markdown::html_to_markdown;
use crate::net::Purpose;

/// Setting: the tafsir packs whose commentary goes into chat (JSON array).
pub const CHAT_TAFSIRS_SETTING: &str = "chat.tafsirs";

/// The chat tafsirs until the user picks others (spec §10).
pub const DEFAULT_CHAT_TAFSIRS: &[&str] = &["ar.jalalayn", "ar.ibn-kathir"];

/// Most chat tafsirs.
pub const MAX_CHAT_TAFSIRS: usize = 3;

/// Verses retrieved per message.
pub const TOP_K: usize = 8;

/// The best verses whose tafsir is added.
pub const TAFSIR_VERSES: usize = 3;

/// Tokens of tafsir per message, spread over [`TAFSIR_VERSES`].
pub const TAFSIR_BUDGET: u32 = 6000;

/// Earlier messages of the session sent along.
const HISTORY: u32 = 12;

/// Longest message accepted.
const MAX_MESSAGE_CHARS: usize = 8000;

/// Most verses one citation range expands to.
const MAX_CITED_RANGE: u16 = 20;

/// Most tokens an answer may use.
pub const ANSWER_MAX_TOKENS: u32 = 8192;

/// Answer tokens estimated before sending.
pub const ANSWER_ESTIMATE_TOKENS: u32 = 800;

/// Prompt tokens of the retrieved context, estimated before sending.
pub const CONTEXT_ESTIMATE_TOKENS: u32 = TAFSIR_BUDGET + 8 * 150 + 400;

/// Earlier messages of the session the search query is written from.
const QUERY_HISTORY: usize = 4;

/// Characters of each earlier message sent for the search query.
const QUERY_HISTORY_CHARS: usize = 1500;

/// Most tokens the search query may use; some models think before answering.
pub const QUERY_MAX_TOKENS: u32 = 1024;

/// Longest search query kept, in characters.
const MAX_QUERY_CHARS: usize = 400;

/// Tokens of the search-query request besides the message and history, and of
/// its answer, estimated before sending.
pub const QUERY_ESTIMATE_TOKENS: (u32, u32) = (250, 60);

/// What went into the context of an answer, shown as "sources used".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct ChatSources {
    /// The retrieved verses, best first.
    pub verses: Vec<SourceVerse>,
    /// The tafsir passages added.
    pub tafsirs: Vec<SourceTafsir>,
    /// What was left out and why.
    pub notices: Vec<ChatNotice>,
    /// The search query the verses were retrieved with, if the model wrote
    /// one (answers saved before it was recorded have none).
    #[serde(default)]
    pub search_query: Option<String>,
}

/// A retrieved verse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct SourceVerse {
    /// Global verse id.
    pub verse_id: u32,
    /// Surah and verse number.
    pub verse: VerseRef,
    /// Similarity to the question.
    pub score: f32,
}

/// A tafsir passage, or the paragraphs of it that fit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct SourceTafsir {
    /// Tafsir pack.
    pub pack_id: String,
    /// Its title.
    pub title: String,
    /// The passage (`tafsir.id`).
    pub tafsir_id: u32,
    /// First verse it covers.
    pub verse_from: VerseRef,
    /// Last verse it covers.
    pub verse_to: VerseRef,
    /// The paragraphs used, in order.
    pub paragraphs: Vec<u32>,
    /// Whether every paragraph was used.
    pub complete: bool,
}

/// Something the context lacks, said to the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChatNotice {
    /// The model may not read Arabic, so Arabic tafsirs were left out.
    ArabicTafsirsSkipped,
    /// A chat tafsir is not installed.
    TafsirMissing {
        /// Its pack id.
        pack_id: String,
    },
}

/// A verse the answer cites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct VerseCitation {
    /// Global verse id.
    pub verse_id: u32,
    /// Surah and verse number.
    pub verse: VerseRef,
}

/// A message of a session.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ChatMessage {
    /// Message id.
    pub id: u32,
    /// Who wrote it.
    pub role: Role,
    /// Its text; an answer is untrusted model output.
    pub content: String,
    /// The verses an answer cites, in order.
    pub cited: Vec<VerseCitation>,
    /// What went into an answer's context.
    pub sources: Option<ChatSources>,
    /// When it was written (UTC).
    pub created_at: String,
}

/// A chat session in the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct ChatSessionSummary {
    /// Session id.
    pub id: u32,
    /// Its title: the start of the first message, or the user's.
    pub title: Option<String>,
    /// The provider of its answers.
    pub provider: String,
    /// The model of its answers.
    pub model: String,
    /// When it started.
    pub created_at: String,
    /// When its last message was written.
    pub updated_at: String,
}

/// A session with its messages, oldest first.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ChatSession {
    /// The session.
    pub session: ChatSessionSummary,
    /// Its messages.
    pub messages: Vec<ChatMessage>,
}

/// A verse of the context.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextVerse {
    /// Global verse id.
    pub verse_id: u32,
    /// Surah and verse number.
    pub verse: VerseRef,
    /// Similarity to the question.
    pub score: f32,
    /// Arabic text.
    pub arabic: String,
    /// `(translation title, text)` in the reader's order.
    pub translations: Vec<(String, String)>,
}

/// A tafsir passage that may go into the context, before it is fitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// What is recorded as a source.
    pub source: SourceTafsir,
    /// Paragraphs `(idx, plain text)`, in order.
    pub paragraphs: Vec<(u32, String)>,
    /// Its share of [`TAFSIR_BUDGET`], in tokens.
    pub budget: u32,
    /// Whether it is in Arabic (quoted in its original wording).
    pub arabic: bool,
}

/// The retrieved context of one answer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Context {
    /// The retrieved verses.
    pub verses: Vec<ContextVerse>,
    /// The fitted tafsir passages: the candidate with the paragraphs kept.
    pub tafsirs: Vec<Candidate>,
    /// What was left out.
    pub notices: Vec<ChatNotice>,
    /// The search query the model wrote, if any.
    pub search_query: Option<String>,
}

/// The request that has the chat model rewrite `message` into a search query
/// for the verse vectors, with the end of `history` for follow-up questions.
/// The conversation goes in one user message, so any provider accepts it.
pub fn search_query_request(model: &str, history: &[Message], message: &str) -> LlmRequest {
    let system = "You write the search query for a meaning-based search over the verses of the \
                  Quran. Each verse is indexed with its Arabic text and its Turkish (Diyanet) and \
                  English (Pickthall) translations.\n\
                  Rewrite the user's last message as one descriptive sentence that names its topic \
                  and the specific things it asks about, worded the way a translation of the \
                  relevant verses would word them. Write it in the language of the last message. \
                  If the message continues the conversation, make the sentence stand on its own.\n\
                  Reply with the sentence only: no quotes, no explanation.\n\
                  Example: \"İslam'da miras\" → Kur'an'da miras hükümleri: anne, baba, çocuklar ve \
                  eşlerin mirastan payları";
    let mut content = String::new();
    let earlier = &history[history.len().saturating_sub(QUERY_HISTORY)..];
    if !earlier.is_empty() {
        content.push_str("Conversation so far:\n");
        for m in earlier {
            let who = match m.role {
                Role::User => "User",
                Role::Assistant => "Assistant",
            };
            let text: String = m.content.chars().take(QUERY_HISTORY_CHARS).collect();
            content.push_str(&format!("{who}: {text}\n"));
        }
        content.push('\n');
    }
    content.push_str(&format!("Last message: {message}"));
    LlmRequest {
        model: model.to_owned(),
        system: system.to_owned(),
        messages: vec![Message {
            role: Role::User,
            content,
        }],
        max_tokens: QUERY_MAX_TOKENS,
        purpose: Purpose::Chat,
    }
}

/// The search query in the model's reply: its first line after any thinking,
/// without quotes, at most [`MAX_QUERY_CHARS`]; `None` if there is none.
pub fn parse_search_query(reply: &str) -> Option<String> {
    let reply = reply
        .rsplit_once("</think>")
        .map_or(reply, |(_, after)| after);
    let line = reply
        .lines()
        .map(|l| {
            l.trim()
                .trim_matches(|c: char| matches!(c, '"' | '“' | '”' | '`' | '*'))
                .trim()
        })
        .find(|l| !l.is_empty())?;
    Some(line.chars().take(MAX_QUERY_CHARS).collect())
}

/// Checks a message the user sent.
pub fn check_message(message: &str) -> AppResult<&str> {
    let message = message.trim();
    if message.is_empty() || message.chars().count() > MAX_MESSAGE_CHARS {
        return Err(AppError::new(
            AppErrorCode::InvalidInput,
            "a chat message is between 1 and 8000 characters",
        ));
    }
    Ok(message)
}

/// The chat tafsirs chosen in the settings, or the defaults.
pub fn chat_tafsirs(settings: &Settings) -> Vec<String> {
    let chosen: Option<Vec<String>> = settings
        .get(CHAT_TAFSIRS_SETTING)
        .and_then(|json| serde_json::from_str(json).ok());
    let mut ids = chosen.unwrap_or_else(|| {
        DEFAULT_CHAT_TAFSIRS
            .iter()
            .map(|id| (*id).to_owned())
            .collect()
    });
    let mut seen = HashSet::new();
    ids.retain(|id| seen.insert(id.clone()));
    ids.truncate(MAX_CHAT_TAFSIRS);
    ids
}

/// The translations shown in the reader, which also go into the context.
pub fn reader_translations(settings: &Settings) -> Vec<String> {
    settings
        .get("reader.translations")
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_else(|| vec!["en.pickthall".to_owned()])
}

/// The verses of `neighbors` with their Arabic text and translations.
pub fn context_verses(
    content: &Connection,
    neighbors: &[Neighbor],
    translation_ids: &[String],
) -> AppResult<Vec<ContextVerse>> {
    let mut verse_stmt =
        content.prepare("SELECT surah, ayah, text_uthmani FROM verses WHERE id = ?1")?;
    let mut translation_stmt = content.prepare(
        "SELECT p.title, t.text FROM translations t JOIN packs p ON p.id = t.pack_id
         WHERE t.verse_id = ?1 AND t.pack_id = ?2",
    )?;
    let mut verses = Vec::new();
    for neighbor in neighbors {
        let Some((surah, ayah, arabic)) = verse_stmt
            .query_row([neighbor.verse_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get::<_, String>(2)?))
            })
            .optional()?
        else {
            continue;
        };
        let mut translations = Vec::new();
        for id in translation_ids {
            if let Some(found) = translation_stmt
                .query_row(params![neighbor.verse_id, id], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .optional()?
            {
                translations.push(found);
            }
        }
        verses.push(ContextVerse {
            verse_id: neighbor.verse_id,
            verse: VerseRef { surah, ayah },
            score: neighbor.score,
            arabic,
            translations,
        });
    }
    Ok(verses)
}

/// The passages of the chat tafsirs on the best verses, each with its share
/// of the budget. Arabic tafsirs are left out unless `arabic_sources`.
pub fn tafsir_candidates(
    content: &Connection,
    verses: &[ContextVerse],
    tafsir_ids: &[String],
    arabic_sources: bool,
) -> AppResult<(Vec<Candidate>, Vec<ChatNotice>)> {
    let installed = content::list_tafsirs(content)?;
    let mut notices = Vec::new();
    let mut usable = Vec::new();
    for id in tafsir_ids {
        match installed.iter().find(|t| &t.id == id) {
            None => notices.push(ChatNotice::TafsirMissing {
                pack_id: id.clone(),
            }),
            Some(info) => {
                let arabic = info.lang.as_deref().is_some_and(|l| l.starts_with("ar"));
                if arabic && !arabic_sources {
                    if !notices.contains(&ChatNotice::ArabicTafsirsSkipped) {
                        notices.push(ChatNotice::ArabicTafsirsSkipped);
                    }
                } else {
                    usable.push((info, arabic));
                }
            }
        }
    }
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let top: Vec<&ContextVerse> = verses.iter().take(TAFSIR_VERSES).collect();
    if usable.is_empty() || top.is_empty() {
        return Ok((candidates, notices));
    }
    let per_verse = TAFSIR_BUDGET / u32::try_from(top.len()).unwrap_or(1);
    let share = per_verse / u32::try_from(usable.len()).unwrap_or(1);
    for verse in top {
        for (info, arabic) in &usable {
            let Some(passage) = content::get_tafsir(content, verse.verse_id, &info.id)? else {
                continue;
            };
            if !seen.insert((info.id.clone(), passage.id)) {
                continue;
            }
            let paragraphs: Vec<(u32, String)> = passage
                .paragraphs
                .iter()
                .map(|p| (p.idx, html_to_markdown(&p.text).trim().to_owned()))
                .filter(|(_, text)| !text.is_empty())
                .collect();
            candidates.push(Candidate {
                source: SourceTafsir {
                    pack_id: info.id.clone(),
                    title: info.title.clone(),
                    tafsir_id: passage.id,
                    verse_from: passage.verse_from,
                    verse_to: passage.verse_to,
                    paragraphs: paragraphs.iter().map(|(idx, _)| *idx).collect(),
                    complete: true,
                },
                paragraphs,
                budget: share,
                arabic: *arabic,
            });
        }
    }
    Ok((candidates, notices))
}

/// Cuts each candidate to its budget: a passage that fits stays whole; a
/// longer one keeps its opening paragraphs, in order, until the budget is
/// full. If even the first paragraph is over the budget, it is kept cut to
/// fit. Tafsir is not embedded (spec §10).
pub fn fit_tafsirs(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut fitted = Vec::with_capacity(candidates.len());
    for mut candidate in candidates {
        let mut kept = Vec::new();
        let mut used = 0_u32;
        let mut complete = true;
        for (idx, text) in std::mem::take(&mut candidate.paragraphs) {
            let cost = estimate_tokens(&text);
            if used + cost <= candidate.budget {
                used += cost;
                kept.push((idx, text));
                continue;
            }
            if kept.is_empty() {
                let cut = cut_to_tokens(&text, candidate.budget);
                if !cut.is_empty() {
                    kept.push((idx, cut));
                }
            }
            complete = false;
            break;
        }
        candidate.source.paragraphs = kept.iter().map(|(idx, _)| *idx).collect();
        candidate.source.complete = complete;
        candidate.paragraphs = kept;
        if !candidate.paragraphs.is_empty() {
            fitted.push(candidate);
        }
    }
    fitted
}

/// The longest start of `text`, ending at a word break, within `budget`
/// estimated tokens.
fn cut_to_tokens(text: &str, budget: u32) -> String {
    let breaks: Vec<usize> = text
        .char_indices()
        .filter(|(_, c)| c.is_whitespace())
        .map(|(i, _)| i)
        .collect();
    // The estimate grows with the prefix, so the last break that fits is found
    // by binary search.
    let fits = breaks.partition_point(|&i| estimate_tokens(&text[..i]) <= budget);
    let end = fits.checked_sub(1).map_or(0, |i| breaks[i]);
    text[..end].trim_end().to_owned()
}

impl Context {
    /// The sources recorded with the answer.
    pub fn sources(&self) -> ChatSources {
        ChatSources {
            verses: self
                .verses
                .iter()
                .map(|v| SourceVerse {
                    verse_id: v.verse_id,
                    verse: v.verse,
                    score: v.score,
                })
                .collect(),
            tafsirs: self.tafsirs.iter().map(|t| t.source.clone()).collect(),
            notices: self.notices.clone(),
            search_query: self.search_query.clone(),
        }
    }

    /// The system prompt with the context (spec §10.4), asking for an answer
    /// in `language` (e.g. "Turkish").
    pub fn system_prompt(&self, language: &str) -> String {
        let mut prompt = format!(
            "You answer questions about the Quran in Siyak, a Quran research app. Answer in {language}.\n\
             Rules:\n\
             - Rely only on the sources below. Give a citation in the form [surah:ayah], e.g. [2:153], \
             for every claim, and name the tafsir when you use one.\n\
             - If the sources are not enough to answer, say so plainly instead of answering from memory.\n\
             - Do not issue legal rulings (fatwa); refer such questions to qualified scholars.\n"
        );
        if self.tafsirs.iter().any(|t| t.arabic) {
            prompt.push_str(
                "- When you cite an Arabic tafsir, quote its original Arabic wording and translate it.\n",
            );
        }
        prompt.push_str("\n# Sources\n\n## Verses\n");
        if self.verses.is_empty() {
            prompt.push_str("(no verses were found)\n");
        }
        for verse in &self.verses {
            prompt.push_str(&format!(
                "\n[{}:{}] {}\n",
                verse.verse.surah, verse.verse.ayah, verse.arabic
            ));
            for (title, text) in &verse.translations {
                prompt.push_str(&format!("{title}: {text}\n"));
            }
        }
        if !self.tafsirs.is_empty() {
            prompt.push_str("\n## Tafsir\n");
        }
        for tafsir in &self.tafsirs {
            let source = &tafsir.source;
            let range = if source.verse_from == source.verse_to {
                format!("[{}:{}]", source.verse_from.surah, source.verse_from.ayah)
            } else {
                format!(
                    "[{}:{}]–[{}:{}]",
                    source.verse_from.surah,
                    source.verse_from.ayah,
                    source.verse_to.surah,
                    source.verse_to.ayah
                )
            };
            prompt.push_str(&format!("\n### {} on {range}\n", source.title));
            let mut last: Option<u32> = None;
            for (idx, text) in &tafsir.paragraphs {
                let skipped = match last {
                    None => *idx > 0 && !source.complete,
                    Some(last) => *idx > last + 1,
                };
                if skipped {
                    prompt.push_str("[…]\n");
                }
                prompt.push_str(text);
                prompt.push('\n');
                last = Some(*idx);
            }
            if !source.complete {
                prompt.push_str("[…]\n");
            }
        }
        prompt
    }
}

/// The verses an answer cites: `[2:153]`, `[2:153-157]` and `[2:153, 3:200]`,
/// in order of first mention, only those that exist.
pub fn citations(content: &Connection, text: &str) -> AppResult<Vec<VerseCitation>> {
    let mut stmt = content.prepare("SELECT id FROM verses WHERE surah = ?1 AND ayah = ?2")?;
    let mut found = Vec::new();
    let mut seen = HashSet::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else { break };
        for part in after[..close].split(',') {
            let Some((surah, ayahs)) = part.trim().split_once(':') else {
                continue;
            };
            let Ok(surah) = surah.trim().parse::<u16>() else {
                continue;
            };
            let (from, to) = match ayahs.split_once(['-', '–']) {
                Some((a, b)) => (a.trim().parse::<u16>(), b.trim().parse::<u16>()),
                None => (ayahs.trim().parse::<u16>(), ayahs.trim().parse::<u16>()),
            };
            let (Ok(from), Ok(to)) = (from, to) else {
                continue;
            };
            let to = to.min(from.saturating_add(MAX_CITED_RANGE - 1));
            for ayah in from..=to {
                let id: Option<u32> = stmt
                    .query_row(params![surah, ayah], |row| row.get(0))
                    .optional()?;
                if let Some(verse_id) = id
                    && seen.insert(verse_id)
                {
                    found.push(VerseCitation {
                        verse_id,
                        verse: VerseRef { surah, ayah },
                    });
                }
            }
        }
        rest = &after[close + 1..];
    }
    Ok(found)
}

/// The earlier messages of a session sent with a new one, oldest first.
pub fn history(user: &Connection, session_id: u32) -> AppResult<Vec<Message>> {
    let mut stmt = user.prepare(
        "SELECT role, content FROM (
           SELECT id, role, content FROM chat_messages WHERE session_id = ?1
           ORDER BY id DESC LIMIT ?2)
         ORDER BY id",
    )?;
    let rows = stmt.query_map(params![session_id, HISTORY], |row| {
        let role: String = row.get(0)?;
        Ok(Message {
            role: if role == "assistant" {
                Role::Assistant
            } else {
                Role::User
            },
            content: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Starts a session titled after its first message.
pub fn create_session(
    user: &Connection,
    first_message: &str,
    provider: &str,
    model: &str,
) -> AppResult<u32> {
    let title: String = first_message.chars().take(60).collect();
    user.execute(
        &format!(
            "INSERT INTO chat_sessions (title, provider, model, created_at) VALUES (?1, ?2, ?3, {})",
            crate::notes::NOW
        ),
        params![title.trim(), provider, model],
    )?;
    u32::try_from(user.last_insert_rowid())
        .map_err(|_| AppError::new(AppErrorCode::QueryFailed, "session id out of range"))
}

/// Fails with `not_found` unless the session exists.
pub fn require_session(user: &Connection, session_id: u32) -> AppResult<()> {
    user.query_row(
        "SELECT 1 FROM chat_sessions WHERE id = ?1",
        [session_id],
        |_| Ok(()),
    )
    .optional()?
    .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("chat session {session_id}")))
}

/// Saves a message; an answer carries its citations and sources.
pub fn add_message(
    user: &Connection,
    session_id: u32,
    role: Role,
    content: &str,
    cited: &[VerseCitation],
    sources: Option<&ChatSources>,
) -> AppResult<ChatMessage> {
    let cited_json = (!cited.is_empty())
        .then(|| serde_json::to_string(&cited.iter().map(|c| c.verse_id).collect::<Vec<_>>()))
        .transpose()
        .map_err(|err| AppError::new(AppErrorCode::QueryFailed, err.to_string()))?;
    let sources_json = sources
        .map(serde_json::to_string)
        .transpose()
        .map_err(|err| AppError::new(AppErrorCode::QueryFailed, err.to_string()))?;
    user.execute(
        &format!(
            "INSERT INTO chat_messages (session_id, role, content, cited_verses, sources_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, {})",
            crate::notes::NOW
        ),
        params![session_id, role.as_str(), content, cited_json, sources_json],
    )?;
    let id = u32::try_from(user.last_insert_rowid())
        .map_err(|_| AppError::new(AppErrorCode::QueryFailed, "message id out of range"))?;
    let created_at = user.query_row(
        "SELECT created_at FROM chat_messages WHERE id = ?1",
        [id],
        |row| row.get(0),
    )?;
    Ok(ChatMessage {
        id,
        role,
        content: content.to_owned(),
        cited: cited.to_vec(),
        sources: sources.cloned(),
        created_at,
    })
}

/// Every session, the most recently used first.
pub fn list_sessions(user: &Connection) -> AppResult<Vec<ChatSessionSummary>> {
    let mut stmt = user.prepare(&format!(
        "{SESSION_SELECT} ORDER BY updated_at DESC, s.id DESC"
    ))?;
    let rows = stmt.query_map([], session_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// A session with its messages; citations are resolved against `content`.
pub fn get_session(
    user: &Connection,
    content: &Connection,
    session_id: u32,
) -> AppResult<ChatSession> {
    let session = user
        .query_row(
            &format!("{SESSION_SELECT} WHERE s.id = ?1"),
            [session_id],
            session_row,
        )
        .optional()?
        .ok_or_else(|| {
            AppError::new(AppErrorCode::NotFound, format!("chat session {session_id}"))
        })?;
    let mut stmt = user.prepare(
        "SELECT id, role, content, cited_verses, sources_json, created_at FROM chat_messages
         WHERE session_id = ?1 ORDER BY id",
    )?;
    let rows: Vec<StoredMessage> = stmt
        .query_map([session_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    let mut verse_stmt = content.prepare("SELECT surah, ayah FROM verses WHERE id = ?1")?;
    let mut messages = Vec::with_capacity(rows.len());
    for (id, role, text, cited, sources, created_at) in rows {
        let ids: Vec<u32> = cited
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        let mut resolved = Vec::new();
        for verse_id in ids {
            if let Some((surah, ayah)) = verse_stmt
                .query_row([verse_id], |row| Ok((row.get(0)?, row.get(1)?)))
                .optional()?
            {
                resolved.push(VerseCitation {
                    verse_id,
                    verse: VerseRef { surah, ayah },
                });
            }
        }
        messages.push(ChatMessage {
            id,
            role: if role == "assistant" {
                Role::Assistant
            } else {
                Role::User
            },
            content: text,
            cited: resolved,
            sources: sources.and_then(|json| serde_json::from_str(&json).ok()),
            created_at,
        });
    }
    Ok(ChatSession { session, messages })
}

/// Deletes a session and its messages.
pub fn delete_session(user: &Connection, session_id: u32) -> AppResult<()> {
    user.execute(
        "DELETE FROM chat_messages WHERE session_id = ?1",
        [session_id],
    )?;
    user.execute("DELETE FROM chat_sessions WHERE id = ?1", [session_id])?;
    Ok(())
}

/// Renames a session.
pub fn rename_session(user: &Connection, session_id: u32, title: &str) -> AppResult<()> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(AppError::new(
            AppErrorCode::InvalidInput,
            "a session title is between 1 and 200 characters",
        ));
    }
    let changed = user.execute(
        "UPDATE chat_sessions SET title = ?1 WHERE id = ?2",
        params![title, session_id],
    )?;
    if changed == 0 {
        return Err(AppError::new(
            AppErrorCode::NotFound,
            format!("chat session {session_id}"),
        ));
    }
    Ok(())
}

/// A `chat_messages` row: id, role, content, cited verse ids, sources, time.
type StoredMessage = (u32, String, String, Option<String>, Option<String>, String);

const SESSION_SELECT: &str = "SELECT s.id, s.title, s.provider, s.model, s.created_at,
       COALESCE((SELECT MAX(m.created_at) FROM chat_messages m WHERE m.session_id = s.id), s.created_at)
         AS updated_at
     FROM chat_sessions s";

fn session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChatSessionSummary> {
    Ok(ChatSessionSummary {
        id: row.get(0)?,
        title: row.get(1)?,
        provider: row.get(2)?,
        model: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::test_support::{content, user};

    fn neighbors(ids: &[u32]) -> Vec<Neighbor> {
        ids.iter()
            .map(|&verse_id| Neighbor {
                verse_id,
                score: 0.5,
            })
            .collect()
    }

    #[test]
    fn tafsir_choice_defaults_dedupes_and_is_capped() {
        assert_eq!(chat_tafsirs(&Settings::new()), DEFAULT_CHAT_TAFSIRS);
        let settings: Settings = [(
            CHAT_TAFSIRS_SETTING.to_owned(),
            r#"["a.x","a.x","b.y","c.z","d.w"]"#.to_owned(),
        )]
        .into();
        assert_eq!(chat_tafsirs(&settings), ["a.x", "b.y", "c.z"]);
        assert!(check_message("  ").is_err());
        assert_eq!(check_message(" sabır ").unwrap(), "sabır");
    }

    #[test]
    fn builds_the_context_with_verses_translations_and_tafsir() {
        let conn = content();
        let verses =
            context_verses(&conn, &neighbors(&[2, 99, 1]), &["en.test".to_owned()]).unwrap();
        // Unknown verse 99 is skipped.
        assert_eq!(verses.len(), 2);
        assert_eq!(verses[0].verse, VerseRef { surah: 1, ayah: 2 });
        assert_eq!(
            verses[0].translations,
            [("Test Translation".to_owned(), "Praise".to_owned())]
        );

        let ids = vec!["ar.tafsir".to_owned(), "ar.missing".to_owned()];
        let (skipped, notices) = tafsir_candidates(&conn, &verses, &ids, false).unwrap();
        assert!(skipped.is_empty());
        assert_eq!(
            notices,
            [
                ChatNotice::ArabicTafsirsSkipped,
                ChatNotice::TafsirMissing {
                    pack_id: "ar.missing".into()
                }
            ]
        );
        let (candidates, _) = tafsir_candidates(&conn, &verses, &ids, true).unwrap();
        // One passage covers both verses: it is added once.
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].budget, TAFSIR_BUDGET / 2);

        let fitted = fit_tafsirs(candidates);
        assert!(fitted[0].source.complete);
        let context = Context {
            verses,
            tafsirs: fitted,
            notices,
            search_query: Some("hamd".into()),
        };
        let prompt = context.system_prompt("Turkish");
        assert!(prompt.contains("Answer in Turkish"));
        assert!(prompt.contains("[1:2] الحمد\nTest Translation: Praise"));
        assert!(prompt.contains("### Tafsir Test on [1:1]–[1:2]\nBu **ayet** & devamı"));
        assert!(prompt.contains("original Arabic wording"));
        assert!(!prompt.contains("[…]"));
        let sources = context.sources();
        assert_eq!(sources.verses.len(), 2);
        assert_eq!(sources.tafsirs[0].paragraphs, [0, 1]);
        assert_eq!(sources.search_query.as_deref(), Some("hamd"));
    }

    #[test]
    fn a_long_passage_keeps_its_opening_and_marks_the_rest() {
        let long = "word ".repeat(400);
        let candidate = Candidate {
            source: SourceTafsir {
                pack_id: "ar.t".into(),
                title: "T".into(),
                tafsir_id: 1,
                verse_from: VerseRef { surah: 1, ayah: 1 },
                verse_to: VerseRef { surah: 1, ayah: 1 },
                paragraphs: vec![0, 1, 2, 3],
                complete: true,
            },
            paragraphs: (0..4).map(|i| (i, long.clone())).collect(),
            budget: 1200,
            arabic: false,
        };
        let fitted = fit_tafsirs(vec![candidate]);
        let kept = &fitted[0];
        // Each paragraph is about 572 tokens: the first two fit in 1200.
        assert_eq!(kept.source.paragraphs, [0, 1]);
        assert_eq!(kept.paragraphs.len(), 2);
        assert!(!kept.source.complete);
        let prompt = Context {
            tafsirs: fitted,
            ..Context::default()
        }
        .system_prompt("English");
        assert!(prompt.contains("[…]"));
        assert!(prompt.contains("(no verses were found)"));
    }

    #[test]
    fn the_search_query_request_carries_the_end_of_the_conversation() {
        let first = check_message("Bu ilk soru").unwrap();
        let mut history: Vec<Message> = (0..6)
            .map(|i| Message {
                role: if i % 2 == 0 {
                    Role::User
                } else {
                    Role::Assistant
                },
                content: format!("message {i}"),
            })
            .collect();
        history[0].content = first.to_owned();
        history[5].content = "x".repeat(3000);
        let request = search_query_request("m", &history, "peki kızların payı?");
        assert_eq!(request.model, "m");
        assert_eq!(request.max_tokens, QUERY_MAX_TOKENS);
        let [only] = request.messages.as_slice() else {
            panic!("one message: {:?}", request.messages);
        };
        assert_eq!(only.role, Role::User);
        // The last four messages, each cut, then the new one.
        assert!(!only.content.contains("Bu ilk soru"));
        assert!(
            only.content
                .contains("User: message 2\nAssistant: message 3\n")
        );
        assert!(!only.content.contains(&"x".repeat(QUERY_HISTORY_CHARS + 1)));
        assert!(only.content.ends_with("Last message: peki kızların payı?"));

        let alone = search_query_request("m", &[], "İslam'da miras");
        assert_eq!(alone.messages[0].content, "Last message: İslam'da miras");
    }

    #[test]
    fn the_search_query_is_the_first_line_without_quotes_or_thinking() {
        assert_eq!(
            parse_search_query("\n  \"Kur'an'da miras hükümleri\"  \nExplanation: …").as_deref(),
            Some("Kur'an'da miras hükümleri")
        );
        assert_eq!(
            parse_search_query("<think>miras...\nhmm</think>\n**Mirasın paylaştırılması**")
                .as_deref(),
            Some("Mirasın paylaştırılması")
        );
        assert_eq!(parse_search_query(" \n \"\" \n"), None);
        let long = "a".repeat(1000);
        assert_eq!(
            parse_search_query(&long).map(|q| q.chars().count()),
            Some(MAX_QUERY_CHARS)
        );
    }

    #[test]
    fn a_first_paragraph_over_the_budget_is_cut_to_fit() {
        let huge = "كلمة ".repeat(2000);
        let candidate = Candidate {
            source: SourceTafsir {
                pack_id: "ar.t".into(),
                title: "T".into(),
                tafsir_id: 1,
                verse_from: VerseRef { surah: 1, ayah: 1 },
                verse_to: VerseRef { surah: 1, ayah: 1 },
                paragraphs: vec![0, 1],
                complete: true,
            },
            paragraphs: vec![(0, huge), (1, "تتمة".into())],
            budget: 500,
            arabic: true,
        };
        let fitted = fit_tafsirs(vec![candidate]);
        let kept = &fitted[0];
        assert_eq!(kept.source.paragraphs, [0]);
        assert!(!kept.source.complete);
        let text = &kept.paragraphs[0].1;
        assert!(estimate_tokens(text) <= 500);
        assert!(estimate_tokens(text) > 450, "{}", estimate_tokens(text));
        assert!(text.ends_with("كلمة"));
    }

    #[test]
    fn citations_are_parsed_in_order_and_checked() {
        let conn = content();
        let cited = citations(
            &conn,
            "Patience [1:2] and [1:1-2], also [1:1, 2:1]; not [9:9], [x:1] or [1:2",
        )
        .unwrap();
        let refs: Vec<(u16, u16)> = cited
            .iter()
            .map(|c| (c.verse.surah, c.verse.ayah))
            .collect();
        assert_eq!(refs, [(1, 2), (1, 1), (2, 1)]);
    }

    #[test]
    fn sessions_store_messages_with_citations_and_sources() {
        let content = content();
        let user = user();
        let id = create_session(&user, "  What is patience? ", "anthropic", "m").unwrap();
        add_message(&user, id, Role::User, "What is patience?", &[], None).unwrap();
        let cited = citations(&content, "[1:2]").unwrap();
        let sources = ChatSources {
            verses: vec![],
            tafsirs: vec![],
            notices: vec![ChatNotice::ArabicTafsirsSkipped],
            search_query: Some("sabır".into()),
        };
        add_message(
            &user,
            id,
            Role::Assistant,
            "See [1:2].",
            &cited,
            Some(&sources),
        )
        .unwrap();

        let session = get_session(&user, &content, id).unwrap();
        assert_eq!(session.session.title.as_deref(), Some("What is patience?"));
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[1].cited, cited);
        assert_eq!(session.messages[1].sources, Some(sources));
        let past = history(&user, id).unwrap();
        assert_eq!(past[1].role, Role::Assistant);

        rename_session(&user, id, "Sabır").unwrap();
        assert_eq!(
            list_sessions(&user).unwrap()[0].title.as_deref(),
            Some("Sabır")
        );
        assert_eq!(
            rename_session(&user, 99, "x").unwrap_err().code,
            AppErrorCode::NotFound
        );
        delete_session(&user, id).unwrap();
        assert!(list_sessions(&user).unwrap().is_empty());
        assert_eq!(
            require_session(&user, id).unwrap_err().code,
            AppErrorCode::NotFound
        );
    }
}
