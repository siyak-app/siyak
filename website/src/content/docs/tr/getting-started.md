---
title: Geliştirmeye başlarken
description: Siyak'ı kaynak koddan çalıştırın.
---

İngilizce sürümle son eşitleme: 2026-10-21

Siyak henüz yayımlanmadı; denemenin tek yolu kaynak koddan çalıştırmak. Hedef, hazır bir
bilgisayarda bunun 15 dakikadan kısa sürmesi.

Güncel ve eksiksiz talimatlar — Windows, macOS ve Linux için gereksinimler, kurulum, kontroller ve
kurallar — deponun
[katkı rehberinde](https://github.com/siyak-app/siyak/blob/main/CONTRIBUTING.tr.md).

Kısaca, gereksinimler kurulduktan sonra:

```sh
git clone https://github.com/siyak-app/siyak.git
cd siyak
pnpm install
pnpm tauri dev
```

Uygulama Kuran'ın tam metni ve Pickthall mealiyle açılır; veri hattına (pipeline) ihtiyacınız
olmaz.
