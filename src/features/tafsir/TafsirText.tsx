import { type ReactNode, useMemo } from "react";

/** Classes of the allowed `span`s (spec §5.2); any other `span` is plain text. */
const SPAN_CLASSES: Readonly<Record<string, string>> = {
  ayah: "font-quran text-primary",
  ref: "text-muted-foreground",
  poem: "my-2 block text-center",
  hemistich: "block",
};

/** Elements whose content is not text to read. */
const DROPPED = new Set(["script", "style", "iframe", "object", "embed", "svg", "math"]);

function toNodes(parent: Node): ReactNode[] {
  return Array.from(parent.childNodes).map((node, index) => {
    if (node.nodeType === Node.TEXT_NODE) return node.textContent;
    if (node.nodeType !== Node.ELEMENT_NODE) return null;
    const element = node as Element;
    const name = element.localName;
    if (DROPPED.has(name)) return null;
    // The order of these nodes never changes for a given text.
    if (name === "br") return <br key={index} />;
    const children = toNodes(element);
    if (name === "b") return <b key={index}>{children}</b>;
    if (name === "i") return <i key={index}>{children}</i>;
    const className =
      name === "span"
        ? Array.from(element.classList)
            .map((value) => SPAN_CLASSES[value])
            .find(Boolean)
        : undefined;
    return className ? (
      <span key={index} className={className}>
        {children}
      </span>
    ) : (
      <span key={index}>{children}</span>
    );
  });
}

/**
 * A tafsir paragraph. Pack text is untrusted: it is parsed in an inert document
 * and rebuilt from the allowed elements of spec §5.2, without any attribute,
 * so no markup from a pack ever reaches the page.
 */
export function TafsirText({ html }: { html: string }) {
  const nodes = useMemo(
    () => toNodes(new DOMParser().parseFromString(html, "text/html").body),
    [html],
  );
  return <>{nodes}</>;
}
