import { BookOpenIcon, type LucideIcon, MicroscopeIcon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useApplyProfile } from "@/features/profile/useApplyProfile";
import type { ExperienceProfile } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

/**
 * Steps of the first-launch flow (spec §13.3 item 5), in order.
 * TODO(M3): add the "Recommended packs" card after the profile step.
 */
const STEPS = ["profile"] as const;

/**
 * First launch: asks for the experience profile, then (from M3) offers the
 * recommended packs. Shown once, while `experience_profile` is unset; it
 * cannot be dismissed without an answer because the answer sets the defaults.
 */
export function OnboardingDialog() {
  const { t } = useTranslation();
  // Settings are hydrated before the shell mounts, so this is known on mount.
  const [open, setOpen] = useState(
    () => useUiStore.getState().preferences.experience_profile === null,
  );
  const [step, setStep] = useState(0);
  const next = () => {
    if (step + 1 < STEPS.length) setStep(step + 1);
    else setOpen(false);
  };

  return (
    <Dialog open={open}>
      <DialogContent
        showCloseButton={false}
        className="sm:max-w-xl"
        onEscapeKeyDown={(event) => {
          event.preventDefault();
        }}
        onInteractOutside={(event) => {
          event.preventDefault();
        }}
      >
        <DialogHeader>
          <DialogTitle>{t("onboarding.title")}</DialogTitle>
          <DialogDescription>{t("onboarding.description")}</DialogDescription>
        </DialogHeader>
        {STEPS[step] === "profile" && <ProfileStep onDone={next} />}
      </DialogContent>
    </Dialog>
  );
}

const PROFILES: { profile: ExperienceProfile; icon: LucideIcon }[] = [
  { profile: "reader", icon: BookOpenIcon },
  { profile: "researcher", icon: MicroscopeIcon },
];

/** The profile question; writes every default of the chosen profile at once. */
function ProfileStep({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation();
  const { apply, busy, error } = useApplyProfile();

  return (
    <div className="grid gap-3">
      <div className="grid gap-3 sm:grid-cols-2">
        {PROFILES.map(({ profile, icon: Icon }) => (
          <button
            key={profile}
            type="button"
            disabled={busy}
            onClick={() => {
              apply(profile, onDone);
            }}
            className="flex flex-col items-start gap-2 rounded-lg border p-4 text-start transition-colors hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none disabled:opacity-50"
          >
            <Icon aria-hidden className="size-5 text-primary" />
            <span className="font-medium">{t(`onboarding.${profile}_title`)}</span>
            <span className="text-muted-foreground">{t(`onboarding.${profile}_description`)}</span>
          </button>
        ))}
      </div>
      {error && <ErrorMessage error={error} />}
      <p className="text-xs text-muted-foreground">{t("onboarding.note")}</p>
    </div>
  );
}
