import type { ComponentProps } from "react";
import Markdown from "react-markdown";

/** Images would load from the network, so they are not rendered at all. */
const DISALLOWED = ["img"];

/**
 * A link stays visible as text: following it would navigate the app's own
 * window, and the webview may not open a browser (spec §14.3).
 */
function PlainLink({ children, href }: ComponentProps<"a">) {
  return (
    <span title={href} className="underline underline-offset-2">
      {children}
    </span>
  );
}

const COMPONENTS = { a: PlainLink };

/** The user's Markdown, with HTML disabled (spec §14). */
export function NoteMarkdown({ children }: { children: string }) {
  return (
    <div
      dir="auto"
      className="grid gap-2 text-sm wrap-anywhere [&_blockquote]:border-s-2 [&_blockquote]:ps-3 [&_code]:font-mono [&_h1]:font-semibold [&_h2]:font-semibold [&_h3]:font-medium [&_ol]:list-decimal [&_ol]:ps-5 [&_ul]:list-disc [&_ul]:ps-5"
    >
      <Markdown disallowedElements={DISALLOWED} unwrapDisallowed components={COMPONENTS}>
        {children}
      </Markdown>
    </div>
  );
}
