import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  notFound,
  redirect,
} from "@tanstack/react-router";

import { ReaderView } from "@/features/reader/ReaderView";
import { SurahNotFound } from "@/features/reader/SurahNotFound";

import { AppShell } from "./layout/AppShell";

const SURAH_COUNT = 114;

const rootRoute = createRootRoute({ component: AppShell });

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  beforeLoad: () => {
    // TODO(M2): open the last read position instead of Al-Fatiha.
    // eslint-disable-next-line @typescript-eslint/only-throw-error -- TanStack Router redirects by throwing
    throw redirect({ to: "/read/$surah", params: { surah: 1 } });
  },
});

/** `/read/:surah`, the reader (spec §13.1 deep links). */
export const readRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/read/$surah",
  params: {
    parse: ({ surah }) => {
      const number = Number(surah);
      // eslint-disable-next-line @typescript-eslint/only-throw-error -- TanStack Router signals 404 by throwing
      if (!Number.isInteger(number) || number < 1 || number > SURAH_COUNT) throw notFound();
      return { surah: number };
    },
    stringify: ({ surah }) => ({ surah: String(surah) }),
  },
  component: ReaderView,
  notFoundComponent: SurahNotFound,
});

/**
 * Hash history: the app is served from local files inside the webview, where
 * there is no server to answer deep paths on reload.
 */
export const router = createRouter({
  routeTree: rootRoute.addChildren([indexRoute, readRoute]),
  history: createHashHistory(),
  defaultNotFoundComponent: SurahNotFound,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
