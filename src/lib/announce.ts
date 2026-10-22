import { create } from "zustand";

interface Announcement {
  message: string;
  /** Changes with every announcement, so a repeated message is read again. */
  id: number;
}

/** The latest message for screen readers, shown by `LiveAnnouncer`. */
export const useAnnouncement = create<Announcement>()(() => ({ message: "", id: 0 }));

/** Announces `message` politely to screen readers (e.g. "Copied"). */
export function announce(message: string) {
  useAnnouncement.setState((state) => ({ message, id: state.id + 1 }));
}
