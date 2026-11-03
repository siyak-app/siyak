# Siyak

**Read every verse in context.** · [Türkçe](README.tr.md)

Siyak is a free, open-source desktop app for reading and researching the Quran. Arabic text and
translations, tafsir, word and root analysis, search and notes live in one interface that stays
simple for readers and reveals depth for researchers. Everything runs on your computer.

> **Status: pre-alpha.** There is no release yet. The skeleton runs with a small development
> dataset; see the [milestones](docs/spec/spec.md#17-milestones).

## Principles

- **Offline first:** no account and no server; all content lives in local databases.
- **Free and open source:** code under `MIT OR Apache-2.0`; every data source keeps its own license.
- **No API key required** for reading, comparison, root analysis, search and notes. AI features
  are an optional layer with your own key or a local model.
- **Faithful to sources:** the original text is always one click away, and machine output is
  clearly labelled.

## Run it from source

With the [prerequisites](CONTRIBUTING.md#prerequisites) for your OS installed:

1. `git clone https://github.com/siyak-app/siyak.git`
2. `cd siyak`
3. `pnpm install`
4. `pnpm tauri dev`

The app opens with Al-Fatiha and a few other surahs from the development dataset.

## Learn more

- [CONTRIBUTING](CONTRIBUTING.md): setup, checks, conventions and ways to contribute (also without
  writing code)
- [ARCHITECTURE](ARCHITECTURE.md): how the app is put together
- [Specification](docs/spec/spec.md) and [decision records](docs/adr/README.md)
- [Glossary](docs/glossary.md)
- Website: <https://quran.kemalozyon.com> (coming soon)

## License

Code: dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

Data has its own licenses. The development dataset uses the Quran text of the
[Tanzil Project](https://tanzil.net) (CC BY 3.0, verbatim) and the public-domain Pickthall
translation; see [dev-data/README.md](dev-data/README.md). The Arabic font is
[Amiri Quran](https://github.com/aliftype/amiri) (SIL Open Font License).
