# Siyak

> İngilizce sürümle son eşitleme: 2026-10-21

**Ayeti bağlamıyla okuyun.** · [English](README.md)

Siyak, Kur'an okumak ve üzerine araştırma yapmak için ücretsiz ve açık kaynaklı bir masaüstü
uygulamasıdır. Arapça metin ve mealler, tefsir, kelime ve kök analizi, arama ve notlar tek bir
arayüzde bir araya gelir: okuyucu için sade kalır, araştırmacı için derinleşir. Her şey kendi
bilgisayarınızda çalışır.

> **Durum: pre-alpha.** Henüz yayımlanmış bir sürüm yok. İskelet, küçük bir geliştirme veri setiyle
> çalışıyor; [kilometre taşlarına](docs/spec/spec.md#17-milestones) bakabilirsiniz.

## İlkeler

- **Önce çevrimdışı:** Hesap da sunucu da yok; tüm içerik yerel veritabanlarında durur.
- **Ücretsiz ve açık kaynak:** Kod `MIT OR Apache-2.0` lisanslı; her veri kaynağı kendi lisansını
  korur.
- **API anahtarı gerekmez:** Okuma, karşılaştırma, kök analizi, arama ve notlar anahtarsız çalışır.
  Yapay zekâ özellikleri, kendi anahtarınızla ya da yerel bir modelle kullanılan isteğe bağlı bir
  katmandır.
- **Kaynağa sadakat:** Orijinal metin her zaman bir tık uzaktadır; makine çıktısı açıkça
  etiketlenir.

## Kaynak koddan çalıştırma

İşletim sisteminize uygun [gereksinimler](CONTRIBUTING.tr.md#gereksinimler) kuruluyken:

1. `git clone https://github.com/siyak-app/siyak.git`
2. `cd siyak`
3. `pnpm install`
4. `pnpm tauri dev`

Uygulama, geliştirme veri setindeki Fâtiha ve birkaç sureyle açılır.

## Daha fazlası

- [Katkı rehberi](CONTRIBUTING.tr.md): kurulum, kontroller, kurallar ve kod yazmadan da katkı
  yolları
- [ARCHITECTURE](ARCHITECTURE.md) (İngilizce): uygulamanın yapısı
- [Şartname](docs/spec/spec.md) ve [karar kayıtları](docs/adr/README.md) (İngilizce)
- [Terimler sözlüğü](docs/glossary.md)
- Web sitesi: <https://quran.kemalozyon.com> (yakında)

## Lisans

Kod: tercihinize göre [MIT](LICENSE-MIT) veya [Apache-2.0](LICENSE-APACHE) lisansı.

Verilerin kendi lisansları vardır. Geliştirme veri seti [Tanzil Projesi](https://tanzil.net)'nin
Kur'an metnini (CC BY 3.0, değiştirilmeden) ve kamu malı Pickthall mealini kullanır; ayrıntılar
[dev-data/README.md](dev-data/README.md) dosyasında. Arapça yazı tipi
[Amiri Quran](https://github.com/aliftype/amiri)'dır (SIL Open Font License).
