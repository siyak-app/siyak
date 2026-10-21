# Siyak website

Landing page and documentation for Siyak, built with [Astro](https://astro.build) and
[Starlight](https://starlight.astro.build). Pages live in `src/content/docs/{en,tr}/`; English is
the source of truth and Turkish pages start with a "last synced" line.

```sh
pnpm --dir website dev     # local preview
pnpm --dir website build   # static build into website/dist
```

Deployment to GitHub Pages and the `quran.kemalozyon.com` domain come in a later milestone.
