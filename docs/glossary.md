# Glossary

Terms used in code, documentation and the interface. **Code always uses the English code name**
(`surah`, `verse`, `translation`, `tafsir`, …); Turkish words such as `sure`, `ayet` or `meal` never
appear in identifiers (ADR 0009). Turkish and English interface wording lives in
`src/i18n/{tr,en}.json`; where a term has a plain and a technical form, both are kept
(`term.<name>.plain` / `term.<name>.technical`, spec §13.2).

| Code name              | Turkish                      | Arabic         | Definition                                                                                                                                        |
| ---------------------- | ---------------------------- | -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `surah`                | sure                         | سورة           | One of the 114 chapters of the Quran, numbered 1–114.                                                                                             |
| `verse` (also `ayah`)  | ayet                         | آية            | A verse within a surah. `verse` is the entity (global id 1–6236); `ayah` is the verse number within its surah, as in `surah:ayah` (e.g. `2:255`). |
| `juz`                  | cüz                          | جزء            | One of 30 parts of roughly equal length into which the Quran is divided for reading.                                                              |
| `basmala`              | besmele                      | البسملة        | The formula "In the name of God…" at the start of every surah except the ninth; part of verse 1 only in surah 1.                                  |
| `meccan` / `medinan`   | Mekkî / Medenî               | مكي / مدني     | Whether a surah was revealed before or after the Hijra (`surahs.revelation_type`).                                                                |
| `text_uthmani`         | Osmanî hat (Mushaf yazımı)   | الرسم العثماني | Arabic text in the orthography of the Madina Mushaf, used for display.                                                                            |
| `text_clean`           | harekesiz metin              | نص بلا تشكيل   | Arabic text without diacritics or marks, used for search.                                                                                         |
| `translation`          | meal                         | ترجمة          | A rendering of the meaning of the verses in another language; one translation pack per work.                                                      |
| `tafsir`               | tefsir                       | تفسير          | A commentary explaining verses; stored as passages covering a verse range and reached through a verse (ADR 0003).                                 |
| `root`                 | kök                          | جذر            | The (usually three) consonants that carry a word's core meaning, written spaced (e.g. ر ح م).                                                     |
| `lemma`                | sözlük biçimi (lemma)        | المدخل المعجمي | The dictionary form under which inflected words are grouped. Plain wording: "dictionary form".                                                    |
| `segment`              | parça (ön ek, gövde, son ek) | مقطع           | A morphological part of a word: prefix, stem or suffix, each with its own part-of-speech tag (spec §5.3).                                         |
| `concordance`          | geçtiği yerler (konkordans)  | فهرس المواضع   | Every occurrence of a root or lemma, with counts per surah. Plain wording: "where it occurs".                                                     |
| `pack`                 | paket                        | —              | A downloadable unit of content (translation, tafsir, morphology, vectors or model) with its own version and license (spec §6).                    |
| `bundle`               | paket seti                   | —              | A named set of packs installed with one click, e.g. "Recommended packs" (spec §6.3).                                                              |
| `experience_profile`   | kullanım profili             | —              | The first-launch answer (`reader` or `researcher`) that only sets defaults (spec §13.2).                                                          |
| progressive disclosure | kademeli gösterim            | —              | The interface principle: screens open simple and show depth when the user asks for it (ADR 0005).                                                 |
