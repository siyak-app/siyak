/**
 * Folding of surah names and user input for matching, so that `bakara`,
 * `Baqarah`, `Al-Baqara` and `البقره` all find al-Baqarah.
 *
 * This is for matching names in the interface only. Arabic search uses the
 * normalizer shared with the data pipeline (M4).
 *
 * @packageDocumentation
 */

/** Combining marks: Latin accents after NFD, and Arabic harakat. */
const MARKS = /\p{M}/gu;
/** Leading Arabic article of English transliterations: `Al-`, `An-`, `Ash-`… */
const ARTICLE = /^(al|an|ar|as|ash|at|ad|adh|az)[-\s'’]+/u;
/** Everything except letters and digits. */
const SEPARATORS = /[^\p{L}\p{N}]/gu;

const ARABIC_LETTERS: Record<string, string> = {
  أ: "ا",
  إ: "ا",
  آ: "ا",
  ٱ: "ا",
  ى: "ي",
  ة: "ه",
  ـ: "",
};

/**
 * Folds `text` to a matching key: lower case, without accents, harakat,
 * punctuation or spaces; Turkish dotless ı as i; Arabic alef variants as ا,
 * alef maqsura as ي and teh marbuta as ه.
 */
export function fold(text: string): string {
  return text
    .normalize("NFD")
    .replace(MARKS, "")
    .toLowerCase()
    .replaceAll("ı", "i")
    .replace(/[أإآٱىةـ]/gu, (letter) => ARABIC_LETTERS[letter] ?? letter)
    .replace(SEPARATORS, "");
}

/**
 * Folds a Latin-script surah name further, so that different
 * transliterations meet: the article is dropped, doubled vowels are
 * collapsed (`Faatiha`, `Yaseen`) and a final `h` after a vowel is dropped
 * (`Baqarah`). `Al-Baqara`, `Baqarah` and `baqara` all become `baqara`.
 */
export function foldName(name: string): string {
  const stripped = name.normalize("NFD").replace(MARKS, "").toLowerCase().replace(ARTICLE, "");
  return fold(stripped)
    .replace(/ee/gu, "i")
    .replace(/([aiu])\1+/gu, "$1")
    .replace(/([aeiou])h$/u, "$1");
}
