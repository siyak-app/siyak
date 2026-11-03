---
title: Getting started (development)
description: Run Siyak from source.
---

Siyak is not released yet, so the only way to try it is to run it from source. The goal is that
this takes less than 15 minutes on a prepared machine.

The full, maintained instructions — prerequisites for Windows, macOS and Linux, setup, checks and
conventions — are in the repository's
[CONTRIBUTING guide](https://github.com/siyak-app/siyak/blob/main/CONTRIBUTING.md).

In short, once the prerequisites are installed:

```sh
git clone https://github.com/siyak-app/siyak.git
cd siyak
pnpm install
pnpm tauri dev
```

The app opens with a small development dataset (Al-Fatiha and the last three surahs with the
Pickthall translation), so you do not need the data pipeline.
