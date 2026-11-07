# Siyak'a katkı

> İngilizce sürümle son eşitleme: 2026-10-21

Yardımınız için teşekkürler. Siyak; kod, veri ve lisans araştırması, çeviri, terim incelemesi ve
dokümantasyon katkılarına açıktır. Asıl metin İngilizce [CONTRIBUTING.md](CONTRIBUTING.md)
dosyasıdır; bu dosya onun Türkçesidir.

**15 dakika hedefi:** Gereksinimleri kurulu bir bilgisayarda depoyu klonlayıp uygulamayı 15
dakikadan kısa sürede çalışır hâlde görebilmelisiniz. Sizi yavaşlatan bir şey olursa bu bir
hatadır; lütfen bir issue açın.

## Gereksinimler

Tüm platformlarda:

- [Git](https://git-scm.com/)
- rustup ile [Rust](https://rustup.rs/). Kullanılacak sürüm `rust-toolchain.toml` dosyasında
  sabitlenmiştir ve ilk `cargo` komutunda kendiliğinden kurulur.
- [Node.js](https://nodejs.org/) 24 LTS veya daha yenisi
- [pnpm](https://pnpm.io/installation) 12: `npm install -g pnpm@12`

Ardından işletim sisteminize göre:

### Windows

- "Desktop development with C++" iş yüküyle
  [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC).
- WebView2 çalışma ortamı. Windows 10 (1803 ve sonrası) ile Windows 11'de hazır gelir; yoksa
  [Evergreen Bootstrapper](https://developer.microsoft.com/microsoft-edge/webview2/) ile kurun.

### macOS

- Xcode Command Line Tools: `xcode-select --install`

### Linux

Debian / Ubuntu:

```sh
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Linux'ta tilavetler (ses paketleri) GStreamer ile çalınır; MP3 çözücüsünü de kurun
(Debian/Ubuntu'da `gstreamer1.0-plugins-good`, Fedora'da `gstreamer1-plugins-good`).

Fedora:

```sh
sudo dnf check-update
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel
sudo dnf group install "c-development"
```

### Dev container

Dilerseniz depoyu [dev container](.devcontainer/) içinde açabilirsiniz (VS Code'da "Reopen in
Container" ya da GitHub Codespaces). Rust, Node, pnpm ve Linux kütüphaneleri hazırdır.

## Kurulum

```sh
git clone https://github.com/siyak-app/siyak.git
cd siyak
pnpm install
pnpm tauri dev
```

İlk `pnpm tauri dev` Rust çekirdeğini derler ve birkaç dakika sürer; sonraki çalıştırmalar
hızlıdır. Uygulama Kuran'ın tam verisiyle (`src-tauri/resources/core.db`) açılır; veri hattına
(pipeline) hiç ihtiyacınız olmaz.

Uygulama verileri (iki veritabanı) işletim sisteminin `com.kemalozyon.siyak` için ayırdığı veri
dizininde durur: macOS'ta `~/Library/Application Support/com.kemalozyon.siyak`, Windows'ta
`%APPDATA%\com.kemalozyon.siyak`, Linux'ta `~/.local/share/com.kemalozyon.siyak`. Bu dizini silmek
uygulamayı sıfırlar.

### Kuran verisi

Debug ve release derlemeler, [`siyak-data`](https://github.com/siyak-app/siyak-data) tarafından
üretilen tam veriyi, `src-tauri/resources/core.db`'yi kullanır. Kaynağı [`dev-data/README.md`](dev-data/README.md#real-data-coredb)
dosyasında kayıtlıdır: şu an siyak-data commit'i `72bc795`, sha256 `76ed118ad4be…`,
`core_version` 1. Bakımcılar onu `cargo xtask sync-core` ile günceller.

Debug derlemeyi, morfolojiyi ve anlamsal özellikler için bir vektör paketini (`vectors.dev`: anlamı
değil ortak kelimeleri eşler; bkz. [`dev-data/README.md`](dev-data/README.md#development-vectors))
hazır içeren küçük geliştirme veri setiyle (`dev-data/`) çalıştırmak için
`SIYAK_DEV_DATA=dev pnpm tauri dev` kullanın. Her veri setinin kendi içerik veritabanı vardır
(geliştirme verisi için `content.db`, tam veri için `content.full.db`); böylece hiçbir şeyi
sıfırlamadan ikisi arasında geçiş yapabilirsiniz. `user.db` ortaktır. Debug derlemeler hangi veri
setinin yüklü olduğunu üst çubukta gösterir. Yayımlanan paketler Kuran'ın tamamını kapsadığı için
yalnızca tam veriye kurulur; geliştirme verisinde uygulama bunları reddeder ve nedenini söyler.

Paketleri yayımlamadan denemek için onları `siyak-data` içinde derleyin ve bir debug derlemeyi
çıktısına yönlendirin: `SIYAK_PACK_SOURCE=../siyak-data/dist pnpm tauri dev`. Uygulama bu durumda
`manifest.json` dosyasını o dizinden okur ve her paketi dosya adıyla oradan, indirmeyle aynı
denetimlerden geçirerek kopyalar. Embedding paketleri gibi henüz yayımlanmamış paketleri yalnızca
`siyak-data manifest --unpublished` listeler.

## Komutlar

| Komut                      | Ne yapar                                                                       |
| -------------------------- | ------------------------------------------------------------------------------ |
| `pnpm tauri dev`           | Uygulamayı anlık yenilemeyle çalıştırır                                        |
| `pnpm tauri build --debug` | İmzasız bir debug uygulaması ve kurulum paketi üretir                          |
| `pnpm lint`                | i18n ve sağdan sola kuralları dahil ESLint                                     |
| `pnpm format`              | Prettier ile biçimlendirir (`pnpm format:check` yalnızca denetler)             |
| `pnpm typecheck`           | TypeScript tip denetimi                                                        |
| `pnpm test`                | Frontend testleri (Vitest)                                                     |
| `pnpm docs:ts`             | Dışa açık sorgu hook'larının TSDoc'unu denetler (TypeDoc)                      |
| `pnpm changelog`           | `CHANGELOG.md`'yi Conventional Commits'ten yeniden üretir (git-cliff)          |
| `pnpm --dir website dev`   | Web sitesini önizler                                                           |
| `cargo test`               | Rust testleri                                                                  |
| `cargo xtask bindings`     | Bir komutu ya da tiplerini değiştirdikten sonra `src/lib/bindings.ts`'i üretir |
| `cargo xtask build-dev-db` | `dev-data/fixtures/` değişince `dev-data/dev-content.db`'yi yeniden üretir     |
| `cargo xtask sync-core`    | `siyak-data`'nın `core.db`'sini denetleyip `src-tauri/resources/`'a kopyalar   |

## Kontrolleri yerelde çalıştırma

CI bunları her pull request'te çalıştırır; push etmeden önce siz de çalıştırın:

```sh
pnpm lint && pnpm format:check && pnpm typecheck && pnpm test && pnpm docs:ts
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo xtask bindings && git diff --exit-code -- src/lib/bindings.ts
pnpm --dir website build
```

## Kurallar

- **Kodda İngilizce.** Tanımlayıcılar, yorumlar, commit mesajları, branch adları ve PR metinleri
  İngilizcedir. İslami terimler [sözlüğe](docs/glossary.md) uyar: `surah`, `verse`, `translation`,
  `tafsir`; asla `sure`, `ayet` ya da `meal` değil.
- **Kullanıcıya görünen metin koda gömülmez.** Her metni hem `src/i18n/en.json` hem
  `src/i18n/tr.json` dosyasına ekleyin. Teknik terimlerin sade ve teknik iki biçimi olur
  (`term.<ad>.plain` / `term.<ad>.technical`).
- **Sağdan sola uyumlu.** Mantıksal Tailwind sınıfları kullanın (`ms-`, `me-`, `ps-`, `pe-`,
  `start-`, `end-`); Arapça metin `dir="rtl" lang="ar"` alır.
- **Hatalar koddur.** Rust, kararlı bir kodla `AppError` döndürür; arayüz onu çevirir. Testler ve
  kurtarılamaz başlangıç kodu dışında `unwrap()`/`expect()` kullanılmaz.
- **Belgelenmiş kod.** Her Rust modülü `//!` ile başlar, her dışa açık öğenin `///` yorumu vardır,
  dışa açık hook'ların TSDoc'u vardır. Yorumlar "neden"i anlatır ve mantık açık değilse ilgili spec
  bölümüne ya da ADR'ye bağlantı verir.
- **Küçük ve odaklı.** Ölü ya da yoruma alınmış kod yok; sonraki aşamalar yarım bir özellik değil,
  belgelenmiş bir bağlantı noktası (seam) alır.

### Branch ve commit'ler

- `main`'den dallanın: `feat/reader-verse-actions`, `fix/settings-save`, `docs/glossary-terms`.
- Commit mesajları ve PR başlıkları [Conventional Commits](https://www.conventionalcommits.org/)
  biçimindedir: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `build:`, `ci:`, `chore:`. PR'lar
  squash ile birleştirilir; PR başlığı commit mesajı olur (CI bunu denetler).

### Pull request'ler

1. Küçük bir düzeltmeden büyük her şey için önce bir issue açın ya da var olana yorum yazın.
2. Mimari değişiklikler bir ADR önerisiyle başlar: `docs/adr/NNNN-baslik.md` ekleyen bir PR
   ([şablon](docs/adr/0000-template.md)).
3. [Pull request şablonunu](.github/PULL_REQUEST_TEMPLATE.md) doldurun; değişikliğin okuyucu (sade)
   varsayılanlarında nasıl göründüğü ve iki dilin de güncellenip güncellenmediği dahil.
4. CI yeşil olmalı ve bir bakımcının onayı gerekir.

Tamamlanmış sayılma ölçütü: kod, testler, gerekiyorsa İngilizce dokümantasyon ve değişiklik
kullanıcıya dönükse iki dilde rehber güncellemesi.

## Katkı yolları

| Yol                                                                                                            | Nerede                                                | Rehber                                                               |
| -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- | -------------------------------------------------------------------- |
| Kod                                                                                                            | bu depo                                               | bu dosya, `good first issue` etiketli issue'lar                      |
| Yeni meal veya tefsir kaynakları, lisans araştırması                                                           | [siyak-data](https://github.com/siyak-app/siyak-data) | oradaki `docs/adding-a-source.md`, "New source proposal" issue formu |
| Arayüz çevirisi (yeni bir dil)                                                                                 | `src/i18n/`                                           | `docs/translating.md` (yakında)                                      |
| Morfoloji etiketlerinin Türkçe karşılıklarını ve terim çevirilerini gözden geçirme; QAC etiket hatası bildirme | issue'lar ve Discussions                              | "Translation or term correction" ve "QAC tag error" issue formları   |
| Dokümantasyon                                                                                                  | `website/`                                            | her sayfadaki "Bu sayfayı düzenle" bağlantısı                        |

Issue'lar ve Discussions'ta Türkçe ya da İngilizce yazabilirsiniz.

## Topluluk dosyaları

Davranış kuralları, güvenlik politikası (açıkların gizlice nasıl bildirileceği) ve destek bilgileri
organizasyonun [`.github` deposunda](https://github.com/siyak-app/.github) durur ve tüm Siyak depoları
için geçerlidir.

## Katkıların lisansı

Aksini açıkça belirtmediğiniz sürece, Siyak'a eklenmesi amacıyla gönderdiğiniz her katkı, ek bir
şart olmaksızın [LICENSE-MIT](LICENSE-MIT) ve [LICENSE-APACHE](LICENSE-APACHE) dosyalarında
açıklandığı gibi `MIT OR Apache-2.0` çift lisansıyla lisanslanır.
