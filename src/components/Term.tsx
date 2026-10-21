import { useTranslation } from "react-i18next";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { termLabel, type TermName } from "@/i18n/terms";
import { useUiStore } from "@/stores/ui";

/**
 * A term in running text. With technical terms in tooltips (the reader
 * default) the technical form is one hover or focus away.
 */
export function Term({ name }: { name: TermName }) {
  const { t } = useTranslation();
  const mode = useUiStore((state) => state.preferences["ui.technical_terms"]);
  const label = termLabel(t, name, mode);
  const technical = t(`term.${name}.technical`);
  if (mode === "inline" || label === technical) return <span>{label}</span>;
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span tabIndex={0} className="underline decoration-dotted underline-offset-4">
          {label}
        </span>
      </TooltipTrigger>
      <TooltipContent>{technical}</TooltipContent>
    </Tooltip>
  );
}
