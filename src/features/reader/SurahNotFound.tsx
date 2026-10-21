import { useTranslation } from "react-i18next";

/** Shown for `/read/:surah` with a number outside 1..114, or an unknown path. */
export function SurahNotFound() {
  const { t } = useTranslation();
  return <p className="p-8 text-muted-foreground">{t("reader.surah_not_found")}</p>;
}
