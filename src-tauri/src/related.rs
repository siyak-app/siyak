//! Cross-references: related verses from semantic similarity and IDF-weighted
//! shared roots, each shown with its reason (spec §7.6).
//!
//! **Shared roots.** A verse is described by the roots of its words, and by
//! the dictionary forms of proper nouns that have no root (موسى, فرعون).
//! Each such term is weighted by its inverse verse frequency (IDF); terms in
//! more than a tenth of all verses (ا ل ه, ق و ل, ك و ن …) are left out, so
//! they neither rank nor appear as a reason. Two verses score the cosine of
//! their weighted term sets, which keeps long verses from winning by size.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use specta::Type;

use crate::db::content::VerseTranslation;
use crate::error::AppResult;
use crate::morphology::ConcordanceTarget;

/// Terms in more than this share of all verses are left out.
const MAX_SHARE: f64 = 0.1;

/// Largest number of related verses returned at once.
pub const MAX_RELATED: u32 = 50;

/// A root or dictionary form two verses share.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct SharedTerm {
    /// Its concordance.
    pub target: ConcordanceTarget,
    /// The root (`ر ح م`) or dictionary form, in Arabic letters.
    pub text: String,
}

/// A verse found by meaning or by shared roots.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct RelatedVerse {
    /// Surah number.
    pub surah: u16,
    /// Verse number within the surah.
    pub ayah: u16,
    /// Uthmani script.
    pub text_ar: String,
    /// The requested translation, if the verse has one.
    pub translation: Option<VerseTranslation>,
    /// The shared roots and dictionary forms, rarest first; empty for verses
    /// related by meaning.
    pub shared: Vec<SharedTerm>,
}

/// One term of the index.
#[derive(Debug, Clone, PartialEq)]
struct Term {
    target: ConcordanceTarget,
    text: String,
    /// Squared IDF: the term's share of a cosine.
    weight: f64,
}

/// The weighted terms of every verse, built once per morphology pack.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RootIndex {
    terms: Vec<Term>,
    /// Term indexes of each verse, sorted.
    verses: HashMap<u32, Vec<usize>>,
    /// Length of each verse's weighted term vector.
    norms: HashMap<u32, f64>,
}

/// A verse related by shared roots.
#[derive(Debug, Clone, PartialEq)]
pub struct RootMatch {
    /// The related verse.
    pub verse_id: u32,
    /// Cosine of the weighted term sets, 0 to 1.
    pub score: f64,
    /// The shared terms, rarest first.
    pub shared: Vec<SharedTerm>,
}

impl RootIndex {
    /// Reads the terms of every verse from the installed morphology pack.
    pub fn load(conn: &Connection) -> AppResult<Self> {
        let (rows, verse_count) = term_rows(conn)?;
        Ok(Self::build(rows, verse_count, MAX_SHARE))
    }

    /// Builds the index from `(verse, term, text)` rows. Terms in more than
    /// `max_share` of the `verse_count` verses are left out.
    pub fn build(rows: Vec<TermRow>, verse_count: u32, max_share: f64) -> Self {
        let mut index_of: HashMap<ConcordanceTarget, usize> = HashMap::new();
        let mut texts: Vec<(ConcordanceTarget, String)> = Vec::new();
        let mut verses: HashMap<u32, Vec<usize>> = HashMap::new();
        for (verse, target, text) in rows {
            let term = *index_of.entry(target).or_insert_with(|| {
                texts.push((target, text));
                texts.len() - 1
            });
            verses.entry(verse).or_default().push(term);
        }
        let mut frequency = vec![0_u32; texts.len()];
        for terms in verses.values_mut() {
            terms.sort_unstable();
            terms.dedup();
            for &term in terms.iter() {
                frequency[term] += 1;
            }
        }
        let total = f64::from(verse_count.max(1));
        let terms: Vec<Term> = texts
            .into_iter()
            .zip(frequency)
            .map(|((target, text), count)| {
                let count = f64::from(count);
                let weight = if count > total * max_share {
                    0.0
                } else {
                    (total / count).ln().powi(2)
                };
                Term {
                    target,
                    text,
                    weight,
                }
            })
            .collect();
        for list in verses.values_mut() {
            list.retain(|&term| terms[term].weight > 0.0);
        }
        verses.retain(|_, list| !list.is_empty());
        let norms = verses
            .iter()
            .map(|(&verse, list)| {
                (
                    verse,
                    list.iter().map(|&t| terms[t].weight).sum::<f64>().sqrt(),
                )
            })
            .collect();
        Self {
            terms,
            verses,
            norms,
        }
    }

