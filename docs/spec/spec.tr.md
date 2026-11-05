# Siyak — Kur'an Araştırma Aracı · Teknik Spesifikasyon (v1)

> İngilizce sürümle son eşitleme: 2026-10-21. Kaynak sürüm `spec.md`'dir (İngilizce); bu dosya onun Türkçe çevirisidir.

> Durum: Taslak · Son güncelleme: 2026-10-21
> Bu doküman uygulamanın ana iskeletini tanımlar. "Karar" olarak işaretlenenler kilitlidir; "Açık soru" olanlar §18'de listelenir.

---

## 1. Vizyon ve kapsam

Kuran'la ilgilenen herkes (ilahiyat öğrencileri, araştırmacılar, meraklı okuyucular) için **açık kaynak, ücretsiz, hepsi bir arada bir araştırma aracı**. Okuma, karşılaştırma, kelime/kök inceleme, tefsir, arama, not alma ve AI destekli sorgulama tek uygulamada bulunur.

Uygulamanın backend'i yoktur; her şey kullanıcının bilgisayarında çalışır. Tüm veri (ayetler, mealler, tefsirler, morfoloji, vektörler, kullanıcı notları) lokal SQLite'ta durur. İnternet yalnızca şu durumlarda gerekir:
- veri paketi indirmek,
- uygulama güncellemesi almak,
- bulut tabanlı bir AI sağlayıcısıyla sohbet veya çeviri yapmak (lokal model kullanılırsa bu da gerekmez).

**Temel ilkeler**
1. **Kaynağa sadakat:** Her metnin orijinali her zaman bir tık uzaktadır. Makine çıktıları (çeviri, AI cevabı) açıkça etiketlenir ve kaynağıyla yan yana gösterilir.
2. **Anahtarsız tam değer:** Okuma, karşılaştırma, kök analizi, arama ve notlar API anahtarı olmadan çalışır. AI özellikleri ek katmandır.
3. **Doğrulanabilirlik:** Kod, veri pipeline'ı ve kaynak lisansları açıktır; binary'nin koddan üretildiği kanıtlanabilir.
4. **Lisans önceliklidir:** Redistribution hakkı teyit edilmemiş içerik dağıtılmaz.
5. **Herkes için sade, isteyene derin:** Aynı arayüz hem okuyucuya hem araştırmacıya hizmet eder. Ekran sade açılır; derinlik kullanıcı istedikçe ortaya çıkar (kademeli açılım, §13.2). Ayrı "basit/uzman" modları yoktur.
6. **Katkıya açık:** Kod okunabilir, kararlar kayıtlı, dokümantasyon iki dilli; kod yazmayanların da katkı verebileceği yollar vardır (§16).

### v1 özellikleri

| Alan | Özellik |
|---|---|
| Okuma | Sure/ayet okuma, Arapça metin + seçilebilir meal(ler) |
| Karşılaştırma | Bir ayet ya da aralık için birden fazla meal ve tefsiri paralel sütunlarda gösterme |
| Kelime/kök analizi | Kelimeye tıklayınca morfoloji (kök, lemma, sözcük türü, çekim); kökün/lemmanın geçtiği tüm ayetler (konkordans) |
| Tefsir | Ayete bağlı tefsir paneli; Arapça tefsirler için isteğe bağlı Türkçe özet/çeviri (BYOK) |
| Çapraz referans | Bir ayetle ilişkili ayetler: anlamsal benzerlik + ortak kök/lemma |
| Arama | Kelime araması (FTS5), kök/lemma araması, anlamsal arama (lokal embedding) |
| Koleksiyonlar ve notlar | Yer imi, ayete bağlı notlar, ayetleri araştırma konularına göre gruplayan koleksiyonlar |
| Dışa aktarma | Koleksiyon/not → Markdown; ayet ve meal/tefsir için akademik atıf metni; JSON yedekleme |
| AI sohbet (BYOK) | RAG sohbet: Anthropic, OpenAI, Google Gemini, OpenAI-uyumlu endpoint (Ollama, LM Studio, OpenRouter vb.) |
| Paket yöneticisi | Meal/tefsir/morfoloji/vektör/model paketlerini indirme, güncelleme, kaldırma |
| Arayüz dili | Türkçe + İngilizce |

### v1 dışında
Ses/tilavet, hadis, cihazlar arası senkronizasyon, mobil sürüm, topluluk çeviri katkıları (§9.5), önceden çevrilmiş tefsir paketleri.

### Hedef platformlar
Windows, macOS, Linux.

---

## 2. Kararlar özeti

