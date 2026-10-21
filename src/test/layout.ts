/**
 * jsdom has no layout: every element measures 0×0 and nothing scrolls, so a
 * virtualized list renders nothing. `stubLayout` gives every element an
 * 800×800 viewport with plenty of content below, list items a fixed height,
 * and makes `scrollTo` move `scrollTop` and fire a `scroll` event. That is
 * enough for TanStack Virtual to render a realistic window. Returns a function
 * that restores the originals.
 */
export function stubLayout(rowHeight = 150, viewport = 800): () => void {
  const proto = HTMLElement.prototype;
  const names = [
    "clientHeight",
    "clientWidth",
    "offsetHeight",
    "scrollHeight",
    "scrollTop",
    "scrollTo",
  ] as const;
  const originals = names.map((name) => [name, Object.getOwnPropertyDescriptor(proto, name)]);
  const scrollTops = new WeakMap<HTMLElement, number>();

  Object.defineProperty(proto, "clientHeight", { configurable: true, get: () => viewport });
  Object.defineProperty(proto, "clientWidth", { configurable: true, get: () => viewport });
  Object.defineProperty(proto, "offsetHeight", {
    configurable: true,
    get(this: HTMLElement) {
      return this.getAttribute("role") === "listitem" ? rowHeight : 0;
    },
  });
  // Tall enough that scrolling is never clamped.
  Object.defineProperty(proto, "scrollHeight", { configurable: true, get: () => 1_000_000 });
  Object.defineProperty(proto, "scrollTop", {
    configurable: true,
    get(this: HTMLElement) {
      return scrollTops.get(this) ?? 0;
    },
    set(this: HTMLElement, value: number) {
      scrollTops.set(this, value);
    },
  });
  Object.defineProperty(proto, "scrollTo", {
    configurable: true,
    writable: true,
    value(this: HTMLElement, options?: ScrollToOptions) {
      if (options?.top !== undefined) this.scrollTop = options.top;
      this.dispatchEvent(new Event("scroll"));
    },
  });

  return () => {
    for (const [name, descriptor] of originals) {
      if (descriptor) Object.defineProperty(proto, name as string, descriptor);
      else Reflect.deleteProperty(proto, name as string);
    }
  };
}
