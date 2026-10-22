import { useAnnouncement } from "@/lib/announce";

/**
 * The app's one polite live region; `announce()` puts messages here. Mounted
 * once, in the shell.
 */
export function LiveAnnouncer() {
  const { message, id } = useAnnouncement();
  return (
    <div role="status" aria-live="polite" aria-atomic className="sr-only">
      <span key={id}>{message}</span>
    </div>
  );
}