    /// The `k` verses that share the most weight of terms with `verse_id`,
    /// best first; the verse itself is left out.
    pub fn related(&self, verse_id: u32, k: usize) -> Vec<RootMatch> {
        let (Some(own), Some(&own_norm)) = (self.verses.get(&verse_id), self.norms.get(&verse_id))
        else {
            return Vec::new();
        };
        let mut matches: Vec<(f64, u32, Vec<usize>)> = self
            .verses
            .iter()
            .filter(|(other, _)| **other != verse_id)
            .filter_map(|(&other, terms)| {
                let shared = intersect(own, terms);
                if shared.is_empty() {
                    return None;
                }
                let dot: f64 = shared.iter().map(|&t| self.terms[t].weight).sum();
                Some((dot / (own_norm * self.norms[&other]), other, shared))
            })
            .collect();
        matches.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        matches.truncate(k);
        matches
            .into_iter()
            .map(|(score, verse_id, mut shared)| {
                shared.sort_by(|&a, &b| {
                    self.terms[b]
                        .weight
                        .total_cmp(&self.terms[a].weight)
                        .then(a.cmp(&b))
                });
                RootMatch {
                    verse_id,
                    score,
                    shared: shared
                        .into_iter()
                        .map(|t| SharedTerm {
                            target: self.terms[t].target,
                            text: self.terms[t].text.clone(),
                        })
                        .collect(),
                }
            })
            .collect()
    }
}

/// A verse, one of its terms and the term's text.
type TermRow = (u32, ConcordanceTarget, String);

