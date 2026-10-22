import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "./i18n";
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
