// @ts-check
import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";

// https://astro.build/config
export default defineConfig({
  site: "https://quran.kemalozyon.com",
  // TODO(M9): pick /tr/ or /en/ from the browser language (spec §16.2).
  redirects: { "/": "/en/" },
  integrations: [
    starlight({
      title: "Siyak",
      description: "A free, open-source, offline Quran reading and research tool.",
      favicon: "/favicon.svg",
      defaultLocale: "en",
      locales: {
        en: { label: "English", lang: "en" },
        tr: { label: "Türkçe", lang: "tr" },
      },
      social: [{ icon: "github", label: "GitHub", href: "https://github.com/siyak/siyak" }],
      editLink: { baseUrl: "https://github.com/siyak/siyak/edit/main/website/" },
      sidebar: [
        {
          label: "Development",
          translations: { tr: "Geliştirme" },
          items: [{ slug: "getting-started" }],
        },
      ],
    }),
  ],
});
