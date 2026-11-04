import { Link } from "@tanstack/react-router";
import { BookmarkIcon, FolderIcon, type LucideIcon, NotebookPenIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

const ACTIVE_LINK_PROPS = { className: "bg-primary/10 hover:bg-primary/15" };

/** The left pane's way to what the user has kept (spec §13.2). */
export function LibraryNav() {
  const { t } = useTranslation();
  const links: { to: "/collections" | "/bookmarks" | "/notes"; icon: LucideIcon; label: string }[] =
    [
      { to: "/collections", icon: FolderIcon, label: t("navigation.collections") },
      { to: "/bookmarks", icon: BookmarkIcon, label: t("navigation.bookmarks") },
      { to: "/notes", icon: NotebookPenIcon, label: t("navigation.notes") },
    ];
  return (
    <nav aria-label={t("navigation.library")} className="border-b px-2 py-2">
      <ul className="grid gap-0.5">
        {links.map(({ to, icon: Icon, label }) => (
          <li key={to}>
            <Link
              to={to}
              className="flex items-center gap-3 rounded-md px-2 py-1.5 text-sm hover:bg-muted"
              activeProps={ACTIVE_LINK_PROPS}
            >
              <Icon aria-hidden className="size-4 text-muted-foreground" />
              {label}
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  );
}
