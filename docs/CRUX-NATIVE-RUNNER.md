# Crux-MIR Native x86_64 Runner

Tarih: 2026-11-03  
Durum: **hazırlık**; native runner olmadan PASS yok.

## Neden?
- `ghcr.io/galoisinc/crux-mir:0.12` yalnız **amd64**.
- Apple Silicon’da Rosetta altında rustc **SIGSEGV**.
- Log: `reports/crux-simple.log`.

## Gereken
- Ubuntu 22.04/24.04 **x86_64** runner
- Docker
- `mir-json` / `crux-build` akışı (image içinde mevcut değilse ayrıca kurulum)

## CI
`.github/workflows/crux-mir.yml` yalnız `ubuntu-latest` (native x86_64) üzerinde çalışır.
Rosetta/arm64 runner’da çalıştırılmaz.

## Yerel native x86 komut
```bash
docker run --rm -v "$PWD/formal/crux:/work:rw" -w /work \
  ghcr.io/galoisinc/crux-mir:0.12 crux-mir /work/simple.rs
```

## Sınır
- Bu workflow bu ortamda **koşulmadı**.
- Crux PASS iddiası native x86 logu olmadan yapılmaz.
