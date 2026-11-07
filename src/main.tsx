import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "./i18n";
// Reader typefaces (see `lib/fonts.ts`): each loads only the scripts a page uses.
import "@fontsource/amiri/400.css";
import "@fontsource/scheherazade-new/400.css";
import "@fontsource/noto-naskh-arabic/400.css";
import "@fontsource/noto-kufi-arabic/400.css";
import "@fontsource/inter/400.css";
import "@fontsource/inter/600.css";
import "@fontsource/literata/400.css";
import "@fontsource/literata/600.css";
import "@fontsource/noto-serif/400.css";
import "@fontsource/noto-serif/600.css";
import "@fontsource/lora/400.css";
import "@fontsource/lora/600.css";
import "@fontsource/source-serif-4/400.css";
import "@fontsource/source-serif-4/600.css";
import "./styles.css";

import { Providers } from "./app/providers";
import { router } from "./app/router";

if (import.meta.env.DEV) {
  void import("./features/reader/perf").then(({ installPerfTools }) => {
    installPerfTools();
  });
}

const root = document.getElementById("root");
if (!root) throw new Error("#root element missing from index.html");

createRoot(root).render(
  <StrictMode>
    <Providers>
      <RouterProvider router={router} />
    </Providers>
  </StrictMode>,
);
