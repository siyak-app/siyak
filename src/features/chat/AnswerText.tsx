import { Link } from "@tanstack/react-router";
import type { ComponentProps } from "react";
import Markdown from "react-markdown";

import { linkCitations, VERSE_SCHEME } from "./citations";

/** Images would load from the network, so they are not rendered at all. */
const DISALLOWED = ["img"];

/** Only citation links survive; any other address becomes plain text. */
function urlTransform(url: string): string {
  return url.startsWith(VERSE_SCHEME) ? url : "";
}

/**
 * A citation opens the verse in the reader; any other link stays visible as
 * text, because the webview may not open a browser (spec §14.3).
 */
function AnswerLink({ children, href }: ComponentProps<"a">) {
  const target = href?.startsWith(VERSE_SCHEME) ? href.slice(VERSE_SCHEME.length) : null;
  const [surah, ayah] = target?.split(":").map(Number) ?? [];
  if (surah && ayah) {
    return (
      <Link
        to="/read/$surah/$ayah"
        params={{ surah, ayah }}
        className="font-medium text-primary underline underline-offset-2"
      >
        [{children}]
      </Link>
    );
  }
  return <span className="underline underline-offset-2">{children}</span>;
}

const COMPONENTS = { a: AnswerLink };

/**
 * A chat answer: model output rendered as Markdown with HTML disabled
 * (spec §13.1), its `[surah:ayah]` citations clickable.
 */
export function AnswerText({ children }: { children: string }) {
  return (
    <div
      dir="auto"
      className="grid gap-2 text-sm wrap-anywhere [&_blockquote]:border-s-2 [&_blockquote]:ps-3 [&_code]:font-mono [&_h1]:font-semibold [&_h2]:font-semibold [&_h3]:font-medium [&_ol]:list-decimal [&_ol]:ps-5 [&_ul]:list-disc [&_ul]:ps-5"
    >
      <Markdown
        disallowedElements={DISALLOWED}
        unwrapDisallowed
        components={COMPONENTS}
        urlTransform={urlTransform}
      >
        {linkCitations(children)}
      </Markdown>
    </div>
  );
}
