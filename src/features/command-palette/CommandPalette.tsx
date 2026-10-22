import { useNavigate } from "@tanstack/react-router";
import { ArrowRightIcon, BookIcon, SearchIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { matchesSurah } from "@/features/reader/surahFilter";
import { surahName } from "@/features/reader/surahName";
import { verseNumber, verseReference } from "@/features/reader/verseCopy";
import { type GoToTarget, parseGoTo } from "@/lib/goTo";
import { useSurahs } from "@/lib/queries";

/** Whether this is a Mac, where the shortcut is ⌘K rather than Ctrl+K. */
const IS_MAC = typeof navigator !== "undefined" && /Mac|iPhone|iPad/u.test(navigator.userAgent);

/** The shortcut as shown and in `aria-keyshortcuts` syntax. */
const SHORTCUT = IS_MAC ? { label: "⌘K", aria: "Meta+K" } : { label: "Ctrl+K", aria: "Control+K" };

/**
 * The command palette (Ctrl/⌘+K). For now it has one command, "Go to": a
 * verse (`2:255`, `Bakara 255`, `Baqarah 255`) or a surah by number or name.
 */
export function CommandPalette() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const surahs = useSurahs();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key.toLowerCase() !== "k" || event.altKey || event.shiftKey) return;
      if (!(IS_MAC ? event.metaKey : event.ctrlKey)) return;
      event.preventDefault();
      setOpen((current) => !current);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  const list = surahs.data ?? [];
  const result = parseGoTo(query, list);
  const target = result?.ok ? result.target : undefined;
  const targetSurah = target && list.find((surah) => surah.number === target.surah);
  // Surahs matching the text so far, while it is not yet a complete target.
  const matches = target ? [] : list.filter((surah) => matchesSurah(surah, query));
  const name = (number: number) => {
    const surah = list.find((item) => item.number === number);
    return surah ? surahName(surah, i18n.resolvedLanguage) : String(number);
  };

  const go = ({ surah, ayah }: GoToTarget) => {
    setOpen(false);
    setQuery("");
    if (ayah === undefined) void navigate({ to: "/read/$surah", params: { surah } });
    else void navigate({ to: "/read/$surah/$ayah", params: { surah, ayah } });
  };

  return (
    <>
      <Button
        variant="ghost"
        size="sm"
        aria-keyshortcuts={SHORTCUT.aria}
        className="text-muted-foreground"
        onClick={() => {
          setOpen(true);
        }}
      >
        <SearchIcon aria-hidden />
        {t("command_palette.go_to")}
        <kbd className="ms-1 font-sans text-xs">{SHORTCUT.label}</kbd>
      </Button>
      <Dialog
        open={open}
        onOpenChange={(next) => {
          setOpen(next);
          if (!next) setQuery("");
        }}
      >
        <DialogContent className="top-1/3 overflow-hidden p-0" showCloseButton={false}>
          <DialogHeader className="sr-only">
            <DialogTitle>{t("command_palette.title")}</DialogTitle>
            <DialogDescription>{t("command_palette.hint")}</DialogDescription>
          </DialogHeader>
          <Command shouldFilter={false} label={t("command_palette.go_to")}>
            <CommandInput
              value={query}
              onValueChange={setQuery}
              placeholder={t("command_palette.placeholder")}
            />
            <CommandList>
              <CommandEmpty>
                {result && !result.ok
                  ? t(`command_palette.errors.${result.error}`)
                  : t("command_palette.hint")}
              </CommandEmpty>
              {target && (
                <CommandGroup heading={t("command_palette.go_to")}>
                  <CommandItem
                    value="target"
                    onSelect={() => {
                      go(target);
                    }}
                  >
                    <ArrowRightIcon aria-hidden />
                    {target.ayah === undefined
                      ? t("command_palette.go_to_surah", {
                          number: target.surah,
                          name: name(target.surah),
                        })
                      : t("command_palette.go_to_verse", {
                          reference: targetSurah
                            ? verseReference(name(target.surah), {
                                surah: target.surah,
                                ayah: target.ayah,
                              })
                            : verseNumber({ surah: target.surah, ayah: target.ayah }),
                        })}
                  </CommandItem>
                </CommandGroup>
              )}
              {matches.length > 0 && query.trim() !== "" && (
                <CommandGroup heading={t("command_palette.surahs")}>
                  {matches.map((surah) => (
                    <CommandItem
                      key={surah.number}
                      value={`surah-${String(surah.number)}`}
                      onSelect={() => {
                        go({ surah: surah.number });
                      }}
                    >
                      <BookIcon aria-hidden />
                      {t("command_palette.go_to_surah", {
                        number: surah.number,
                        name: surahName(surah, i18n.resolvedLanguage),
                      })}
                    </CommandItem>
                  ))}
                </CommandGroup>
              )}
            </CommandList>
          </Command>
        </DialogContent>
      </Dialog>
    </>
  );
}
