import { Link } from "@tanstack/react-router";
import { BookmarkXIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { AddToCollectionButton } from "@/features/collections/AddToCollectionButton";
import { namedRange } from "@/features/collections/itemLabels";
import { VerseText } from "@/features/collections/VerseText";
import { useBookmarks, useSurahs, useToggleBookmark } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

/** All bookmarked verses, the newest first. */
export function BookmarksView() {
  const { t, i18n } = useTranslation();
  const translations = useUiStore((state) => state.preferences["reader.translations"]);
  const bookmarks = useBookmarks(translations);
  const surahs = useSurahs();
  const toggle = useToggleBookmark();
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto grid max-w-3xl gap-4 p-8">
        <h1 className="text-lg font-semibold">{t("bookmarks.title")}</h1>
        {bookmarks.isError && <ErrorMessage error={bookmarks.error} />}
        {bookmarks.data?.length === 0 && (
          <p className="text-muted-foreground">{t("bookmarks.empty")}</p>
        )}
        <ul className="grid gap-3">
          {bookmarks.data?.map(({ verse }) => {
            const position = { surah: verse.surah, ayah: verse.ayah };
            return (
              <li key={verse.id} className="grid gap-2 rounded-lg border p-3">
                <Link
                  to="/read/$surah/$ayah"
                  params={position}
                  className="text-sm font-medium underline-offset-2 hover:underline"
                >
                  {namedRange(surahs.data, position, position, i18n.resolvedLanguage)}
                </Link>
                <VerseText verse={verse} />
                <div className="-ms-2 flex flex-wrap gap-1">
                  <AddToCollectionButton
                    item={{ kind: "verse_range", from: verse.id, to: verse.id }}
                  />
                  <Button
                    variant="ghost"
                    size="xs"
                    className="text-muted-foreground"
                    onClick={() => {
                      toggle.mutate(verse.id);
                    }}
                  >
                    <BookmarkXIcon aria-hidden />
                    {t("verse_actions.remove_bookmark")}
                  </Button>
                </div>
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}
