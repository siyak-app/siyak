import { CheckIcon, QuoteIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { announce } from "@/lib/announce";
import type { CitationRef } from "@/lib/bindings";
import { fetchCitation } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

/** How long the button shows its check mark. */
const COPIED_MS = 1500;

/** Copies the citation text of `reference` in the chosen style. */
export function CopyCitationButton({ reference }: { reference: CitationRef }) {
  const { t } = useTranslation();
  const style = useUiStore((state) => state.preferences["reader.citation_style"]);
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => {
      setCopied(false);
    }, COPIED_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [copied]);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(await fetchCitation(reference, style));
      setCopied(true);
      announce(t("verse_actions.copied"));
    } catch (error) {
      console.error("Copying the citation failed", error);
      announce(t("verse_actions.copy_failed"));
    }
  };
  const Icon = copied ? CheckIcon : QuoteIcon;
  return (
    <Button variant="ghost" size="xs" className="text-muted-foreground" onClick={() => void copy()}>
      <Icon aria-hidden />
      {t("verse_actions.copy_citation")}
    </Button>
  );
}
