/** `[2:153]` or `[2:153-157]`: a verse citation (spec §10.5). */
const CITATION = /\[(\d{1,3}):(\d{1,3})(?:[-–]\d{1,3})?\]/g;

/** The prefix of the links citations are turned into. */
export const VERSE_SCHEME = "verse:";

/** Turns citations into Markdown links the renderer opens in the reader. */
export function linkCitations(text: string): string {
  return text.replace(CITATION, (match, surah: string, ayah: string) => {
    const label = match.slice(1, -1);
    return `[${label}](${VERSE_SCHEME}${surah}:${ayah})`;
  });
}
