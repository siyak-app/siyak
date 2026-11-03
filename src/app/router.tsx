import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  notFound,
  redirect,
} from "@tanstack/react-router";

import { CompareView } from "@/features/compare/CompareView";
import { ReaderView } from "@/features/reader/ReaderView";
import { SurahNotFound } from "@/features/reader/SurahNotFound";
import { parseSearchParams } from "@/features/search/searchParams";
import { SearchView } from "@/features/search/SearchView";
import { useUiStore } from "@/stores/ui";

import { AppShell } from "./layout/AppShell";
import { formatRangeParam, parseAyahParam, parseRangeParam, parseSurahParam } from "./readParams";

const rootRoute = createRootRoute({ component: AppShell });

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  beforeLoad: () => {
    // Settings are loaded before the router mounts, so the last position is known.
    const last = useUiStore.getState().preferences["reader.last_position"];
    /* eslint-disable @typescript-eslint/only-throw-error -- TanStack Router redirects by throwing */
    if (last) throw redirect({ to: "/read/$surah/$ayah", params: last });
    throw redirect({ to: "/read/$surah", params: { surah: 1 } });
    /* eslint-enable @typescript-eslint/only-throw-error */
  },
});

const parseSurah = (raw: string): number => {
  const surah = parseSurahParam(raw);
  // eslint-disable-next-line @typescript-eslint/only-throw-error -- TanStack Router signals 404 by throwing
  if (surah === null) throw notFound();
  return surah;
};

/** `/read/:surah`, the reader (spec §13.1 deep links). */
export const readRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/read/$surah",
  params: {
    parse: ({ surah }) => ({ surah: parseSurah(surah) }),
    stringify: ({ surah }) => ({ surah: String(surah) }),
  },
  component: ReaderView,
  notFoundComponent: SurahNotFound,
});

/** `/read/:surah/:ayah`: the reader, scrolled to the verse and highlighting it. */
export const readVerseRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/read/$surah/$ayah",
  params: {
    parse: ({ surah, ayah }) => {
      const parsed = parseAyahParam(ayah);
      // eslint-disable-next-line @typescript-eslint/only-throw-error -- TanStack Router signals 404 by throwing
      if (parsed === null) throw notFound();
      return { surah: parseSurah(surah), ayah: parsed };
    },
    stringify: ({ surah, ayah }) => ({ surah: String(surah), ayah: String(ayah) }),
  },
  component: ReaderView,
  notFoundComponent: SurahNotFound,
});

/** `/compare/:surah/:range`, e.g. `/compare/2/153-157` (spec §13.1 deep links). */
export const compareRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/compare/$surah/$range",
  params: {
    parse: ({ surah, range }) => {
      const parsed = parseRangeParam(range);
      // eslint-disable-next-line @typescript-eslint/only-throw-error -- TanStack Router signals 404 by throwing
      if (parsed === null) throw notFound();
      return { surah: parseSurah(surah), range: parsed };
    },
    stringify: ({ surah, range }) => ({ surah: String(surah), range: formatRangeParam(range) }),
  },
  component: function CompareRoute() {
    const { surah, range } = compareRoute.useParams();
    return <CompareView surah={surah} from={range.from} to={range.to} />;
  },
  notFoundComponent: SurahNotFound,
});

/** `/search?q=…&mode=…`: the search screen. */
export const searchRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/search",
  validateSearch: parseSearchParams,
  component: function SearchRoute() {
    const { q, mode } = searchRoute.useSearch();
    return <SearchView q={q} mode={mode} />;
  },
});

/**
 * Hash history: the app is served from local files inside the webview, where
 * there is no server to answer deep paths on reload.
 */
export const router = createRouter({
  routeTree: rootRoute.addChildren([
    indexRoute,
    readRoute,
    readVerseRoute,
    compareRoute,
    searchRoute,
  ]),
  history: createHashHistory(),
  defaultNotFoundComponent: SurahNotFound,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