| Konu | Karar |
|---|---|
| Lisans (kod) | `MIT OR Apache-2.0` (çift lisans, Rust ekosistem standardı). Veri paketleri kendi lisanslarını taşır |
| Uygulama çatısı | Tauri v2 (Rust çekirdek + sistem webview'i) |
| Frontend | React + TypeScript + Vite |
| UI kit | Tailwind CSS + shadcn/ui |
| Veritabanı | SQLite (`rusqlite`, bundled) |
| Vektör arama | Bellekte tutulan vektörler üzerinde Rust'ta brute-force KNN (ADR 0012) |
| Embedding çalıştırma | Rust tarafında ONNX Runtime (`ort`) + `tokenizers` |
| Embedding modeli | **Açık soru.** Sistem modelden bağımsız tasarlanır (§7.4) |
| Tefsir erişimi | Ayet referansıyla anahtarlı lookup; tefsir embed edilmez |
| Tefsir kaynakları | v1: yalnızca modern dönem öncesi klasik tefsirler, Quranpedia veri dökümlerinden; Kurtubî QUL'dan (§6.6). Çağdaş eserler v1'e alınmaz |
| Morfoloji kaynağı | Quranic Arabic Corpus (QAC) v0.4, kullanım şartlarına harfiyen uyularak; etiketler değiştirilmez (§5.3.1). MASAQ ileride serbest lisanslı alternatif olarak değerlendirilir |
| Arapça tefsirlerin Türkçesi | Orijinal paketlenir; çeviri/özet isteğe bağlı, kullanıcının modeliyle, cache'li ve etiketli (§9) |
| Ağ çağrıları | Yalnızca Rust tarafından (`reqwest`), sabit izin listesiyle (§14) |
| API anahtarları | OS keychain (`keyring` crate); DB'ye yazılmaz, webview'e dönmez |
| Dağıtım | GitHub Releases (uygulama ve veri ayrı repo'larda) |
| Güncelleme | `tauri-plugin-updater` + GitHub Releases'taki `latest.json` |
| Not yedekleme | JSON dışa/içe aktarma |
| Telemetri | Yok |
| Arayüz düzeni | Tek arayüz, kademeli açılım; ilk açılışta seçilen deneyim profili yalnızca varsayılanları belirler |
| Dil politikası | Kod, tanımlayıcılar, yorumlar, commit'ler, PR'lar: **yalnızca İngilizce**. Dokümantasyon ve web sitesi: **Türkçe + İngilizce** (§16.1) |
| Web sitesi | Landing page + dokümantasyon tek site: Astro + Starlight, uygulama repo'sunda `website/`, GitHub Pages, `quran.kemalozyon.com` |
| Ad | **Siyak** (tefsir usulündeki siyak-sibak: ayeti bağlamıyla okumak). Landing page sloganı: "Ayeti bağlamıyla okuyun" / "Read every verse in context" |
| Kimlikler | GitHub organizasyonu `siyak-app`; repo'lar `siyak`, `siyak-data`, `.github`; Tauri identifier ve keychain servisi `com.kemalozyon.siyak` |
| GitHub | Ayrı bir organizasyon; kişisel hesap değil |
| Gömülü adresler | Manifest ve updater için birincil adres `quran.kemalozyon.com`, yedek GitHub Releases (§6.3, §15.1) |

---

## 3. Mimari genel bakış

```
┌──────────────────────── Webview (React) ────────────────────────┐
│ Okuma · Karşılaştırma · Kelime paneli · Tefsir · Arama ·        │
│ Koleksiyonlar · Sohbet · Paketler · Ayarlar                      │
│              TanStack Query ── invoke() / events / Channel       │
└───────────────────────────────┬──────────────────────────────────┘
                                │ Tauri IPC
┌───────────────────────────────┴──────── Rust çekirdek ──────────┐
│  db        content.db (içerik) + user.db (kullanıcı)            │
│  search    FTS5 · kök/lemma · bellekte KNN                       │
│  related   çapraz referans (vektör + ortak kök)                  │
│  embed     Embedder trait → lokal ONNX ya da embedding API'si    │
│  packs     manifest · indirme · sha256 · kurulum                 │
│  llm       Provider trait → Anthropic/OpenAI/Gemini/OAI-compat  │
│            ├─ chat (RAG)                                         │
│            └─ translate (tefsir özet/çeviri, maliyet takibi)    │
│  export    Markdown · atıf · JSON                                │
│  net       izin listesi + bağlantı günlüğü                       │
│  secrets   keyring                                               │
└───────────────┬──────────────────────────────┬──────────────────┘
                │ HTTPS                         │ HTTPS
     GitHub Releases (siyak-data)        AI sağlayıcı API'leri
     manifest.json + paketler            (ya da lokal endpoint)
```

**İlke:** Webview yalnızca arayüzdür. Veritabanı, model, dosya sistemi ve ağ erişimi tamamen Rust'tadır.

---

## 4. Repo yapısı

Tüm repo'lar ayrı bir GitHub organizasyonu altında durur: `github.com/siyak-app` (`github.com/siyak` ilgisiz bir hesaba ait). İki ana repo ve bir organizasyon repo'su vardır:

1. **Uygulama repo'su** (`siyak`): Tauri uygulaması, landing page ve dokümantasyon sitesi (`website/`). Lisans: `MIT OR Apache-2.0`. Release'leri installer'lar ve `latest.json` içerir.
2. **`siyak-data`**: Veri paketlerini üreten pipeline ve paket release'leri. Pipeline kodu `MIT OR Apache-2.0`; her veri kaynağı kendi lisansıyla, `sources/<kaynak>/LICENSE` altında. `releases/latest` linkleri uygulama ve veri sürümleri arasında karışmasın diye ayrı tutulur.
3. **`.github`**: Organizasyon geneli topluluk dosyaları. Buradaki `CODE_OF_CONDUCT.md`, `SECURITY.md`, `SUPPORT.md`, ortak issue şablonları ve `profile/README.md` (organizasyon tanıtım sayfası, iki dilli), kendi kopyası olmayan tüm repo'larda otomatik geçerlidir. Böylece aynı dosyalar iki repo'da ayrı ayrı tutulmaz.

### 4.1 Uygulama repo'su

```
/
├─ src/                          # React frontend
│  ├─ app/                       # router, providers, layout
│  ├─ features/
│  │  ├─ reader/                 # okuma, sanallaştırılmış ayet listesi
│  │  ├─ compare/                # paralel sütun karşılaştırma
│  │  ├─ word/                   # kelime/morfoloji paneli, konkordans
│  │  ├─ tafsir/                 # tefsir paneli, özet/çeviri
│  │  ├─ related/                # çapraz referans paneli
│  │  ├─ search/
│  │  ├─ collections/            # koleksiyonlar, notlar, yer imleri
│  │  ├─ chat/
│  │  ├─ packs/
│  │  └─ settings/               # AI sağlayıcıları, harcama, ağ günlüğü
│  ├─ components/ui/             # shadcn bileşenleri
│  ├─ lib/
│  │  ├─ bindings.ts             # tauri-specta ile üretilir
│  │  └─ queries.ts              # TanStack Query hook'ları
│  ├─ stores/                    # Zustand
│  ├─ i18n/{tr,en}.json
│  └─ assets/fonts/              # gömülü fontlar
├─ src-tauri/
│  ├─ src/
│  │  ├─ lib.rs                  # app kurulumu, komut kaydı
│  │  ├─ error.rs
│  │  ├─ db/{mod,content,user}.rs
│  │  ├─ search.rs
│  │  ├─ morphology.rs
│  │  ├─ related.rs
│  │  ├─ embed/{mod,space,service,onnx,api}.rs
│  │  ├─ packs/{manifest,download,install}.rs
│  │  ├─ llm/
│  │  │  ├─ mod.rs               # Provider trait, ortak tipler
│  │  │  ├─ anthropic.rs
│  │  │  ├─ openai.rs            # OpenAI + OpenAI-uyumlu
│  │  │  ├─ gemini.rs
│  │  │  ├─ chat.rs              # RAG akışı
│  │  │  ├─ translate.rs         # tefsir özet/çeviri
│  │  │  └─ cost.rs              # token tahmini, fiyat, harcama kaydı
│  │  ├─ export/{markdown,citation,backup}.rs
│  │  ├─ net.rs                  # izin listeli HTTP client + günlük
│  │  ├─ secrets.rs
│  │  └─ commands/
│  ├─ migrations/{content,user}/
│  ├─ resources/core.db
│  ├─ capabilities/
│  └─ tauri.conf.json
├─ dev-data/
│  └─ dev-content.db             # geliştirme için küçük örnek veri (pipeline gerekmez)
├─ website/                      # Astro + Starlight: landing page + docs sitesi (§16.2)
│  └─ src/content/docs/{en,tr}/
├─ docs/
│  ├─ spec/{spec.md,spec.tr.md}  # bu doküman ve Türkçe çevirisi
│  ├─ adr/                       # Architecture Decision Records (İngilizce)
│  └─ assets/                    # ekran görüntüleri, diyagramlar
├─ .github/
│  ├─ workflows/{ci,release,website}.yml
│  ├─ ISSUE_TEMPLATE/            # bug, feature, yeni kaynak önerisi, çeviri hatası
│  └─ PULL_REQUEST_TEMPLATE.md
├─ .devcontainer/
├─ README.md · README.tr.md
├─ CONTRIBUTING.md · CONTRIBUTING.tr.md
├─ ARCHITECTURE.md
├─ CHANGELOG.md                 # CoC ve SECURITY organizasyonun .github repo'sundan gelir
└─ LICENSE-MIT · LICENSE-APACHE
```

### 4.2 `siyak-data`

```
/
├─ sources/<kaynak>/             # ham veri + LICENSE + README (köken, sürüm)
├─ pipeline/                     # Python
│  ├─ normalize.py               # harekesizleştirme (Rust'taki ile birebir; ortak test vektörleri)
│  ├─ build_core.py              # Tanzil → core.db
│  ├─ build_translation.py
│  ├─ build_tafsir.py            # paragraflara bölme dahil
│  ├─ build_morphology.py        # morfoloji → kelime/segment tabloları
│  ├─ build_vectors.py
│  ├─ export_model.py            # ONNX + quantize
│  ├─ build_pricing.py           # pricing.json
│  └─ validate.py
├─ tests/normalize_vectors.json  # Python ve Rust'ın ortak test seti
├─ docs/
│  ├─ pack-format.md             # paket formatı spesifikasyonu
│  ├─ adding-a-source.md         # yeni meal/tefsir ekleme rehberi (lisans kontrol listesi dahil)
│  └─ sources.md                 # tüm kaynaklar, köken ve lisans tablosu
├─ manifest.template.json
├─ README.md · README.tr.md · CONTRIBUTING.md · CONTRIBUTING.tr.md
└─ .github/
   ├─ workflows/{ci,data-release}.yml
   └─ ISSUE_TEMPLATE/            # yeni kaynak önerisi, veri hatası bildirimi
```

---

## 5. Veri modeli

Kullanıcının uygulama veri klasöründe (`app_data_dir()`) iki veritabanı bulunur:

- **`content.db`**: Kuran metni ve kurulu paketlerin içeriği. Uygulama dışında yazılmaz, istenirse sıfırdan yeniden kurulabilir.
- **`user.db`**: Kullanıcının kendi verisi. Paket işlemleri bu dosyaya asla dokunmaz.

İlk açılışta `resources/core.db` dosyası `content.db` olarak kopyalanır. Uygulama daha yeni bir `core.db` ile geldiğinde çekirdek tablolar yerinde yenilenir (§6.2).

### 5.1 `content.db`: çekirdek

```sql
CREATE TABLE surahs (
  number          INTEGER PRIMARY KEY,         -- 1..114
  name_ar         TEXT NOT NULL,
  name_tr         TEXT NOT NULL,
  name_en         TEXT NOT NULL,
  revelation_type TEXT NOT NULL CHECK (revelation_type IN ('meccan','medinan')),
  ayah_count      INTEGER NOT NULL,
  bismillah       TEXT                         -- sure başlığı için Tanzil'in besmelesi; 1 ve 9 için NULL
);

CREATE TABLE verses (
  id            INTEGER PRIMARY KEY,           -- global 1..6236
  surah         INTEGER NOT NULL REFERENCES surahs(number),
  ayah          INTEGER NOT NULL,
  juz           INTEGER NOT NULL,
  text_uthmani  TEXT NOT NULL,                 -- görüntüleme
  text_clean    TEXT NOT NULL,                 -- harekesiz; arama
  UNIQUE (surah, ayah)
);

CREATE TABLE packs (
  id            TEXT PRIMARY KEY,              -- örn. 'en.pickthall'
  type          TEXT NOT NULL CHECK (type IN
                  ('translation','tafsir','morphology','vectors','model')),
  lang          TEXT,
  title         TEXT NOT NULL,
  author        TEXT,
  version       TEXT NOT NULL,                 -- semver
  license       TEXT NOT NULL,
  attribution   TEXT,
  citation_json TEXT,                          -- atıf meta verisi (§11.2)
  meta_json     TEXT,                          -- tipe özel alanlar
  installed_at  TEXT NOT NULL,
  bundled       INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE translations (
  pack_id   TEXT NOT NULL REFERENCES packs(id) ON DELETE CASCADE,
  verse_id  INTEGER NOT NULL REFERENCES verses(id),
  text      TEXT NOT NULL,
  PRIMARY KEY (pack_id, verse_id)
) WITHOUT ROWID;

CREATE TABLE content_meta (
  key    TEXT PRIMARY KEY,                     -- örn. 'core_version', 'tanzil_version'
  value  TEXT NOT NULL
);

CREATE TABLE notices (                       -- veriyle birlikte dağıtılması gereken notlar (Hakkında → Kaynaklar)
  id      TEXT PRIMARY KEY,                    -- örn. 'tanzil.text.uthmani' veya bir paket kimliği
  source  TEXT NOT NULL,                       -- kaynak kimliği, örn. 'tanzil'
  title   TEXT NOT NULL,
  body    TEXT NOT NULL,                       -- olduğu gibi
  url     TEXT
);
```

**Kur'an metni (Tanzil):** Düz metin çıktısından değil, Tanzil'in XML dosyalarından alınır (düz metin çıktısı her surenin 1. ayetinin başına besmeleyi ekliyor).
- `text_uthmani` (görüntüleme): Tanzil Uthmani; durak işaretleri ve secde işaretleri **dahil**, rub' el-hizb işaretleri **kapalı**, diğer seçenekler Tanzil varsayılanında. Seçenekler `siyak-data/sources/tanzil/source.toml`'da ve uygulamanın geliştirme verisinde kayıtlıdır ve ikisinde birebir aynı olmalıdır.
- `text_clean` (arama): Tanzil Simple Clean, olduğu gibi (kendi normalizasyonumuzla türetilmez; Tanzil'in şartları metnin değiştirilmesine izin vermiyor).
- **Besmele:** Yalnızca Fâtiha'nın 1. ayetidir. Diğer surelerde ayet değildir ve `verses` tablosunda saklanmaz; okuma ekranı onu sure başlığı olarak (Tevbe suresi, 9, hariç) Tanzil'in kendi besmele metnini tutan `surahs.bismillah` alanından gösterir. Arama ve morfoloji etkilenmez.

### 5.2 `content.db`: tefsir

Tefsir metni bir ayet aralığını kapsayabilir. Kısmi çeviri ve atıf için pipeline'da paragraflara bölünür.

```sql
CREATE TABLE tafsir (
  id          INTEGER PRIMARY KEY,
  pack_id     TEXT NOT NULL REFERENCES packs(id) ON DELETE CASCADE,
  verse_from  INTEGER NOT NULL REFERENCES verses(id),
  verse_to    INTEGER NOT NULL REFERENCES verses(id),
  kind        TEXT NOT NULL DEFAULT 'commentary' CHECK (kind IN ('commentary','introduction')),
  CHECK (verse_to >= verse_from)
);
CREATE INDEX tafsir_lookup ON tafsir(pack_id, verse_from, verse_to);

CREATE TABLE tafsir_paragraphs (
  tafsir_id  INTEGER NOT NULL REFERENCES tafsir(id) ON DELETE CASCADE,
  idx        INTEGER NOT NULL,                 -- 0'dan başlar
  text       TEXT NOT NULL,                    -- küçük, güvenli HTML alt kümesi (aşağıda)
  part       TEXT,                             -- basılı baskının cildi, biliniyorsa
  page       INTEGER,                          -- basılı baskının sayfası, biliniyorsa
  UNIQUE (tafsir_id, idx)
);                                             -- rowid tablo: WITHOUT ROWID uzun metin satırlarında disk kullanımını ~2 katına çıkarıyor
```

Paketin `lang` alanı tefsirin dilini belirtir. `lang != arayüz dili` olan tefsirlerde arayüz özet/çeviri butonlarını gösterir.

**Ayet grupları:** Kaynak veriler ayet bazında anahtarlıdır, ama klasik tefsirler çoğu zaman birkaç ayeti birlikte açıklar. Pipeline, aynı metni paylaşan ardışık ayetleri tek bir `tafsir` satırında (`verse_from`–`verse_to`) birleştirir; metin bir kez saklanır. Bir ayetin tefsiri istendiğinde onu kapsayan aralık döner ve arayüz "Bu açıklama 2:153–157 ayetlerini kapsar" bilgisini gösterir.

**Paragraf metni** sabit, küçük bir HTML alt kümesi kullanır: `b`, `i`, `br`, `span.ayah` (ayet alıntısı), `span.ref` ve beyitler için `span.poem > span.hemistich`. Uygulama render etmeden önce yeniden temizler. `part`/`page` "Taberî 1/113" gibi atıfları mümkün kılar.

**Muhakkik dipnotları v1'de çıkarılır.** Modern tahkik baskılarının dipnotları çağdaş emektir ve "çağdaş eser yok" kuralına girer; paketler yalnızca klasik metni taşır. Pipeline'da her paketin `footnotes = "drop" | "keep"` ayarı vardır; böylece klasik dipnot malzemesi (örn. Keşşâf'taki İbnü'l-Müneyyir'in *el-İntisâf*'ı) ileride tek tek açılabilir.

**Müellif mukaddimesi:** 1:1'e bağlı, `kind = 'introduction'` olan bir satır olarak saklanır. Tefsir paneli bunu 1:1'in şerhi olarak değil, ayrı bir "Mukaddime" girdisi olarak gösterir; sohbet (§10) bunu bağlama hiç eklemez.

**Baskı malzemesi** (muhakkik dirasaları, fihristler, kolofonlar) ve aynı metnin etiketsiz kopya parçaları, kitap bazında belgelenmiş kurallarla çıkarılır; çıkarılan her karakter pipeline'ın metin sadakati kapısında hesaba katılır.

**Aralık sınırları** kaynağın gruplamasını izler; kaynak verisi grupları hatalı bağladığında sınırlar basılı baskının kendi aralık başlıklarından çizilir (örn. Şevkânî'deki "[سورة … : الآيات a الى b]"). Metnin kendisi asla değiştirilmez.

**Boşluklar:** Kendi kaydı olmayan bir ayet, yalnızca önceki aralığın metni o ayeti doğrulanabilir şekilde içeriyorsa o aralığa eklenir; aralıklar sure sınırını geçmez. Doğrulanamayan boşluklar boş kalır ve raporlanır; tefsir paneli "Bu tefsirde bu ayet için ayrı bir açıklama yok" der.

**Tefsir metninde arama yapılmaz** (FTS indeksi yok). Bu, büyük tefsirlerde paket boyutunu ciddi ölçüde düşürür; tefsire erişim her zaman ayet üzerinden olur.

### 5.3 `content.db`: morfoloji

Morfoloji ayrı bir paket tipidir; v1 kaynağı Quranic Arabic Corpus'tur (§5.3.1). Kelime konumları `surah:ayah:word` olarak, segmentler ise kelime içindeki ön ek/gövde/son ek parçaları olarak tutulur.

```sql
CREATE TABLE words (
  id         INTEGER PRIMARY KEY,
  verse_id   INTEGER NOT NULL REFERENCES verses(id),
  position   INTEGER NOT NULL,                 -- ayet içindeki kelime sırası, 1'den
  text_ar    TEXT NOT NULL,                    -- Uthmani
  translit   TEXT,
  gloss_en   TEXT,                             -- kaynakta varsa
  UNIQUE (verse_id, position)
);

CREATE TABLE roots (
  id    INTEGER PRIMARY KEY,
  root  TEXT NOT NULL UNIQUE                   -- Arapça harflerle, örn. 'ق و ل'
);

CREATE TABLE lemmas (
  id       INTEGER PRIMARY KEY,
  lemma    TEXT NOT NULL UNIQUE,
  root_id  INTEGER REFERENCES roots(id)
);

CREATE TABLE word_segments (
  word_id   INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
  idx       INTEGER NOT NULL,
  kind      TEXT NOT NULL CHECK (kind IN ('prefix','stem','suffix')),
  text_ar   TEXT NOT NULL,
  pos       TEXT NOT NULL,                     -- sözcük türü etiketi
  lemma_id  INTEGER REFERENCES lemmas(id),
  root_id   INTEGER REFERENCES roots(id),
  features  TEXT,                              -- JSON: kişi, sayı, cinsiyet, kip, hâl vb.
  PRIMARY KEY (word_id, idx)
) WITHOUT ROWID;

CREATE INDEX seg_root  ON word_segments(root_id);
CREATE INDEX seg_lemma ON word_segments(lemma_id);
```

- **Hizalama:** Morfoloji kaynağının kelime bölümlemesi Tanzil Uthmani metniyle birebir eşleşmelidir. `validate.py` her ayette kelime sayısını ve metni karşılaştırır; uyuşmazlık varsa paket üretilmez.
- **Okuma ekranı:** Morfoloji paketi kuruluysa Arapça metin `words` üzerinden kelime kelime render edilir, böylece her kelime tıklanabilir olur. Kurulu değilse `verses.text_uthmani` düz metin olarak gösterilir.
- **Konkordans:** Kök veya lemmaya göre `word_segments → words → verses` sorgusuyla bulunur; sure ve sıklık özetiyle döner.

#### 5.3.1 Kaynak: Quranic Arabic Corpus (QAC)

**Seçim gerekçesi:** Değerlendirilen kaynaklar (QAC, MASAQ, Birzeit Quran Corpus/QuranMorph, QUL) arasında kök, lemma, sözcük türü ve çekim özelliklerinin tamamını içeren, en uzun süredir kullanılan ve gönüllü incelemeden geçmiş kaynak QAC'dir (v0.4, 77.429 kelime).

**Kullanım şartları ve uygulanışı:**

| QAC şartı | Uygulanışı |
|---|---|
| GNU lisansı; dosya yalnızca birebir kopyalanıp dağıtılabilir, değiştirilemez | Morfoloji paketi orijinal QAC dosyasını **değiştirilmeden** ve telif notuyla birlikte içerir (`qac/quranic-corpus-morphology-0.4.txt` + `qac/NOTICE`). SQLite tabloları bu dosyadan türetilir; dönüşüm yalnızca formattır, hiçbir etiket eklenmez, silinmez veya düzeltilmez. |
| Kaynak açıkça belirtilmeli | Kelime kartı, Kelime paneli, konkordans sayfası ve dışa aktarmalarda "Kaynak: Quranic Arabic Corpus" ibaresi. |
| corpus.quran.com'a link verilmeli | Aynı yerlerde ve "Hakkında → Kaynaklar" ekranında link. |
| Telif notu, dosyanın önemli kısmını içeren türev çalışmalarda yeniden üretilmeli | `qac/NOTICE` paketle dağıtılır ve Kaynaklar ekranında tam metin olarak gösterilir. |
| SSS: verinin araştırma amaçlı, ticari olmayan kullanımı | Siyak ticari değildir. Paket ekranında ve `siyak-data/docs/sources.md`'de bu şart açıkça belirtilir; uygulamanın kodu (MIT/Apache) bu şarttan etkilenmez, çünkü veri ayrı paket olarak indirilir. |

**Sonuçları:**
- **Hata düzeltmesi yapılmaz.** Kullanıcıların bildirdiği etiket hataları pakete işlenmez; `siyak-data/docs/qac-errata.md` dosyasında toplanır, mümkünse QAC'ye iletilir. Arayüz, errata listesindeki kelimelerde "Bu etiket için bilinen bir not var" uyarısı gösterebilir; bu not ayrı bir dosyadan gelir ve QAC verisini değiştirmez.
- **Türkçe etiketler** (sözcük türü ve çekim özelliklerinin Türkçe/İngilizce okunur karşılıkları) uygulamanın kendi i18n dosyalarındadır; QAC verisi değil, onun gösterim katmanıdır.
- **Bakım:** QAC 2011'den beri güncellenmemektedir. Bu bir risk değil, sabitliktir: sürüm `0.4` olarak sabitlenir ve paketin `meta`'sında kaydedilir.
- **Transliterasyon:** QAC'nin Buckwalter transliterasyonu pipeline'da Arapça harflere çevrilir; bu çeviri, eşleme tablosuyla birlikte test edilir.

**Hizalama:** QAC kelime konumları (`surah:ayah:word:segment`) Tanzil Uthmani metniyle eşleştirilir. `validate.py` her ayette kelime sayısını ve segmentlerin birleşiminin Tanzil kelimesiyle eşleşmesini kontrol eder (hareke ve yazım farkları için belgelenmiş normalizasyon kurallarıyla). Uyuşmazlık varsa paket üretilmez.

**Gelecek: MASAQ değerlendirmesi.** MASAQ (CC BY 3.0, Tanzil tabanlı) serbest lisanslı bir alternatif ve geleneksel i'rab etiketleriyle ek bir katman olarak M7 sonrasında değerlendirilir: alanların incelenmesi (özellikle lemma), Tanzil hizalaması ve birkaç yüz kelimelik örnekte QAC ile karşılaştırma. Şema kaynaktan bağımsız olduğu için MASAQ, uygulama kodunda değişiklik gerekmeden QAC'ın yerine morfoloji paketi olabilir (v1'de aynı anda tek morfoloji paketi; ek bir i'rab katmanı ayrı bir paket tipi olur).

**Uygulama için notlar (M1b'den):** Kökler QAC'ın harfleriyle tutulur, örneğin `ءامن`'in kökü `ا م ن`; kök araması hemze biçimlerini normalizasyondan geçirmeli. Hizalama kuralı M ile birleşen dört kelimede `words.text_ar` iki token içerir. QAC lemmaları olduğu gibi aktarılır, `صَّمَد` (112:2) veya `ٱسْتَعِينُ` (2:153) gibi tuhaflıklar dahil; bunlar errata listesine girer, pakete asla işlenmez.

### 5.4 `content.db`: arama indeksleri

```sql
CREATE VIRTUAL TABLE verses_fts USING fts5(
  text_norm                                    -- normalize(text_clean): dağıtılan metin değil, arama indeksi
);
CREATE VIRTUAL TABLE translations_fts USING fts5(
  text, pack_id UNINDEXED, verse_id UNINDEXED,
  tokenize = 'unicode61 remove_diacritics 2'
);
```

Vektör tablosu bir migration'la değil, ilk vektör paketi kurulurken oluşturulur (paketler şema sürümüne göre denetlenir; bir migration yayımlanmış tüm paketlerin şema sürümünü geçersiz kılardı):

```sql
CREATE TABLE verse_vectors (
  pack_id    TEXT NOT NULL REFERENCES packs(id) ON DELETE CASCADE,
  verse_id   INTEGER NOT NULL REFERENCES verses(id),
  embedding  BLOB NOT NULL,             -- dim adet little-endian float32
  UNIQUE (pack_id, verse_id)
);
```

Aktif paketin vektörleri bir kez belleğe yüklenir ve her sorguda taranır (6.236 iç çarpım milisaniyeler sürer). Düz bir tablo SQLite eklentisi gerektirmez; böylece `content.db` her araçla okunabilir kalır ve eklenti yüklemek için `unsafe` kod gerekmez (ADR 0012).

Aynı anda en fazla bir vektör paketi aktiftir (`semantic.vector_pack` ayarı, §7.4).

### 5.5 `user.db`

```sql
CREATE TABLE bookmarks (
  id          INTEGER PRIMARY KEY,
  verse_id    INTEGER NOT NULL UNIQUE,
  created_at  TEXT NOT NULL
);

CREATE TABLE notes (
  id          INTEGER PRIMARY KEY,
  verse_from  INTEGER NOT NULL,                -- tek ayet için from = to
  verse_to    INTEGER NOT NULL,
  body        TEXT NOT NULL,                   -- Markdown
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);
CREATE INDEX notes_range ON notes(verse_from, verse_to);

-- Araştırma koleksiyonları
CREATE TABLE collections (
  id           INTEGER PRIMARY KEY,
  title        TEXT NOT NULL,
  description  TEXT,                           -- Markdown
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL
);

CREATE TABLE collection_items (
  id             INTEGER PRIMARY KEY,
  collection_id  INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  position       INTEGER NOT NULL,             -- kullanıcı sıralaması
  kind           TEXT NOT NULL CHECK (kind IN ('verse_range','note','root','lemma','tafsir')),
  ref_json       TEXT NOT NULL,                -- örn. {"from":262,"to":263} / {"root":"ر ح م"}
  annotation     TEXT,                         -- öğeye özel kısa not
  created_at     TEXT NOT NULL
);
CREATE INDEX collection_items_order ON collection_items(collection_id, position);

-- Tefsir özet/çeviri cache'i
CREATE TABLE tafsir_translations (
  pack_id        TEXT NOT NULL,
  tafsir_id      INTEGER NOT NULL,
  paragraph_idx  INTEGER NOT NULL DEFAULT -1,  -- -1: tüm pasajın özeti (NULL değil: SQLite birincil anahtarda NULL'ları birbirinden farklı sayar)
  mode           TEXT NOT NULL CHECK (mode IN ('summary','full')),
  target_lang    TEXT NOT NULL,
  pack_version   TEXT NOT NULL,                -- kaynak değişirse cache geçersiz
  text           TEXT NOT NULL,
  provider       TEXT NOT NULL,
  model          TEXT NOT NULL,
  created_at     TEXT NOT NULL,
  PRIMARY KEY (pack_id, tafsir_id, paragraph_idx, mode, target_lang)
);

-- AI harcama kaydı
CREATE TABLE llm_usage (
  id             INTEGER PRIMARY KEY,
  feature        TEXT NOT NULL CHECK (feature IN ('chat','translate')),
  provider       TEXT NOT NULL,
  model          TEXT NOT NULL,
  input_tokens   INTEGER NOT NULL,
  output_tokens  INTEGER NOT NULL,
  cost_usd       REAL,                         -- fiyat bilinmiyorsa NULL
  created_at     TEXT NOT NULL
);

CREATE TABLE chat_sessions (
  id          INTEGER PRIMARY KEY,
  title       TEXT,
  provider    TEXT NOT NULL,
  model       TEXT NOT NULL,
  created_at  TEXT NOT NULL
);

CREATE TABLE chat_messages (
  id            INTEGER PRIMARY KEY,
  session_id    INTEGER NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
  role          TEXT NOT NULL CHECK (role IN ('user','assistant')),
  content       TEXT NOT NULL,
  cited_verses  TEXT,                          -- JSON: [262, 263]
  sources_json  TEXT,                          -- RAG'a giren kaynaklar
  created_at    TEXT NOT NULL
);

CREATE TABLE settings (
  key    TEXT PRIMARY KEY,
  value  TEXT NOT NULL                         -- JSON
);
```

`user.db` içindeki ayet referansları `content.db`'ye FK ile bağlanamaz (ayrı DB). Bütünlük uygulama katmanında sağlanır.

### 5.6 Migration
Her iki DB için `PRAGMA user_version` tabanlı, sıralı SQL migration'lar. `user.db` migration'ından önce `user.db.bak` yedeği alınır.

---

## 6. Paket sistemi

### 6.1 Paket tipleri

| Tip | Format | İçerik |
|---|---|---|
| `translation` | `.sqlite.zst` | `translations` şemasına uygun tablo |
| `tafsir` | `.sqlite.zst` | `tafsir` + `tafsir_paragraphs` |
| `morphology` | `.tar.zst` (`.sqlite` + `qac/`) | `words`, `roots`, `lemmas`, `word_segments`; QAC için ayrıca orijinal dosya ve `NOTICE` (§5.3.1) |
| `vectors` | `.sqlite.zst` | `(verse_id INTEGER, embedding BLOB)`, little-endian float32; `meta` vektör uzayını ve backend'i belirtir (aşağıda) |
| `model` | `.tar.zst` | `model.sqlite` (yalnızca `pack_info`), `model/model.onnx`, `model/tokenizer.json`; modelin ayarları `meta`'dadır. Yalnızca `local` backend'li vektör paketleri için |

**Vektör paketi meta verisi** (ADR 0012). Vektör paketi kendi vektör uzayını tanımlar: backend önekiyle kanonik `embedding_model` (`local/…`, `openai/…`, `voyage/…`, `gemini/…`, `compat/…`), `dim` ve `normalized`. `backend` sorguların nerede embed edileceğini söyler:
- `local`: `model_pack` model paketini belirtir; bu paket `requires` içinde de yer alır. `embedding_model` ve `dim` değerleri eşleşmelidir.
- `api`: `provider` (`openai`, `voyage`, `gemini`, `openai_compatible`), `api_model` (sağlayıcının model kimliği) ve isteğe bağlı `api_dimensions` (vektörlerini kısaltabilen modeller için, `dim`'e eşit). Model paketi yoktur. Vektörler pipeline'da bir kez, bakımcıların anahtarıyla üretilir.

Meta verisi geçersiz bir vektör paketi "desteklenmiyor" olarak listelenir; manifestin geri kalanı kullanılmaya devam eder.

Tüm SQLite paketleri zstd ile sıkıştırılmış dağıtılır ve bir `pack_info` tablosu taşır; ayrıntılı format `siyak-data/docs/pack-format.md`'dedir. Arşiv paketleri ek dosyalarını `<app_data>/packs/<pack-id>/` altında tutar (örn. QAC'ın `NOTICE`'u, Hakkında → Kaynaklar'da tam gösterilir). v1'de aynı anda en fazla bir morfoloji paketi kurulu olabilir.

### 6.2 Gömülü içerik
`core.db` installer'a gömülür ve şunları içerir: Tanzil Arapça metin (Uthmani + Simple Clean) ve metadata, bir Türkçe meal ve Pickthall (EN). Böylece uygulama ilk açılışta, internet olmadan kullanılabilir. Türkçe mealin Elmalılı Hamdi Yazır meali olması planlanıyor; orijinal metnin temiz ve lisansı net bir kaynağının doğrulanmasına bağlı (§18).

**Çekirdek yenileme.** `core.db`, `content_meta.core_version` taşır (tam sayı; `siyak-data` çekirdek içerik her değiştiğinde artırır). Açılışta, installer'la gelen `core.db`'nin `core_version`'ı `content.db`'dekinden büyükse:
1. `content.db`, `content.db.bak` olarak kopyalanır.
2. `core.db` attach edilmiş halde, tek transaction içinde `surahs`, `verses`, `verses_fts`, `notices` ve gömülü paketlerin satırları (`packs.bundled = 1` ve bunların `translations` ile `translations_fts` satırları) `core.db`'den yenilenir; `content_meta` güncellenir.
3. Kullanıcının kurduğu paketler (`bundled = 0`) ve tüm tabloları olduğu gibi kalır. Ayet ID'leri (1..6236) hiç değişmediği için bu güvenlidir.
4. Herhangi bir hatada transaction geri alınır ve yedek korunur; uygulama eski çekirdekle çalışmaya devam eder ve hata kodunu bildirir.


### 6.3 Manifest
Adresler (uygulamaya gömülü, sırayla denenir):
1. **Birincil:** `https://quran.kemalozyon.com/data/manifest.json`
2. **Yedek:** `https://github.com/siyak-app/siyak-data/releases/latest/download/manifest.json`

Birincil adres GitHub'daki organizasyon/repo adından bağımsızdır; ileride yeniden adlandırma eski uygulama sürümlerini kırmaz. Web sitesi workflow'u, her yeni veri release'inde (`repository_dispatch` ile tetiklenir) ve her site deploy'unda `siyak-data`'nun son release'inden `manifest.json` ve `pricing.json`'u `website/public/data/` altına kopyalar. Manifest içindeki paket `url`'leri değişmez (immutable) release asset adresleridir; paket dosyaları GitHub Releases'tan indirilir.

```json
{
  "schema_version": 1,
  "generated_at": "2026-10-21T00:00:00Z",
  "pricing_url": "https://quran.kemalozyon.com/data/pricing.json",
  "packs": [
    {
      "id": "en.pickthall",
      "type": "translation",
      "lang": "en",
      "title": "The Meaning of the Glorious Koran",
      "author": "Marmaduke Pickthall",
      "version": "1.0.0",
      "min_app_version": "0.1.0",
      "size": 1900000,
      "sha256": "…",
      "url": "https://github.com/siyak-app/siyak-data/releases/download/data-2026.10.0/en.pickthall.sqlite",
      "license": "Public domain",
      "license_terms": null,
      "attribution": null,
      "citation": { "type": "translation", "author": "…", "title": "…", "publisher": "…", "year": "…" }
    },
    {
      "id": "vectors.<model>",
      "type": "vectors",
      "requires": ["model.<model>"],
      "meta": { "embedding_model": "local/<model>", "dim": 384, "source_text": "<bkz. §18>", "normalized": true,
                "backend": "local", "model_pack": "model.<model>" }
    },
    {
      "id": "model.<model>",
      "type": "model",
      "meta": { "embedding_model": "local/<model>", "dim": 384, "query_prefix": "query: ", "passage_prefix": "passage: ",
                "max_tokens": 512, "pooling": "mean" }
    },
    {
      "id": "vectors.openai-3-small",
      "type": "vectors",
      "terms_ack": true,
      "meta": { "embedding_model": "openai/text-embedding-3-small", "dim": 512, "source_text": "<bkz. §18>", "normalized": true,
                "backend": "api", "provider": "openai", "api_model": "text-embedding-3-small", "api_dimensions": 512 }
    }
  ],
  "bundles": [
    {
      "id": "recommended",
      "title": { "tr": "Önerilen paketler", "en": "Recommended packs" },
      "description": { "tr": "Kök analizi, anlamsal arama ve bir tefsir", "en": "Root analysis, semantic search and a tafsir" },
      "packs": ["morphology.qac", "vectors.<model>", "model.<model>", "ar.jalalayn"]
    }
  ]
}
```

**Bundles:** Kullanıcıya teker teker paket seçtirmek yerine tek tuşla kurulan setler. Sade kullanıcı ilk açılışta "Önerilen paketler" kartını görür ve toplam boyutla birlikte tek tuşla kurar. "Vektör paketi", "model paketi" gibi kavramlarla karşılaşmaz; tek tek paket seçimi Paketler ekranında "Gelişmiş" altında kalır. Paket ve bundle başlıkları `{ "tr", "en" }` nesnesi olarak verilebilir.

### 6.4 Kurulum akışı
1. Manifest'i çek: açılışta ve paket ekranında, 1 saat cache'li. Ağ yoksa son başarılı manifest kullanılır.
2. `min_app_version` kontrolü yap; uyumsuz paketler "uygulamayı güncelle" uyarısıyla listelenir.
3. `requires` bağımlılıklarını çöz ve kullanıcıya toplam boyutu göster.
4. `<app_data>/downloads/<id>.part` dosyasına stream ederek indir. İlerleme `pack://progress` event'iyle bildirilir. İndirme iptal edilebilir; HTTP Range ile kaldığı yerden devam eder.
5. **sha256 doğrula.** Uyuşmazsa dosyayı sil ve hata ver.
6. Kur:
   - SQLite paketleri: tek transaction içinde `ATTACH` → mevcut `pack_id` satırlarını sil → yalnızca beklenen tablo/kolonlardan `INSERT … SELECT` → FTS güncelle → `packs` upsert → `DETACH`. Vektör paketi önce denetlenir: `content.db`'deki her ayet için bir vektör, her biri `dim` adet sonlu değer, `normalized` ise birim uzunlukta.
   - `model` (ve diğer arşiv paketleri): geçici klasöre aç, sonra atomik rename ile `<app_data>/packs/<id>/` altına taşı.
7. İndirilen dosyayı sil.

**Güncelleme:** Aynı akışla yeniden kurulum. Tefsir güncellenirse `tafsir_translations` içindeki eski `pack_version` kayıtları "kaynak güncellendi" etiketiyle gösterilir; kullanıcı yeniden çevirebilir.
**Kaldırma:** `bundled = 0` olan paketler kaldırılabilir; ardından `VACUUM` önerilir.

### 6.5 Lisans
- Her paketin `license`, `attribution` ve kaynak linki "Hakkında → Kaynaklar" ekranında listelenir. Paket ekranında indirmeden önce de gösterilir.
- Kaynağın gerektirdiği atıflar (ör. Tanzil CC BY 3.0) her zaman görünür olur.
- Kod lisansı (MIT/Apache) veri paketlerinin lisansını değiştirmez. Daha kısıtlayıcı lisanslı paketler (örn. QAC morfoloji paketi: değiştirilmeme ve araştırma amaçlı kullanım şartları) ayrı paket olarak dağıtılır ve paket ekranında belirgin şekilde işaretlenir. Manifest'e `license_terms` alanı (kısa şart listesi, TR/EN) eklenir; paket ekranı bunu indirmeden önce gösterir.

---

### 6.6 Tefsir paketleri: kaynak ve seçim

**Kaynak: Quranpedia veri dökümleri** (`quranpedia.net/dumps`). Ayet bazında anahtarlı, kitap başına `.json.gz`, SHA-256 hash'li, sürümlü (`built_at`) ve bir değişiklik API'si olan 149 tefsir kitabı.

**Quranpedia lisansının uygulanışı:**

| Şart | Uygulanışı |
|---|---|
| Klasik tefsirler ortak miras; Quranpedia'nın derleme emeği uygulama ve araştırma araçlarında serbest | Klasik tefsir paketleri bu kapsamda dağıtılır |
| Çeviriler ve çağdaş eserler yazar ve yayıncılarına aittir | v1'e çağdaş eser alınmaz (aşağıdaki kural) |
| İndirilebilir veri seti olarak yeniden dağıtımda Quranpedia.net link ve döküm sürümüyle kaynak gösterilmeli | Her tefsir paketinin `attribution` alanı: "Kaynak: Quranpedia.net · döküm <sürüm>" + link; paket ekranında, tefsir panelinde ve Kaynaklar ekranında gösterilir. `siyak-data/docs/sources.md`'de de yer alır |
| Güncel olmayan metin dağıtmanın sorumluluğu dağıtıcıdadır | Veri pipeline'ı her release'de değişiklik API'sini sorgular; değişen kitapların paketleri yeni sürümle yayınlanır. Uygulama, kurulu tefsir paketlerinin güncellemesini bildirir |
| Veri olduğu gibi sağlanır | Arayüzde "metin kaynağın baskısıdır; hata görürseniz bildirin" notu; hatalar Quranpedia'ya iletilir |

**Seçim kuralı (v1):** Yalnızca modern dönem öncesi (müellifi en geç 19. yüzyılda vefat etmiş) klasik eserler. 20. yüzyıl ve sonrası eserler (Elmalılı, Merâgî, Menâr, Şa'râvî, Sâbûnî, Zuhaylî, Tefsîr-i Müyesser vb.) v1'e alınmaz; sonraki sürümlerde eser bazında lisans durumuna göre değerlendirilir.

**Doğrulama:** Quranpedia'da aynı başlık birden fazla ID ile geçebilir (farklı eser ya da baskı). Pipeline'da her paket için müellif, eser ve baskı bilgisi kitap meta verisinden tek tek doğrulanır ve `siyak-data/sources/tafsir/<pack-id>/README.md`'ye yazılır. ID eşlemesi doğrulanmadan paket üretilmez.

**v1 tefsir listesi:**

| Paket ID | Eser | Müellif (vefat) | Not |
|---|---|---|---|
| `ar.tabari` | Câmi'u'l-beyân | Taberî (310/923) | Rivayet tefsirinin ana kaynağı |
| `ar.ibn-kathir` | Tefsîru'l-Kur'âni'l-azîm | İbn Kesîr (774/1373) | En çok okunan klasik tefsir; **varsayılan sohbet tefsiri** |
| `ar.razi` | Mefâtîhu'l-gayb | Fahreddin Râzî (606/1210) | Dirayet ve kelâm; çok büyük |
| `ar.zamakhshari` | el-Keşşâf | Zemahşerî (538/1144) | Dil ve belâgat; Mu'tezilî yaklaşım paket açıklamasında belirtilir |
| `ar.baydawi` | Envâru't-tenzîl | Beyzâvî (685/1286 civ.) | Osmanlı medreselerinin standart tefsiri |
| `ar.jalalayn` | Tefsîru'l-Celâleyn | Mahallî (864/1459) & Suyûtî (911/1505) | Kısa; **varsayılan sohbet tefsiri** |
| `ar.baghawi` | Meâlimü't-tenzîl | Begavî (516/1122) | Orta hacimli rivayet tefsiri |
| `ar.nasafi` | Medâriku't-tenzîl | Nesefî (710/1310) | Hanefî-Mâtürîdî çizgisi |
| `ar.maturidi` | Te'vîlâtü Ehli's-sünne | Mâtürîdî (333/944) | Mâtürîdî kelâm geleneği |
| `ar.abussuud` | İrşâdü'l-akli's-selîm | Ebüssuûd Efendi (982/1574) | Osmanlı şeyhülislamı |
| `ar.bursevi` | Rûhu'l-beyân | İsmail Hakkı Bursevî (1137/1725) | Osmanlı; işârî yorum |
| `ar.alusi` | Rûhu'l-meânî | Şihâbüddîn Mahmûd el-Âlûsî (1270/1854) | Klasik birikimin son büyük sentezi; çok büyük. Quranpedia meta verisi torunu Mahmûd Şükrî el-Âlûsî'yi (ö. 1924) gösteriyor; paket README'sinde düzeltildi, Quranpedia'ya bildirildi |
| `ar.suyuti-durr` | ed-Dürrü'l-mensûr | Suyûtî (911/1505) | Rivayet derlemesi; her ayeti şerh etmez (~4.400 ayet) |
| `ar.ibn-atiyya` | el-Muharrerü'l-vecîz | İbn Atıyye (541/1147) | Endülüs geleneği |
| `ar.abu-hayyan` | el-Bahrü'l-muhît | Ebû Hayyân (745/1344) | Nahiv ağırlıklı |
| `ar.shawkani` | Fethu'l-kadîr | Şevkânî (1250/1834) | Rivayet ve dirayet birlikte |
| `ar.qurtubi` | el-Câmi' li-ahkâmi'l-Kur'ân | Kurtubî (671/1273) | Ahkâm tefsiri; kaynak QUL, yayını Tarteel'in yanıtına bağlı (aşağıya bkz.) |

Paket açıklamaları (TR/EN) eserin yöntemini ve öne çıkan yönünü tek cümleyle anlatır (rivayet, dirayet, dil, fıkıh, işârî), böylece sade kullanıcı da hangi tefsiri neden seçeceğini anlar.

**Kurtubî — kaynak: QUL (Tarteel), kaynak #23.** Quranpedia tefsir dökümlerinde yer almadığı için Kurtubî QUL'dan alınır. QUL verisi ayet gruplu (`from_ayah`, `to_ayah`, `group_ayah_key`) geldiği için §5.2'deki aralık şemasına doğrudan dönüşür; metindeki HTML etiketleri (`<b>`, `<i>`) pipeline'da güvenli bir alt kümeye indirgenir, içerik değiştirilmez.

- **İzin:** QUL bu kaynak için köken veya lisans belirtmiyor; SSS'si her kaynağın lisansının ayrıca kontrol edilmesini istiyor. Tarteel'e kaynağın kökeni (baskı/tahkik) ve açık kaynak, ticari olmayan bir uygulamada değiştirilmeden yeniden dağıtım izni için yazılı talep gönderilir.
- **Teknik entegrasyon izni beklemez:** Pipeline'a `build_tafsir_qul.py` eklenir, paket geliştirme ortamında üretilir ve test edilir.
- **Yayın kapısı:** `ar.qurtubi` paketi, Tarteel'den yanıt gelene kadar veri release'ine dahil edilmez (`manifest`'te yer almaz). Yanıt olumluysa bir sonraki veri release'iyle yayınlanır; uygulama sürümü gerekmez. Yanıtın içeriği (izin metni, istenen atıf biçimi) `siyak-data/sources/tafsir/ar.qurtubi/PERMISSION.md`'ye kaydedilir.
- **Atıf:** "Kaynak: Quranic Universal Library (Tarteel)" + link, ve yanıtta belirtilirse köken baskı.
- Kaldırma talebi gelirse paket bir sonraki veri release'inde manifest'ten çıkarılır.

## 7. Arama, kelime analizi ve çapraz referans

### 7.1 Tek arama kutusu ve modlar
Uygulamada tek bir arama kutusu vardır. Modlar: **Tümü** (varsayılan) · **Kelime** · **Kök** · **Anlam**. Sade varsayılanda mod çipleri gösterilmez, yalnızca Tümü davranışı çalışır; sonuç sayfasındaki "tümünü gör" bağlantıları ilgili moda geçirir.

**Tümü — sorgu sınıflandırma ve gruplu sonuçlar.** `classify_query` (Rust) sorguyu şu sırayla sınıflandırır:

| Girdi | Örnek | Davranış |
|---|---|---|
| Ayet referansı | `2:255`, `2:153-157`, `Bakara 255` | Arama yapılmaz; okuma ekranında ilgili ayete gidilir |
| Arapça harfler | `صبر`, `الصبر`, `ص ب ر` | En üstte kök/sözlük biçimi kartı, altında Arapça metin eşleşmeleri |
| Latin harfli kök | `s-b-r` | Kök kartı |
| Diğer metin | `sabır`, `zor zamanlarda sabretmek` | Önce meallerde birebir eşleşmeler, ardından "Anlamca yakın" grubu |

Her grup ilk 3–5 sonucu gösterir; "tümünü gör" ilgili modun tam sonuç sayfasını açar. Gerekli paket kurulu değilse o grup, sonuç yerine paket indirme kartı gösterir.

**Kelime modu:** §7.2. **Kök modu:** §7.3. **Anlam modu:** §7.4.

**Paket gerektiren modlar:** Kök (morfoloji) ve Anlam (model + vektör) çipleri paket yoksa gizlenmez veya devre dışı görünmez. Tıklanınca "Bu arama için X paketi gerekli (boyut) · İndir" kartı gösterilir. Böylece kullanıcı özelliğin varlığından haberdar olur.

### 7.2 Kelime araması (FTS5)
- **Meal içinde:** `translations_fts`, seçili meal paketleriyle sınırlı.
- **Arapça içinde:** `verses_fts`. Sorgu, pipeline'daki harekesizleştirme kurallarının Rust karşılığından geçer. İki implementasyon `tests/normalize_vectors.json` ile birlikte test edilir. Hareke farkı yok sayılır (`الصبر` sorgusu `الصَّبْرِ`'yi bulur).
- Tırnak içinde tam ifade araması: `"sabredenlerle beraberdir"`.
- Filtreler: meal paketleri, sure aralığı, Mekkî/Medenî.
- Sonuçlar `snippet()` ile vurgulu döner.

### 7.3 Kök ve sözlük biçimi (lemma) araması
- Girdi: Arapça kök (`ص ب ر`, `صبر`), sözlük biçimi ya da Latin harfli yazım (`s-b-r`).
- Sonuç bir konkordanstır: kökün sözlük biçimleri ve her birinin geçiş sayısı, sure bazında dağılım, bağlamıyla ayet listesi.
- Sözlük biçimi listesinden seçim yapılarak sonuç daraltılabilir.
- Kelime panelindeki "Kökün geçtiği yerler" bağlantısı aynı sayfayı açar.

### 7.4 Anlamsal arama

```rust
pub enum TextKind { Query, Passage }
pub enum Locality { Local, Remote(ApiProvider) }

pub trait Embedder: Send + Sync {
    fn space(&self) -> &EmbeddingSpace;   // embedding_model, dim, normalized
    fn locality(&self) -> Locality;
    fn embed(&self, texts: Vec<String>, kind: TextKind) -> EmbedFuture<'_>;  // kutulu future
}
```

- **İki backend** (ADR 0012); hangisinin kullanılacağını aktif vektör paketinin meta verisi belirler (§6.1):
  - **Lokal (varsayılan):** `OnnxEmbedder`, model paketinin meta verisine göre yapılandırılır (önekler, pooling, token sınırı). İnternetsiz ve anahtarsız çalışır; çıkarım ayrı bir thread'de yapılır.
  - **API:** OpenAI, Voyage, Gemini ya da OpenAI-uyumlu bir sunucu (Ollama, LM Studio). Sağlayıcının keychain'deki anahtarını kullanır (§14.2); bu anahtar AI katmanıyla ortaktır (§8). OpenAI-uyumlu sunucu için kullanıcının girdiği adres gerekir (`semantic.base_url`). Anthropic'in embedding modeli olmadığı için Claude listede yoktur.
- **Sorgu ve pasaj** farklı embed edilir (e5 `query: `/`passage: ` önekleri, Voyage `input_type`, Gemini görev türleri); her backend `TextKind`'ı kendisi eşler.
- **Aktif paket:** Aynı anda en fazla bir vektör paketi aktiftir: `semantic.vector_pack` ayarında adı geçen, yoksa ilk kurulu paket.
- **Durum:** `get_semantic_status`, anlamsal özelliklerin neden kullanılabildiğini ya da kullanılamadığını döner: `no_vector_pack`, `invalid`, `model_pack_missing`, `mismatch`, `key_missing`, `base_url_missing`, `not_built` veya `ready` (sorgular bilgisayardan çıkıyorsa sağlayıcıyla birlikte). Arayüz her birini bir eyleme çevirir (kur, anahtar ekle, adres gir).
- **Lazy-load:** Embedder ilk kullanımda oluşturulur; paketlerden birinin sürümü ya da adres değişene kadar tutulur.
- **Uyumluluk:** Embedder'ın `embedding_model` ve `dim` değerleri aktif vektör paketininkilerle eşleşmezse anlamsal özellikler devre dışı kalır ve arayüz nedenini gösterir.
- **Gizlilik:** API paketiyle sorgular (sohbette ayrıca sıralanan tefsir paragrafları, §10) `net.rs` üzerinden sağlayıcıya gönderilir (§14.1). Bu paketler "Gelişmiş" altındadır ve indirmeden önce bunu söyler (`terms_ack`).
- **Sorgu:** Sorgu vektörünün aktif paketin her ayet vektörüyle kosinüs benzerliği, bellekte (§5.4); en iyi `k` sonuç, istenirse bazı ayetler hariç (çapraz referansın başladığı ayet gibi).
- Embedding'ler **ayet bazında** hesaplanır. Tefsir hiçbir zaman embed edilmez; sohbete ait olduğu ayetler üzerinden gider (§10).

### 7.5 Kelime paneli
Okuma ekranında bir kelimeye tıklanınca açılır:
- Segment segment morfoloji: ön ek/gövde/son ek, sözcük türü, kök, lemma, özellikler (insan-okunur Türkçe/İngilizce etiketlerle).
- "Bu kökün geçtiği yerler (N)" ve "Bu lemmanın geçtiği yerler (N)" bağlantıları konkordansa gider.
- Kelime, kökü veya lemması koleksiyona eklenebilir.

### 7.6 Çapraz referans
Bir ayet için ilişkili ayetler iki kaynaktan birleştirilir:
1. **Anlamsal benzerlik:** Ayetin kendi vektörüyle KNN (k=10, ayetin kendisi hariç). Vektör paketi gerektirir.
2. **Ortak kök/lemma:** Ayetteki kök/lemmaların ters sıklığına (IDF) göre ağırlıklı ortaklık skoru. Morfoloji paketi gerektirir. Böylece çok sık geçen kökler (örn. "قول") sonucu domine etmez.

Sonuçlar iki ayrı bölümde ve gerekçesiyle gösterilir ("anlamca benzer", "ortak kökler: ر ح م, غ ف ر"). Kullanıcı neden ilişkili bulunduğunu her zaman görür. Hazır, küratörlü bir çapraz referans veri seti bulunursa üçüncü kaynak olarak eklenir (§18).

---

## 8. AI sağlayıcı katmanı

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    async fn list_models(&self) -> Result<Vec<String>>;
    async fn stream(&self, req: LlmRequest, out: Channel<LlmEvent>) -> Result<Usage>;
}
```

| Sağlayıcı | Not |
|---|---|
| Anthropic | Messages API, streaming |
| OpenAI | streaming |
| Google Gemini | streaming |
| OpenAI-uyumlu | Kullanıcı `base_url` girer. Ollama/LM Studio ile tamamen offline; OpenRouter gibi aggregator'larla tek anahtarla çok model. Anahtar opsiyonel. |

**Model rolleri** (ayarlarda ayrı seçilir):
- `chat_model`: sohbet için.
- `translation_model`: tefsir özet/çeviri için. Varsayılan olarak seçilen sağlayıcının en ucuz modeli önerilir.

**Model yetenek bayrakları** (kullanıcı ayarlayabilir, bilinen modeller için varsayılanları vardır):
- `arabic_sources`: Model klasik Arapça kaynakları güvenilir şekilde okuyabilir mi? Lokal/küçük modellerde varsayılan olarak kapalıdır.

---

## 9. Tefsir özet ve çevirisi

### 9.1 Modlar
- **Özet (varsayılan):** Pasajın kısa Türkçe özeti. Çıktı kısa olduğu için ucuzdur.
- **Tam çeviri:** Paragraf paragraf. İlk paragraf çevrilir; devamı "devamını çevir" ile gelir.

### 9.2 Akış
1. Cache'e bak (`tafsir_translations`). Varsa ücretsiz göster.
2. Yoksa butonun yanında **tahmini maliyeti** göster: girdi tokenları lokalde tokenizer yaklaşımıyla tahmin edilir, çıktı için moda göre bir tahmin kullanılır, sonuç `pricing.json` ile çarpılır. Örn. "Özetle (≈ 0,3 ¢)".
3. Kullanıcı onaylarsa `translation_model` ile istek atılır ve sonuç stream edilir.
4. Sonuç cache'lenir, gerçek token sayıları `llm_usage`'a yazılır.

### 9.3 Gösterim kuralları
- Çeviri/özet her zaman **orijinalin yanında** gösterilir, asla onun yerine geçmez.
- Kalıcı etiket: "Makine çevirisi · <sağlayıcı>/<model> · <tarih>". Tıklanınca ilgili orijinal paragraf vurgulanır.
- Sistem promptu: terimleri (fıkhi, kelami) çevirirken orijinal Arapça terimi parantez içinde koru; yorum ekleme; emin olmadığın ifadeyi işaretle.

### 9.4 Harcama kontrolü
- Ayarlar → AI → Harcama: bu ay ve toplam harcama (özellik ve model bazında), `llm_usage`'dan hesaplanır.
- Opsiyonel aylık limit: limite ulaşılınca işlem öncesi uyarı verilir (engellemez, onay ister).
- `pricing.json` `siyak-data` release'inde tutulur; uygulama sürümü çıkarmadan güncellenebilir. Fiyatı bilinmeyen modelde (örn. lokal) maliyet "bilinmiyor/ücretsiz" olarak gösterilir.

```json
{
  "updated_at": "2026-10-21",
  "currency": "USD",
  "models": {
    "anthropic/<model>": { "input_per_mtok": 0.0, "output_per_mtok": 0.0 }
  }
}
```

### 9.5 v1 dışında: topluluk çevirileri
Kullanıcının bir çeviriyi `siyak-data`'ya katkı olarak göndermesi ve incelenen çevirilerin "topluluk çevirisi" paketi olarak dağıtılması. İnceleme süreci tanımlanmadan uygulanmaz.

---

## 10. AI sohbet (RAG)

1. Kullanıcı mesajını embed et ve aktif vektör paketinden top-k (varsayılan 8) ayeti getir.
2. Her ayet için Arapça metni ve seçili meal(ler)i ekle.
3. Bulunan ayetlerin tefsirlerini **otomatik olarak** ekle, bağlam bütçesiyle sınırlı:
   - **Sohbet tefsirleri** ayrı seçilir (`settings.chat.tafsirs`, en fazla 3). Okuma ekranında kurulu tüm tefsirler kullanılabilir; sohbete yalnızca bu seçim gider. Varsayılan: Celâleyn (kısa) + İbn Kesîr (orta hacimli). Seçili tefsirin paketi kurulu değilse atlanır ve arayüz bunu belirtir.
   - **Ayet sırasına göre bütçe:** Toplam tefsir bütçesi (varsayılan ~6.000 token, modelin bağlam penceresine göre ayarlanır) en alakalı ilk 3 ayete dağıtılır; kalan ayetlere yalnızca Arapça metin ve meal gider.
   - **Uzun pasajlarda paragraf seçimi:** Bir pasaj ayet başına bütçeyi aşarsa, `tafsir_paragraphs` o anda aktif embedder'la soruya göre sıralanır (§7.4; API paketinde paragraflar sağlayıcıya gönderilir) ve en alakalı paragraflar, orijinal sıralarıyla, bütçe dolana kadar eklenir. Tefsir için kalıcı vektör saklanmaz; yalnızca o pasajın birkaç düzine paragrafı anlık embed edilir. Atlanan paragraflar bağlamda "[…]" ile işaretlenir.
   - **Dil:** Arayüz dilindeki tefsirler her zaman eklenebilir. Arapça tefsirler yalnızca `chat_model.arabic_sources = true` ise eklenir; model aktarım yaparken orijinal ifadeyi de verir. Bayrak kapalıysa arayüz "Seçili model Arapça tefsirleri okuyamıyor; yalnızca meal kullanıldı" uyarısı gösterir.
   - **Maliyet:** Bağlam, kullanıcının belirlediği eşiği aşarsa gönder butonunun yanında tahmini maliyet gösterilir (§9.4).
4. Sistem promptu:
   - Yalnızca verilen bağlama dayan, her iddia için `[sure:ayet]` atfı ver; tefsirden aktarımda tefsir adını belirt.
   - Bağlam yetersizse açıkça söyle.
   - Fıkhi hüküm/fetva verme; bu tür sorularda ilim ehline yönlendir.
5. Yanıttaki `[sure:ayet]` referansları parse edilir, `cited_verses` alanına yazılır ve tıklanabilir link olarak gösterilir. RAG'a giren tüm kaynaklar `sources_json`'a kaydedilir ve "kullanılan kaynaklar" olarak açılabilir.
6. Vektör paketi kurulu değilse sohbet çalışmaz; arayüz paket indirmeyi önerir.
7. Sohbet cevabı veya bir kısmı koleksiyona not olarak eklenebilir ("AI cevabı" etiketiyle).

Kalıcı uyarı: "Yanıtlar yapay zekâ tarafından üretilir, hata içerebilir; dini hüküm yerine geçmez."

---

## 11. Koleksiyonlar ve dışa aktarma

### 11.1 Koleksiyonlar
- Bir araştırma konusu = bir koleksiyon (örn. "Kuran'da sabır").
- Öğeler: ayet/aralık, not, kök, lemma, tefsir pasajı. Sürükle-bırak ile sıralanır; her öğeye kısa açıklama eklenebilir.
- Okuma, arama, konkordans, çapraz referans ve sohbet ekranlarının hepsinde "koleksiyona ekle" eylemi bulunur.

### 11.2 Dışa aktarma
- **Markdown:** Koleksiyon başlığı ve açıklaması; her öğe için Arapça metin, seçili meal(ler), kullanıcının notları ve kaynakça. Dosya kaydetme diyaloğu Rust tarafından açılır.
- **Atıf metni:** Ayet ve meal/tefsir için kopyalanabilir atıf. Paketlerin `citation` meta verisinden üretilir. Şablonlar ayrı dosyalarda tutulur (`export/citation/*.tmpl`), böylece yeni stil eklemek kod değişikliği gerektirmez. v1 stilleri: İSNAD ve APA (geçici, §18).
- **JSON yedek:** Yer imleri, notlar, koleksiyonlar ve tefsir çeviri cache'i. Sohbet geçmişi ve API anahtarları dahil edilmez.

```json
{
  "format": "siyak-userdata",
  "version": 1,
  "exported_at": "…",
  "bookmarks": [{ "surah": 2, "ayah": 255, "created_at": "…" }],
  "notes": [{ "from": "2:255", "to": "2:255", "body": "…", "created_at": "…", "updated_at": "…" }],
  "collections": [{ "title": "…", "description": "…", "items": [{ "kind": "verse_range", "ref": { "from": "2:153", "to": "2:157" }, "annotation": "…" }] }],
  "tafsir_translations": [{ "pack_id": "…", "surah": 2, "ayah": 255, "paragraph_idx": 0, "mode": "summary", "text": "…", "provider": "…", "model": "…", "pack_version": "…" }]
}
```

Taşınabilirlik için dışa aktarmada `verse_id` yerine `sure:ayet` kullanılır. `note` ögesi, notların taşınabilir bir kimliği olmadığı için nota dosyadaki `notes` listesindeki sırasıyla başvurur (`{ "note": 3 }`). Notlar, koleksiyonlar ve ögeler zaman damgalarını, tefsir çevirileri de `target_lang` ve `created_at` alanlarını taşır; böylece yedek kaydettiğini geri getirir. İçe aktarma `merge` veya `replace` modunda çalışır ve bir rapor döner; aynı dosyayı iki kez birleştirmek bir şey eklemez, `replace` ise önce `user.db` dosyasının bir kopyasını yanında tutar.

---

## 12. IPC sözleşmesi

Tüm komutlar `Result<T, AppError>` döner (`{ code, message }`). Tipler `specta` + `tauri-specta` ile `src/lib/bindings.ts`'e otomatik aktarılır.

| Alan | Komutlar |
|---|---|
| İçerik | `list_surahs`, `get_verses(surah, translation_ids[])` (morfoloji paketi kuruluysa her ayetin kelimeleriyle), `get_verse_words(verse_id)`, `get_compare(from, to, translation_ids[], tafsir_ids[])` |
| Tefsir | `list_tafsirs`, `get_tafsir(verse_id, pack_id)`, `get_tafsir_introduction(pack_id)`, `estimate_tafsir_translation(tafsir_id, paragraph_idx?, mode)`, `translate_tafsir(…, channel)` |
| Morfoloji | `get_morphology_pack`, `get_word(word_id)`, `concordance(root_id \| lemma_id, translation_id?, limit, offset)`, `search_roots(query)` |
| Arama | `classify_query(query)`, `search_text(query, scope, pack_ids[], filters, limit)`, `search_semantic(query, limit)`, `semantic_status()` |
| Çapraz ref. | `related_verses(verse_id)` → `{ semantic[], shared_roots[] }` |
| Paketler | `fetch_manifest(force?)`, `install_pack(id)`, `install_bundle(id)`, `cancel_install(id)`, `uninstall_pack(id)` |
| Kullanıcı | `toggle_bookmark`, `list_bookmarks`, `upsert_note`, `delete_note`, `list_notes(range?)` |
| Koleksiyon | `list_collections`, `get_collection`, `upsert_collection`, `delete_collection`, `add_collection_item`, `update_collection_item`, `move_collection_item`, `remove_collection_item` |
| Dışa aktarma | `export_collection_markdown(id, translation_ids[], labels, style)`, `list_citation_styles()`, `format_citation(ref, style)`, `export_user_data()`, `import_user_data(mode)` |
| AI | `set_api_key`, `delete_api_key`, `has_api_key`, `list_models(provider)`, `chat_send(…, channel)`, `list_chat_sessions`, `get_chat_session`, `delete_chat_session`, `get_usage_summary(period)` |
| Sistem | `get_settings`, `set_setting`, `get_network_log(limit)` |

Event'ler: `pack://progress`, `pack://done`, `pack://error`. Streaming (sohbet, çeviri) için `ipc::Channel` kullanılır.

---

## 13. Frontend

### 13.1 Stack
React + TypeScript + Vite, Tailwind CSS + shadcn/ui, TanStack Router (derin linkler: `/read/2/255`, `/compare/2/153-157`, `/root/رحم`), TanStack Query, TanStack Virtual, Zustand, react-i18next, react-markdown (HTML kapalı), dnd-kit (koleksiyon sıralama).

### 13.2 Ana düzen ve kademeli açılım
Tek bir arayüz, üç bölme:
- **Sol:** Gezinti (sureler, koleksiyonlar, arama geçmişi).
- **Orta:** Ana içerik (okuma / karşılaştırma / arama sonuçları / konkordans).
- **Sağ:** Bağlam paneli, sekmeli: Tefsir · Kelime · İlişkili · Notlar · Sohbet. Seçili ayete veya kelimeye göre güncellenir.

Paneller daraltılabilir ve yeniden boyutlandırılabilir; düzen tercihi `settings`'te saklanır.

**Deneyim profili.** İlk açılışta tek soru sorulur: "Uygulamayı nasıl kullanmak istersiniz? Okumak ve anlamak / Araştırma yapmak". Cevap `settings.experience_profile` (`reader` | `researcher`) olarak saklanır ve **yalnızca varsayılanları** belirler. Hiçbir özellik kilitlenmez; her varsayılan Ayarlar'dan tek tek değiştirilebilir.

| Varsayılan (`settings` anahtarı) | `reader` | `researcher` |
|---|---|---|
| Sağ panel (`layout.right_panel`) | Kapalı | Açık, Kelime sekmesi |
| Meal sayısı (`reader.translations`) | 1 | 2 |
| Ayet eylemleri (`reader.actions`) | Üzerine gelince ikonlar | Sürekli görünür, etiketli |
| Kelime tıklama (`word.click`) | Küçük kart (anlam, kök, geçiş sayısı, "Detaylı analiz") | Sağ panelde tam analiz |
| Kelime vurgusu (`word.underline`) | Yok | Noktalı alt çizgi |
| Teknik terimler (`ui.technical_terms`) | Gizli (ipucunda) | Parantez içinde görünür |
| Arama mod çipleri (`search.mode_chips`) | Gizli (yalnızca Tümü) | Görünür |
| Atıfı kopyala (`reader.citation_action`) | Gizli | Görünür |

**Bileşen ilkesi:** İki profil aynı bileşenleri kullanır, farklı ekranlar yazılmaz. Örneğin küçük kelime kartındaki "Detaylı analiz", sağ paneldeki Kelime sekmesini açar. Yeni bir özellik eklerken "sade varsayılanda nasıl görünür / gizlenir?" sorusu PR şablonunda yer alır.

**Terimler:** i18n dosyalarında teknik terimler sade/teknik çift olarak tutulur (`term.lemma.plain` = "sözlük biçimi", `term.lemma.technical` = "lemma"). Diğer örnekler: konkordans → "geçtiği yerler", anlamsal arama → "anlamına göre ara".

Referans mockup: "Okuma Ekranı Mockup" canvas'ı (sade ve araştırma varsayılanları yan yana).

### 13.3 Ekranlar
1. **Okuma:** Sanallaştırılmış ayet listesi; Fâtiha (besmele 1. ayettir) ve Tevbe dışındaki her surede besmele başlığı; Arapça (morfoloji varsa kelime kelime tıklanabilir) + seçili mealler; her ayette yer imi, not, karşılaştır, koleksiyona ekle eylemleri.
2. **Karşılaştırma:** Ayet/aralık için kullanıcının seçtiği meal ve tefsirler paralel sütunlarda; sütun ekleme/çıkarma/sıralama; satırlar ayet bazında hizalı; tefsir sütunlarında özet/çeviri butonları.
3. **Konkordans:** Kök/lemma başlığı, türevler, sure dağılımı (küçük bar grafik), bağlamlı ayet listesi (anahtar kelime vurgulu).
4. **Arama:** Tümü (gruplu) · Kelime · Kök · Anlam (§7.1).
5. **İlk açılış:** Deneyim profili sorusu, ardından "Önerilen paketler" kartı (§6.3 bundles).
6. **Koleksiyonlar:** Liste + detay; sürükle-bırak sıralama; dışa aktarma.
7. **Sohbet:** Oturumlar, mesajlar, tıklanabilir atıflar, "kullanılan kaynaklar".
8. **Paketler:** Tipe göre gruplu; boyut, lisans, sürüm, ilerleme.
9. **Ayarlar:** Görünüm (tema, fontlar, boyutlar), dil, AI (sağlayıcılar, model rolleri, yetenek bayrakları, harcama), Gizlilik (ağ günlüğü), Hakkında → Kaynaklar.

### 13.4 Tipografi ve yön
- Fontlar gömülüdür, CDN kullanılmaz. Arapça varsayılan font: Amiri Quran (SIL OFL). Alternatif fontların lisansı ayrıca teyit edilir.
- Arapça bloklar `dir="rtl" lang="ar"`; meal blokları paketin `lang`'i.
- Yalnızca mantıksal Tailwind sınıfları kullanılır (`ms-`, `me-`, `ps-`, `pe-`, `start-`, `end-`).
- Arapça ve meal için ayrı font boyutu ve satır aralığı ayarı (Arapça varsayılan ≥ 2.0).

### 13.5 Erişilebilirlik ve klavye
↑/↓ ayet gezintisi, `b` yer imi, `n` not, `t` tefsir, `w` kelime paneli, `r` ilişkili, `c` karşılaştır, `Ctrl/Cmd+K` komut paleti. Görünür odak halkaları.

---

## 14. Güven ve güvenlik

### 14.1 Ağ
- Tüm HTTP istekleri `net.rs` içindeki tek bir client'tan geçer. Bu client'ın sabit bir host izin listesi vardır:
  - `quran.kemalozyon.com` (manifest, fiyatlar, güncelleme meta verisi)
  - `github.com`, `objects.githubusercontent.com` (paketler, güncellemeler)
  - Sağlayıcı API host'ları (Anthropic, OpenAI, Gemini; embedding için Voyage)
  - Kullanıcının OpenAI-uyumlu sağlayıcı için açıkça girdiği `base_url`
- **Bağlantı günlüğü:** Her istek için host, zaman, amaç (`pack`, `update`, `chat`, `translate`, `embed`) ve durum kaydı tutulur; içerik kaydedilmez. Son 500 kayıt bellekte ve `user.db`'de tutulur. Ayarlar → Gizlilik'ten görüntülenir.

### 14.2 API anahtarları
- OS keychain'de saklanır (servis: `com.kemalozyon.siyak`, hesap: `<provider>`). Webview'e asla dönmez; arayüz yalnızca "kayıtlı / kayıtlı değil" bilgisini görür.
- Anahtar ekleme ekranındaki rehber:
  1. "Bu uygulama için ayrı bir anahtar oluşturun."
  2. "Sağlayıcı panelinden bu anahtara/projeye harcama limiti koyun."
  3. "Anahtarınız yalnızca bu bilgisayardan seçtiğiniz sağlayıcıya gönderilir. Şüphelenirseniz sağlayıcı panelinden iptal edin."
  4. Sağlayıcı paneline ve uygulamanın ilgili kaynak koduna (`secrets.rs`, `net.rs`) link.

### 14.3 Uygulama
- Tauri v2 capabilities: webview'e yalnızca uygulama komutları açılır; `fs`, `http`, `shell` plugin izinleri verilmez.
- Sıkı CSP: `default-src 'self'`; harici script yok.
- Paketler sha256 ile, güncellemeler Tauri updater imzasıyla doğrulanır.
- Paket `.sqlite` dosyaları güvenilmeyen girdi olarak ele alınır: yalnızca beklenen tablo/kolonlardan kopyalanır; trigger, view veya SQL çalıştırılmaz.
- Markdown (notlar, sohbet) HTML render kapalı gösterilir.

### 14.4 Doğrulanabilir build
- Release'ler yalnızca GitHub Actions'ta üretilir; elle yüklenen binary yoktur.
- `actions/attest-build-provenance` ile her installer için build provenance yayımlanır. README'de doğrulama komutu bulunur.

---

## 15. Build, release ve güncelleme

### 15.1 Uygulama
- `v*` tag'i push'lanınca `tauri-apps/tauri-action` ile Windows (NSIS/MSI), macOS (universal `.dmg`) ve Linux (`.AppImage`, `.deb`) build'leri alınır; taslak release oluşturulur, `latest.json` ve provenance yüklenir.
- Updater imza anahtarı `tauri signer generate` ile üretilir; özel anahtar uygulama repo'sunun Secrets'ında tutulur (organizasyon secret'ı değil).
- Updater endpoint'leri (sırayla denenir): `https://quran.kemalozyon.com/updates/latest.json`, ardından `https://github.com/siyak-app/siyak/releases/latest/download/latest.json`. Web sitesi, release workflow'u tamamlandığında `latest.json`'u kopyalar.
- OS kod imzalama v1'de opsiyonel (§18). İmzasızsa indirme sayfasında SmartScreen/Gatekeeper talimatı bulunur.
- ONNX Runtime native kütüphanesi her platform için bundle'a dahil edilir.

### 15.2 Veri
- `data-YYYY.MM.N` tag'i push'lanınca pipeline çalışır: paketler üretilir → `validate.py` → sha256 → `manifest.json` + `pricing.json` → release.
- **Doğrulama kapıları:**
  - 114 sure, 6.236 ayet.
  - Her meal paketinde 6.236 satır.
  - Tefsir aralıkları geçerli, paragraflar boş değil.
  - Morfoloji kelime bölümlemesi her ayette Tanzil metniyle eşleşiyor.
  - QAC paketinde: orijinal dosyanın sha256'sı bilinen v0.4 hash'iyle aynı (değiştirilmediğinin kanıtı); türetilen satır sayısı dosyadaki segment sayısıyla aynı; `NOTICE` mevcut.
  - Vektör paketinde 6.236 satır ve doğru boyut.
  - Harekesizleştirme test vektörleri geçiyor.
- Vektör ve model paketleri aynı pipeline koşusunda, aynı model sürümüyle üretilir.

### 15.3 İndirme sayfası
Landing page'in bir parçasıdır (§16.2).

### 15.4 Sürümleme
- Uygulama: SemVer, `v0.x` boyunca kırıcı değişiklikler minor sürümde olabilir.
- `CHANGELOG.md` ve release notları Conventional Commits'ten otomatik üretilir (release-please veya git-cliff). Release notları İngilizce üretilir; Türkçe özet web sitesindeki sürüm notları sayfasına eklenir.
- Veri: `data-YYYY.MM.N` tag'leri; her paketin kendi SemVer sürümü vardır.

---

## 16. Açık kaynak, dokümantasyon ve katkı

### 16.1 Dil politikası

| Alan | Dil |
|---|---|
| Kod: tanımlayıcılar, yorumlar, doc comment'ler, log ve hata mesajı anahtarları | **Yalnızca İngilizce** |
| Commit mesajları, branch adları, PR başlık ve açıklamaları | **Yalnızca İngilizce** |
| Test adları, CI çıktıları, `ARCHITECTURE.md`, `docs/adr/`, `docs/pack-format.md` | **Yalnızca İngilizce** |
| Bu spesifikasyon | İngilizce sürüm kaynak; Türkçe çevirisi `docs/spec/spec.tr.md` |
| `README`, `CONTRIBUTING` | İngilizce asıl, Türkçe karşılığı (`*.tr.md`) |
| Web sitesi: landing page, kullanıcı rehberi, geliştirici rehberi | Türkçe + İngilizce |
| Uygulama arayüzü | Türkçe + İngilizce (i18n dosyaları) |
| Issue ve Discussions | Her iki dil kabul edilir; etiketlerle (`lang:tr`, `lang:en`) işaretlenir |

**Kurallar:**
- Kullanıcıya görünen hiçbir metin koda gömülmez; her metin i18n anahtarıdır (ESLint kuralıyla zorunlu). Rust tarafı kullanıcıya metin değil, hata **kodu** döner (`AppError.code`).
- İslami terimler kodda tutarlı İngilizce transliterasyonla yazılır ve tek bir sözlükte tanımlanır (`docs/glossary.md`: `surah`, `ayah`, `tafsir`, `lemma`, `root`…). Kodda Türkçe kelime (`sure`, `meal`, `ayet`) kullanılmaz.
- Çift dilli dokümanlarda İngilizce sürüm kaynak kabul edilir. Türkçe sayfanın başında "İngilizce sürümle son eşitleme: <commit/tarih>" satırı bulunur; Starlight çevrilmemiş sayfalarda otomatik olarak İngilizce sürümü gösterir.

### 16.2 Web sitesi (landing + dokümantasyon)
- **Teknoloji:** Astro + Starlight, uygulama repo'sunda `website/`. Yayın: GitHub Pages (`.github/workflows/website.yml`, `main`'e her merge'de ve her veri release'inde).
- **Alan adı:** `quran.kemalozyon.com`. DNS'te `quran` için `siyak.github.io`'ya CNAME kaydı; uygulama repo'sunun Pages ayarında özel alan adı ve "Enforce HTTPS".
- Site ayrıca uygulamanın kullandığı sabit adresleri yayınlar: `/data/manifest.json`, `/data/pricing.json`, `/updates/latest.json`.
- **i18n:** Starlight yerleşik çoklu dil: `/tr/…` ve `/en/…`; tarayıcı diline göre yönlendirme.
- **Sayfalar:**
  - **Ana sayfa:** Ne olduğu, ekran görüntüleri (sade ve araştırma varsayılanları), temel ilkeler (offline, ücretsiz, açık kaynak, anahtarsız), indirme butonu.
  - **İndir:** İşletim sistemi tespiti; `releases/latest/download/<sabit-ad>` linkleri (installer adlarında sürüm numarası olmaz); sürüm etiketi GitHub API'den, hata durumunda releases sayfasına yönlendirme; imzasız build ise SmartScreen/Gatekeeper talimatı; build provenance doğrulama komutu.
  - **Kullanıcı rehberi:** İlk adımlar, paketler, arama modları, kök analizi, koleksiyonlar, AI özellikleri (anahtar alma ve harcama limiti rehberiyle).
  - **Kaynaklar ve lisanslar:** `siyak-data/docs/sources.md`'den derlenir.
  - **Geliştirici rehberi:** Kurulum, mimari (`ARCHITECTURE.md`), ADR'ler, paket formatı, katkı rehberi.
  - **Sürüm notları.**
- Web sitesi telemetri ve üçüncü taraf script içermez; GitHub API dışında dış istek yoktur.

### 16.3 Katkıyı kolaylaştırmak

**İlk 15 dakika hedefi:** Yeni bir contributor repo'yu klonladıktan sonra 15 dakika içinde uygulamayı çalıştırabilmelidir.
- Tek komut: `pnpm install && pnpm tauri dev`.
- `dev-data/dev-content.db`: Birkaç sure ve meal içeren örnek veri; milestone'larla büyür (M5'te kısmi morfoloji, M7'de küçük bir vektör seti). Dev build'de `SIYAK_DEV_DATA=dev` ile kullanılır; varsayılan olarak depodaki `core.db` kullanılır, böylece yayımlanan paketler kurulabilir. Her iki durumda da contributor'ın Python pipeline'ını çalıştırması gerekmez. Boyut hedefi < 5 MB.
- `.devcontainer/`: Rust, Node, pnpm ve Linux webview bağımlılıkları hazır.
- `CONTRIBUTING.md`'de platforma özel önkoşullar (Windows: WebView2/MSVC; macOS: Xcode CLT; Linux: WebKitGTK paketleri).

**Katkı yolları** (kod yazmayanlar dahil):

| Yol | Nerede | Rehber |
|---|---|---|
| Kod | Uygulama repo'su | `CONTRIBUTING.md`, `good first issue` etiketleri |
| Yeni meal/tefsir kaynağı, lisans araştırması | `siyak-data` | `docs/adding-a-source.md`, "Yeni kaynak önerisi" issue şablonu (lisans alanı zorunlu) |
| Arayüz çevirisi (yeni dil) | `src/i18n/` | `docs/translating.md` |
| Morfoloji etiketlerinin Türkçe karşılıkları ve terim çevirilerinin incelenmesi; QAC etiket hatası bildirimi (errata) | Issue/Discussions | "Terim düzeltmesi" ve "QAC etiket hatası" issue şablonları |
| Dokümantasyon | `website/` | Her sayfada "Bu sayfayı düzenle" linki |

**Proje yönetimi:**
- GitHub Issues + bir GitHub Project panosu (milestone'lar §17 ile eşleşir).
- Etiketler: `good first issue`, `help wanted`, `area:*` (`reader`, `search`, `morphology`, `packs`, `llm`, `website`, `data`), `lang:*`, `needs-license-check`.
- Fikir ve sorular için GitHub Discussions; issue'lar uygulanabilir işler için.
- Mimari değişiklikler önce ADR önerisi olarak (PR ile `docs/adr/NNNN-*.md`) tartışılır.

### 16.4 Kod kalitesi ve CI
Kurallar dokümanda değil, CI'da zorunlu tutulur. Her PR'da çalışan `ci.yml`:

| Alan | Kontroller |
|---|---|
| Rust | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, uyarısız `cargo doc` |
| Frontend | ESLint (i18n zorunluluğu dahil), Prettier, `tsc --noEmit`, Vitest |
| Bağlar | `tauri-specta` bindings'in güncel olduğu kontrolü (fark varsa hata) |
| Commit | Conventional Commits (`feat:`, `fix:`, `docs:`…) PR başlığında kontrol edilir; squash merge |
| Build | Üç platformda derleme (release imzası olmadan) |
| Web sitesi | Astro build + kırık link kontrolü |
| `siyak-data` | `ruff`, `pytest`, `validate.py` örnek veride |

**Kod okunabilirliği:**
- Her Rust modülünün başında `//!` ile ne yaptığı ve neden var olduğu; her public fonksiyonda `///` doc comment. `cargo doc` CI'da uyarısız derlenir.
- TypeScript'te public hook ve bileşenlerde TSDoc.
- Karmaşık mantıkta (harekesizleştirme, IDF skoru, paket kurulum transaction'ı, RAG bağlam bütçesi) "neden böyle" yorumu ve ilgili ADR/spec bölümüne referans. "Ne yaptığı" kodun kendisinden okunabilir olmalıdır.
- Modül sınırları `ARCHITECTURE.md`'deki "X'i değiştirmek istiyorsan Y'ye bak" haritasıyla eşleşir.

**PR şablonu** şunları sorar: ne değişti ve neden; ilgili issue; test nasıl yapıldı; ekran görüntüsü (arayüz değişikliğinde); "Sade varsayılanda nasıl görünür?" (§13.2); yeni kullanıcı metinleri iki dile eklendi mi; dokümantasyon güncellendi mi.

**Tamamlanma tanımı (her milestone ve PR için):** Kod + testler + ilgili dokümantasyon (İngilizce) + kullanıcıya dönükse iki dilli rehber güncellemesi.

### 16.5 Repo dosyaları

| Dosya | İçerik |
|---|---|
| `README.md` / `README.tr.md` | Ne olduğu, ekran görüntüsü, indirme linki, 5 adımda geliştirme kurulumu, katkı yollarına bağlantı |
| `CONTRIBUTING.md` / `.tr.md` | Kurulum, branch/commit kuralları, PR süreci, test çalıştırma, katkı yolları |
| `ARCHITECTURE.md` | Katmanlar, veri akışı, modül haritası, temel değişmezler (ör. "webview ağa çıkmaz") |
| `docs/adr/` | Karar kayıtları. İlk set: 0001 Tauri v2 + React; 0002 iki veritabanı; 0003 tefsirin embed edilmemesi; 0004 Arapça tefsir çevirisinin isteğe bağlı olması; 0005 kademeli açılım; 0006 `MIT OR Apache-2.0`; 0007 QAC morfolojisi, değiştirilmeden; 0008 Quranpedia'dan klasik tefsirler, v1'de çağdaş eser yok; 0009 yalnızca İngilizce kod, iki dilli doküman; 0010 geliştirme araçları için Rust `xtask` (dev DB, bindings) |
| `docs/spec/spec.md` / `spec.tr.md` | Bu spesifikasyon ve Türkçe çevirisi |
| `docs/glossary.md` | Terim sözlüğü (İngilizce kod adı · Türkçe · Arapça) |
| `CODE_OF_CONDUCT.md` (org `.github`) | Contributor Covenant |
| `SECURITY.md` (org `.github`) | Güvenlik açığı bildirimi (GitHub private vulnerability reporting); API anahtarı ve paket bütünlüğü kapsamda |
| `CHANGELOG.md` | Otomatik üretilir |

### 16.6 GitHub organizasyonu ayarları
- Plan: ücretsiz. Owner: proje sahibi (tek owner). Yeni bakımcılara repo bazında `maintain`/`write` yetkisi verilir.
- Tüm üyeler için 2FA zorunlu.
- `main` korumalı: PR zorunlu, CI geçmeden merge yok, en az bir onay (tek bakımcı döneminde kendi PR'ları için muafiyet), squash merge.
- Release ve updater imza anahtarları organizasyon secret'ı değil, ilgili repo'nun secret'ı; yalnızca release workflow'u erişir.
- Private vulnerability reporting açık.
- Organizasyon profil sayfası (`.github/profile/README.md`): proje tanıtımı (TR + EN), web sitesi, indirme ve katkı linkleri.

---

## 17. Kilometre taşları

| # | Kapsam | Bitti sayılması için |
|---|---|---|
| M0 | İskelet + açık kaynak altyapısı | Tauri + React + Tailwind + shadcn + i18n + specta; üç bölmeli düzen; `ci.yml` tüm kontrollerle; `dev-content.db`; README, CONTRIBUTING (iki dilli), ARCHITECTURE, glossary, ilk ADR'ler, issue/PR şablonları, devcontainer |
| M1 | Veri pipeline | `core.db`, meal paketleri, normalize test vektörleri, doğrulama kapıları, manifest, `siyak-data` dokümanları (`pack-format`, `adding-a-source`, `sources`) |
| M2 | Okuma | Sure listesi, sanallaştırılmış ayetler, meal seçimi, fontlar, RTL, deneyim profili ve varsayılanları |
| M3 | Paketler | Manifest, bundles, indirme/doğrulama/kurulum/kaldırma, ilerleme arayüzü, Kaynaklar ekranı |
| M4 | Tefsir + karşılaştırma + arama | Tefsir paneli (paragraflı), karşılaştırma ekranı, tek arama kutusu (Tümü + Kelime), FTS5 |
| M5 | Morfoloji | Morfoloji paketi, tıklanabilir kelimeler, kelime kartı ve paneli, Kök modu, konkordans |
| M6 | Koleksiyonlar | Yer imi, notlar, koleksiyonlar, Markdown/atıf/JSON dışa aktarma |
| M7 | Anlamsal katman | Model seçimi, lokal (ONNX) ve API embedder'lar, vektör paketi, Anlam modu, çapraz referans |
| M8 | AI | Sağlayıcı katmanı, keychain, ağ izin listesi + günlük, tefsir özet/çeviri + maliyet, RAG sohbet |
| M9 | Release + web sitesi | Updater, provenance, landing page, kullanıcı rehberi (iki dil), v0.1.0 |

Sıralama mantığı: Açık kaynak altyapısı en başta kurulur (M0), çünkü sonradan eklemek pahalıdır ve erken gelen contributor'ları kaybettirir. AI'sız değer önce gelir (M2–M6); uygulama M6 sonunda anahtarsız haliyle yayımlanabilir bir araştırma aracıdır.

---

## 18. Açık sorular

| # | Soru | Etkisi |
|---|---|---|
| 1 | Embedding modeli: varsayılan lokal model ve hangi API modellerine paket çıkarılacağı (ADR 0012) | Paket boyutu, arama ve çapraz referans kalitesi |
| 2 | Embed edilecek metin: harekesiz Arapça, meal ya da ikisinin birleşimi. Türkçe sorgular için Arapça-yalnız embedding zayıf kalabilir; değerlendirme setiyle denenmeli | Vektör paketi tasarımı |
| 3 | Anlamsal arama ve çapraz referans için değerlendirme seti (sorgu/ayet → beklenen ayetler): kim hazırlayacak, kaç örnek? | M7 kabul kriteri |
| 4 | **MASAQ değerlendirmesi** (§5.3.1): lemma bilgisi var mı, Tanzil ile hizalama, QAC ile örneklem karşılaştırması; son sürümün lisans ifadesinin netleştirilmesi | İkinci morfoloji paketi, i'rab katmanı |
| 5 | **Kurtubî izni:** Tarteel'den QUL #23'ün kökeni ve yeniden dağıtım izni (§6.6). Talep gönderildi mi, yanıt geldi mi? | `ar.qurtubi` paketinin yayını |
| 6 | **Türkçe meal kaynağı:** `core.db` için Elmalılı Hamdi Yazır'ın orijinal (sadeleştirilmemiş) mealinin ve tefsiri *Hak Dini Kur'an Dili*'nin temiz, dijital ve lisansı net bir kaynağı | Gömülü Türkçe meal (§6.2), Türkçe tefsir katmanı |
| 7 | v1 atıf stilleri: İSNAD ve APA geçici olarak uygulandı, stil kılavuzlarına göre gözden geçirilecek. Chicago ve her paket için gerekli `citation` alanları hâlâ açık | §11.2 |
| 8 | Küratörlü bir çapraz referans veri seti var mı, lisansı uygun mu? | §7.6 üçüncü kaynak |
| 9 | OS kod imzalama v1'de yapılacak mı? (Apple Developer Program yıllık ücretli; Windows sertifikası ayrıca ücretli) | Kurulum deneyimi |
| 10 | KFGQPC Uthmanic Script HAFS gibi alternatif Arapça fontların gömme lisansı | Font seçenekleri |
