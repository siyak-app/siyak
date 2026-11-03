import { useQueryClient } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { SearchIcon } from "lucide-react";
import { type SyntheticEvent, useState } from "react";
import { useTranslation } from "react-i18next";

import { Input } from "@/components/ui/input";
import type { QueryClass } from "@/lib/bindings";
import { classifyQuery } from "@/lib/queries";

/** The single search box in the header (spec §7.1). Follows the URL's query. */
export function SearchBox() {
  const search: { q?: unknown } = useSearch({ strict: false });
  const q = typeof search.q === "string" || typeof search.q === "number" ? String(search.q) : "";
  return <SearchForm key={q} initial={q} />;
}

function SearchForm({ initial }: { initial: string }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [value, setValue] = useState(initial);

  const showResults = (q: string) => {
    void navigate({ to: "/search", search: { q, mode: "all" } });
  };

  const goTo = ({ surah, ayah }: Extract<QueryClass, { kind: "reference" }>) => {
    if (ayah === null) void navigate({ to: "/read/$surah", params: { surah } });
    else void navigate({ to: "/read/$surah/$ayah", params: { surah, ayah } });
  };

  const submit = (event: SyntheticEvent) => {
    event.preventDefault();
    const q = value.trim();
    if (q === "") return;
    // A verse reference needs no search: the reader jumps to it (spec §7.1).
    classifyQuery(queryClient, q).then(
      (result) => {
        if (result.kind === "reference") goTo(result);
        else showResults(q);
      },
      () => {
        // The results screen shows the error.
        showResults(q);
      },
    );
  };

  return (
    <form role="search" onSubmit={submit} className="relative w-56 max-w-[40%]">
      <SearchIcon
        aria-hidden
        className="pointer-events-none absolute start-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"
      />
      <Input
        type="search"
        dir="auto"
        value={value}
        onChange={(event) => {
          setValue(event.target.value);
        }}
        aria-label={t("search.box_label")}
        placeholder={t("search.box_placeholder")}
        className="h-7 ps-7"
      />
    </form>
  );
}
