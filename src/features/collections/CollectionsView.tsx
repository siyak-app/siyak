import { Link, useNavigate } from "@tanstack/react-router";
import { type SubmitEvent, useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useCollections, useUpsertCollection } from "@/lib/queries";

/** The list of research collections (spec §13.3). */
export function CollectionsView() {
  const { t } = useTranslation();
  const collections = useCollections();
  const create = useUpsertCollection();
  const navigate = useNavigate();
  const [title, setTitle] = useState("");
  const titleId = useId();

  const submit = (event: SubmitEvent) => {
    event.preventDefault();
    if (title.trim() === "") return;
    create.mutate(
      { id: null, title, description: null },
      {
        onSuccess: (collection) => {
          void navigate({ to: "/collections/$id", params: { id: collection.id } });
        },
      },
    );
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto grid max-w-3xl gap-4 p-8">
        <header className="grid gap-1">
          <h1 className="text-lg font-semibold">{t("collections.title")}</h1>
          <p className="text-sm text-muted-foreground">{t("collections.intro")}</p>
        </header>
        <form onSubmit={submit} className="grid gap-1.5">
          <label htmlFor={titleId} className="text-sm">
            {t("collections.new_title")}
          </label>
          <div className="flex gap-2">
            <Input
              id={titleId}
              value={title}
              onChange={(event) => {
                setTitle(event.target.value);
              }}
            />
            <Button type="submit" disabled={title.trim() === "" || create.isPending}>
              {t("collections.create")}
            </Button>
          </div>
        </form>
        {create.isError && <ErrorMessage error={create.error} />}
        {collections.isError && <ErrorMessage error={collections.error} />}
        {collections.data?.length === 0 && (
          <p className="text-muted-foreground">{t("collections.empty")}</p>
        )}
        <ul className="grid gap-2">
          {collections.data?.map((collection) => (
            <li key={collection.id}>
              <Link
                to="/collections/$id"
                params={{ id: collection.id }}
                className="grid gap-1 rounded-lg border p-3 transition-colors hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
              >
                <span className="font-medium">{collection.title}</span>
                {collection.description && (
                  <span className="line-clamp-2 text-sm text-muted-foreground">
                    {collection.description}
                  </span>
                )}
                <span className="text-xs text-muted-foreground">
                  {t("collections.item_count", { count: collection.item_count })}
                </span>
              </Link>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