/// The terms of every verse, and the number of verses.
fn term_rows(conn: &Connection) -> AppResult<(Vec<TermRow>, u32)> {
    let verse_count: u32 = conn.query_row("SELECT count(*) FROM verses", [], |r| r.get(0))?;
    let mut stmt = conn.prepare(
        "SELECT DISTINCT w.verse_id,
                CASE WHEN s.root_id IS NOT NULL THEN 'root' ELSE 'lemma' END,
                coalesce(s.root_id, s.lemma_id),
                coalesce(r.root, l.lemma)
         FROM word_segments s
         JOIN words w ON w.id = s.word_id
         LEFT JOIN roots r ON r.id = s.root_id
         LEFT JOIN lemmas l ON l.id = s.lemma_id
         WHERE s.root_id IS NOT NULL OR (s.lemma_id IS NOT NULL AND s.pos = 'PN')",
    )?;
    let rows = stmt
        .query_map([], |row| {
            let id = row.get(2)?;
            let kind: String = row.get(1)?;
            let target = if kind == "root" {
                ConcordanceTarget::Root { id }
            } else {
                ConcordanceTarget::Lemma { id }
            };
            Ok((row.get::<_, u32>(0)?, target, row.get::<_, String>(3)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((rows, verse_count))
}

/// The common entries of two sorted lists.
fn intersect(a: &[usize], b: &[usize]) -> Vec<usize> {
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
        }
    }
    out
}

/// Keeps the [`RootIndex`] of the installed morphology pack.
#[derive(Default)]
pub struct Related {
    /// The index, keyed by the pack's id, version and install time.
    index: Mutex<Option<(String, Arc<RootIndex>)>>,
}

impl Related {
    /// The index of the installed morphology pack, built on first use and
    /// kept until the pack changes; `None` without a morphology pack.
    pub fn index(&self, conn: &Connection) -> AppResult<Option<Arc<RootIndex>>> {
        let key: Option<String> = conn
            .query_row(
                "SELECT id || '@' || version || '@' || installed_at FROM packs
                 WHERE type = 'morphology' ORDER BY id LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let Some(key) = key else {
            return Ok(None);
        };
        let mut cache = self
            .index
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((cached, index)) = cache.as_ref()
            && *cached == key
        {
            return Ok(Some(Arc::clone(index)));
        }
        let index = Arc::new(RootIndex::load(conn)?);
        *cache = Some((key, Arc::clone(&index)));
        Ok(Some(index))
    }
}

/// The verses, in the given order, with their Arabic text and the requested
/// translation; `shared` is paired with each verse (or empty).
pub fn verse_cards(
    conn: &Connection,
    verses: Vec<(u32, Vec<SharedTerm>)>,
    translation_id: Option<&str>,
) -> AppResult<Vec<RelatedVerse>> {
    let mut verse = conn.prepare("SELECT surah, ayah, text_uthmani FROM verses WHERE id = ?1")?;
    let mut translation = conn.prepare(
        "SELECT p.lang, t.text FROM translations t JOIN packs p ON p.id = t.pack_id
         WHERE t.pack_id = ?1 AND t.verse_id = ?2",
    )?;
    let mut out = Vec::with_capacity(verses.len());
    for (verse_id, shared) in verses {
        let Some((surah, ayah, text_ar)) = verse
            .query_row([verse_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .optional()?
        else {
            continue;
        };
        let translation = match translation_id {
            Some(pack_id) => translation
                .query_row((pack_id, verse_id), |row| {
                    Ok(VerseTranslation {
                        pack_id: pack_id.to_owned(),
                        lang: row.get(0)?,
                        text: row.get(1)?,
                    })
                })
                .optional()?,
            None => None,
        };
        out.push(RelatedVerse {
            surah,
            ayah,
            text_ar,
            translation,
            shared,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::content::MIGRATIONS;
    use crate::db::migrate;

    fn root(id: u32) -> ConcordanceTarget {
        ConcordanceTarget::Root { id }
    }

    fn rows(list: &[(u32, u32)]) -> Vec<TermRow> {
        list.iter()
            .map(|&(verse, term)| (verse, root(term), format!("r{term}")))
            .collect()
    }

    fn texts(found: &RootMatch) -> Vec<&str> {
        found.shared.iter().map(|t| t.text.as_str()).collect()
    }

    #[test]
    fn rare_shared_roots_rank_first() {
        // Root 1 is everywhere, root 2 in two verses, root 3 in three.
        let index = RootIndex::build(
            rows(&[
                (1, 1),
                (1, 2),
                (1, 3),
                (2, 1),
                (2, 2),
                (3, 1),
                (3, 3),
                (4, 1),
                (4, 3),
                (5, 1),
            ]),
            10,
            1.0,
        );
        let found = index.related(1, 10);
        let ids: Vec<u32> = found.iter().map(|m| m.verse_id).collect();
        assert_eq!(ids, [2, 3, 4, 5]);
        assert_eq!(texts(&found[0]), ["r2", "r1"]);
        assert!(found.windows(2).all(|w| w[0].score >= w[1].score));
        assert!(found.iter().all(|m| m.score > 0.0 && m.score <= 1.0 + 1e-9));
        assert_eq!(index.related(1, 2).len(), 2);
        assert!(index.related(99, 10).is_empty());
    }

    #[test]
    fn frequent_roots_are_left_out() {
        // Root 1 is in 3 of 10 verses: above a 20% share.
        let index = RootIndex::build(rows(&[(1, 1), (1, 2), (2, 1), (3, 1), (4, 2)]), 10, 0.2);
        let found = index.related(1, 10);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].verse_id, 4);
        assert_eq!(texts(&found[0]), ["r2"]);
        // Verse 2 has only the frequent root: nothing relates to it.
        assert!(index.related(2, 10).is_empty());
    }

    #[test]
    fn a_short_verse_is_not_beaten_by_a_long_one() {
        // Verse 2 shares root 2 and nothing else; verse 3 shares it among many.
        let mut list = vec![(1, 2), (2, 2), (3, 2)];
        list.extend((10..20).map(|t| (3, t)));
        let index = RootIndex::build(rows(&list), 100, 1.0);
        let ids: Vec<u32> = index.related(1, 10).iter().map(|m| m.verse_id).collect();
        assert_eq!(ids, [2, 3]);
    }

    fn database() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrate(&mut conn, MIGRATIONS).unwrap();
        conn.execute_batch(
            "INSERT INTO surahs (number, name_ar, name_tr, name_en, revelation_type, ayah_count)
               VALUES (1, 'a', 'a', 'a', 'meccan', 3);
             INSERT INTO verses (id, surah, ayah, juz, text_uthmani, text_clean) VALUES
               (1, 1, 1, 1, 'x', 'x'), (2, 1, 2, 1, 'y', 'y'), (3, 1, 3, 1, 'z', 'z');
             INSERT INTO packs (id, type, title, version, license, attribution, installed_at, lang)
               VALUES ('morphology.qac', 'morphology', 'QAC', '1.0.0', 'GPL', 'QAC', 'now', NULL),
                      ('en.test', 'translation', 'Test', '1.0.0', 'PD', NULL, 'now', 'en');
             INSERT INTO translations (pack_id, verse_id, text) VALUES ('en.test', 2, 'Moses');
             INSERT INTO roots (id, root) VALUES (1, 'ر ح م'), (2, 'ق و ل');
             INSERT INTO lemmas (id, lemma, root_id) VALUES
               (1, 'رَحْمَة', 1), (2, 'مُوسَىٰ', NULL), (3, 'مِن', NULL), (4, 'قَالَ', 2);
             INSERT INTO words (id, verse_id, position, text_ar) VALUES
               (1, 1, 1, 'a'), (2, 1, 2, 'b'), (3, 1, 3, 'c'),
               (4, 2, 1, 'd'), (5, 2, 2, 'e'), (6, 3, 1, 'f');
             INSERT INTO word_segments
               (word_id, idx, kind, text_ar, pos, lemma_id, root_id, features) VALUES
               (1, 1, 'stem', 'a', 'N', 1, 1, NULL),
               (2, 1, 'stem', 'b', 'PN', 2, NULL, NULL),
               (3, 1, 'stem', 'c', 'P', 3, NULL, NULL),
               (4, 1, 'stem', 'd', 'PN', 2, NULL, NULL),
               (5, 1, 'stem', 'e', 'P', 3, NULL, NULL),
               (6, 1, 'stem', 'f', 'N', 1, 1, NULL);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn proper_nouns_count_and_particles_do_not() {
        let conn = database();
        // Every term is in a third of the 3 verses, above the 10% share.
        let index = Related::default().index(&conn).unwrap().unwrap();
        assert!(index.related(1, 10).is_empty());
        let (rows, verse_count) = term_rows(&conn).unwrap();
        assert_eq!(verse_count, 3);
        let index = RootIndex::build(rows, verse_count, 1.0);
        assert_eq!(index.terms.len(), 2, "the root and the proper noun, not مِن");
        let found = index.related(1, 10);
        let ids: Vec<u32> = found.iter().map(|m| m.verse_id).collect();
        assert_eq!(ids, [2, 3]);
        assert_eq!(texts(&found[0]), ["مُوسَىٰ"]);
        assert_eq!(
            found[0].shared[0].target,
            ConcordanceTarget::Lemma { id: 2 }
        );
        assert_eq!(texts(&found[1]), ["ر ح م"]);
    }

    #[test]
    fn the_index_is_kept_until_the_pack_changes() {
        let conn = database();
        let related = Related::default();
        let first = related.index(&conn).unwrap().unwrap();
        assert!(Arc::ptr_eq(&first, &related.index(&conn).unwrap().unwrap()));
        conn.execute(
            "UPDATE packs SET installed_at = 'later' WHERE id = 'morphology.qac'",
            [],
        )
        .unwrap();
        assert!(!Arc::ptr_eq(
            &first,
            &related.index(&conn).unwrap().unwrap()
        ));
        conn.execute("DELETE FROM packs WHERE id = 'morphology.qac'", [])
            .unwrap();
        assert!(related.index(&conn).unwrap().is_none());
    }

    #[test]
    fn cards_carry_text_translation_and_reasons() {
        let conn = database();
        let shared = vec![SharedTerm {
            target: ConcordanceTarget::Lemma { id: 2 },
            text: "مُوسَىٰ".into(),
        }];
        let cards = verse_cards(
            &conn,
            vec![(2, shared.clone()), (99, vec![]), (3, vec![])],
            Some("en.test"),
        )
        .unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(
            (cards[0].surah, cards[0].ayah, cards[0].text_ar.as_str()),
            (1, 2, "y")
        );
        assert_eq!(cards[0].translation.as_ref().unwrap().text, "Moses");
        assert_eq!(cards[0].shared, shared);
        assert!(cards[1].translation.is_none());
        assert!(
            verse_cards(&conn, vec![(2, vec![])], None).unwrap()[0]
                .translation
                .is_none()
        );
    }
}
