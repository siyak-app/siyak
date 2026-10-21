/**
 * jsdom has no layout: every element measures 0×0, so a virtualized list
 * renders nothing. `stubLayout` gives every element an 800×800 viewport and
 * list items a fixed height, which is enough for TanStack Virtual to render
 * a realistic window. Returns a function that restores the originals.
 */
export function stubLayout(rowHeight = 150, viewport = 800): () => void {
  const proto = HTMLElement.prototype;
  const originals = (["clientHeight", "clientWidth", "offsetHeight"] as const).map(
    (name) => [name, Object.getOwnPropertyDescriptor(proto, name)] as const,
  );
  Object.defineProperty(proto, "clientHeight", { configurable: true, get: () => viewport });
  Object.defineProperty(proto, "clientWidth", { configurable: true, get: () => viewport });
  Object.defineProperty(proto, "offsetHeight", {
    configurable: true,
    get(this: HTMLElement) {
      return this.getAttribute("role") === "listitem" ? rowHeight : 0;
    },
  });
  return () => {
    for (const [name, descriptor] of originals) {
      if (descriptor) Object.defineProperty(proto, name, descriptor);
    }
  };
}
