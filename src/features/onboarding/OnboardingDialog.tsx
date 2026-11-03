import { BookOpenIcon, type LucideIcon, MicroscopeIcon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { BundleCard } from "@/features/packs/BundleCard";
import { useApplyProfile } from "@/features/profile/useApplyProfile";
import { useCatalog } from "@/lib/queries";
import type { ExperienceProfile } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

/** Steps of the first-launch flow (spec §13.3 item 5), in order. */
const STEPS = ["profile", "packs"] as const;

/** The bundle offered on first launch (spec §6.3). */
const RECOMMENDED_BUNDLE = "recommended";

/**
 * First launch: asks for the experience profile, then offers the recommended
 * packs. Shown once, while `experience_profile` is unset; the profile question
 * cannot be dismissed without an answer because the answer sets the defaults.
 * The pack list is only fetched once the profile step is done.
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
        className="max-h-[85vh] overflow-y-auto sm:max-w-xl"
        onEscapeKeyDown={(event) => {
          event.preventDefault();
        }}
        onInteractOutside={(event) => {
          event.preventDefault();
        }}
      >
        <DialogHeader>
          <DialogTitle>
            {STEPS[step] === "packs" ? t("onboarding.packs_title") : t("onboarding.title")}
          </DialogTitle>
          <DialogDescription>
            {STEPS[step] === "packs"
              ? t("onboarding.packs_description")
              : t("onboarding.description")}
          </DialogDescription>
        </DialogHeader>
        {STEPS[step] === "profile" && <ProfileStep onDone={next} />}
        {STEPS[step] === "packs" && <PacksStep onDone={next} />}
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

/** The "Recommended packs" card; downloads go on after the dialog closes. */
function PacksStep({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation();
  const catalog = useCatalog();
  const bundle = catalog.data?.bundles.find((b) => b.id === RECOMMENDED_BUNDLE);
  // A running job or a finished set: there is nothing left to put off.
  const started =
    bundle &&
    (bundle.missing.length === 0 ||
      bundle.packs.some((id) => catalog.data?.packs.find((p) => p.id === id)?.job));

  return (
    <div className="grid gap-3">
      {catalog.isPending && <p className="text-sm text-muted-foreground">{t("packs.loading")}</p>}
      {catalog.data && bundle && <BundleCard bundle={bundle} catalog={catalog.data} />}
      {(catalog.isError || (catalog.data && !bundle)) && (
        <p className="text-sm text-muted-foreground">{t("onboarding.packs_unavailable")}</p>
      )}
      <p className="text-xs text-muted-foreground">{t("onboarding.packs_note")}</p>
      <Button
        variant={started ? "default" : "outline"}
        className="justify-self-end"
        onClick={onDone}
      >
        {started ? t("onboarding.packs_continue") : t("onboarding.packs_later")}
      </Button>
    </div>
  );
}
