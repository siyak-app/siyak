import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// Testing Library only cleans up automatically when test globals are enabled.
afterEach(() => {
  cleanup();
});

// jsdom lacks these; Radix popovers and tooltips observe their size, cmdk
// scrolls the selected item into view. Neither matters without layout.
const noop = () => undefined;
if (!("ResizeObserver" in globalThis)) {
  globalThis.ResizeObserver = class {
    observe = noop;
    unobserve = noop;
    disconnect = noop;
  };
}
if (!("scrollIntoView" in Element.prototype)) {
  Object.defineProperty(Element.prototype, "scrollIntoView", { value: noop, writable: true });
}
