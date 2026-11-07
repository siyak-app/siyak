import { cn } from "cn";
import { BookOpenIcon, MicroscopeIcon } from "lucide-react";
import { type KeyboardEvent, useRef } from "react";
import { useTranslation } from "react-i18next";

import type { ExperienceProfile } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { useApplyProfile } from "./useApplyProfile";

const MODES = [
  { value: "reader", icon: BookOpenIcon },
  { value: "researcher", icon: MicroscopeIcon },
] as const satisfies readonly { value: ExperienceProfile; icon: unknown }[];

/**
 * Reading or research mode, in the header: the experience profile (spec
 * §13.2) as the app's main switch. Switching applies the mode's defaults but
 * keeps the chosen translations. Before the first-launch question is
 * answered the app reads in reading mode.
 */
export function ModeToggle() {
  const { t } = useTranslation();
  const profile = useUiStore((state) => state.preferences.experience_profile) ?? "reader";
  const { apply, busy } = useApplyProfile();
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);

  const choose = (mode: ExperienceProfile) => {
    if (mode !== profile) apply(mode, undefined, { keepTranslations: true });
  };

  // A radio group: arrows move between the two modes and choose.
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) return;
    event.preventDefault();
    const next = MODES.findIndex((mode) => mode.value !== profile);
    const mode = MODES[next];
    if (!mode) return;
    choose(mode.value);
    buttons.current[next]?.focus();
  };

  return (
    <div
      role="radiogroup"
      aria-label={t("mode.label")}
      onKeyDown={onKeyDown}
      className="flex items-center gap-0.5 rounded-full border bg-muted/60 p-0.5"
    >
      {MODES.map(({ value, icon: Icon }, index) => {
        const checked = value === profile;
        return (
          <button
            key={value}
            ref={(element) => {
              buttons.current[index] = element;
            }}
            type="button"
            role="radio"
            aria-checked={checked}
            tabIndex={checked ? 0 : -1}
            disabled={busy}
            title={t(`mode.${value}_hint`)}
            onClick={() => {
              choose(value);
            }}
            className={cn(
              "flex h-7 items-center gap-1.5 rounded-full px-3 text-xs font-medium transition-all outline-none focus-visible:ring-2 focus-visible:ring-ring",
              checked
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            <Icon aria-hidden className="size-3.5" />
            {t(`mode.${value}`)}
          </button>
        );
      })}
    </div>
  );
}
