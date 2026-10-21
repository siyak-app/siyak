---
title: Geliştirmeye başlarken
description: Siyak'ı kaynak koddan çalıştırın.
---

İngilizce sürümle son eşitleme: 2026-10-21

Siyak henüz yayımlanmadı; denemenin tek yolu kaynak koddan çalıştırmak. Hedef, hazır bir
bilgisayarda bunun 15 dakikadan kısa sürmesi.

Güncel ve eksiksiz talimatlar — Windows, macOS ve Linux için gereksinimler, kurulum, kontroller ve
kurallar — deponun
[katkı rehberinde](https://github.com/siyak/siyak/blob/main/CONTRIBUTING.tr.md).

Kısaca, gereksinimler kurulduktan sonra:

```sh
git clone https://github.com/siyak/siyak.git
cd siyak
pnpm install
pnpm tauri dev
```

Uygulama küçük bir geliştirme veri setiyle açılır (Fâtiha ve son üç sure, Pickthall meali); veri
hattına (pipeline) ihtiyacınız olmaz.
