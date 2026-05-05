# FollowUp

FollowUp, X/Twitter üzerinde `#fu` tag'i ile sorduğun soruların kaybolmasını engelleyen minimal takip aracıdır.

Hedef: düşük kaynak tüketen, deploy etmesi kolay, klasik SQL kullanmayan, Velo-Lite destekli tek binary Rust uygulaması.

## Özellikler

- `#fu`, `#fu7`, `#fu14`, `#fu30` tag parse etme
- Mention edilen hedef hesabı takip etme
- WAITING / ANSWERED / EXPIRED / CLOSED durumları
- Velo-Lite key-value dosya veritabanı
- Minimal HTTP dashboard
- JSON API
- CLI komutları
- Framework ve Node.js yok

## Teknik Stack

- Rust 2021
- Rust standard library HTTP server
- Velo-Lite dynamic library (`../velo-lite/target/release/libvelo_lite.dylib` veya `.so`)
- Dış Rust crate yok
- Klasik SQL yok

## Gereksinimler

Parent folder içinde Velo-Lite bulunmalı:

```txt
../velo-lite
```

Velo-Lite native library hazır değilse:

```bash
cd ../velo-lite
cargo build --release
```

## Çalıştırma

```bash
cargo run -- serve
```

Varsayılan adres:

```txt
http://127.0.0.1:8000
```

Ortam değişkenleri:

```bash
FOLLOWUP_DB=./data/followup.velo
FOLLOWUP_HOST=127.0.0.1
FOLLOWUP_PORT=8000
VELO_LITE_LIB_DIR=../velo-lite/target/release
FOLLOWUP_API_TOKEN=change-this-before-public-deploy
```

`FOLLOWUP_API_TOKEN` set edilirse API endpointleri `Authorization: Bearer ...` ister. Public server'da bu token'ı mutlaka set et ve uygulamayı TLS terminasyonu yapan reverse proxy arkasında çalıştır.

## CLI

Tweet takip et:

```bash
cargo run -- add --tweet-id 1001 --author @SosyalRobot --text "@unity yeni bir sey yok mu? #fu30" --created-at 2026-05-05T10:00:00Z
```

Reply kaydet:

```bash
cargo run -- reply --tweet-id 2001 --author @unity --text "Guncelleme yakinda." --in-reply-to 1001 --created-at 2026-05-06T10:00:00Z
```

Listele:

```bash
cargo run -- list
```

Süresi dolanları işaretle:

```bash
cargo run -- check
```

Manuel kapat:

```bash
cargo run -- close 1001
```

## API

Health:

```http
GET /health
```

Tweet ekle:

```http
POST /api/ingest
Content-Type: application/json

{
  "tweet_id": "1001",
  "author": "@SosyalRobot",
  "text": "@unity yeni bir sey yok mu? #fu30",
  "created_at": "2026-05-05T10:00:00Z"
}
```

Reply ekle:

```http
POST /api/ingest
Content-Type: application/json

{
  "tweet_id": "2001",
  "author": "@unity",
  "text": "Guncelleme yakinda.",
  "created_at": "2026-05-06T10:00:00Z",
  "in_reply_to_tweet_id": "1001"
}
```

Listele:

```http
GET /api/tweets
GET /api/tweets?status=WAITING
Authorization: Bearer your-token
```

Expire check:

```http
POST /api/check
Authorization: Bearer your-token
```

Manuel kapat:

```http
POST /api/tweets/1001/close
Authorization: Bearer your-token
```

## Veri Modeli

Velo-Lite key-value olarak kullanılır:

```txt
followup:meta:next_id
followup:index:tracked_tweets
followup:tweet:{tweet_id}
```

Her tweet JSON document olarak saklanır.

```txt
id
tweet_id
author
target
tag
status
text
created_at
expire_at
answered_at
answer_tweet_id
notes
```

## Test

```bash
cargo test
```

## Deploy Notu

Server'a şu dosyalar gerekir:

- build edilmiş `followup` binary
- Velo-Lite shared library: `libvelo_lite.so` veya `libvelo_lite.dylib`
- writable `FOLLOWUP_DB` path

Linux deploy için Velo-Lite'ı server hedefinde build etmek en temiz seçenektir.

Public deploy için önerilen ayarlar:

- `FOLLOWUP_HOST=127.0.0.1`
- Reverse proxy: nginx, Caddy veya benzeri
- TLS açık
- `FOLLOWUP_API_TOKEN` güçlü ve gizli bir değer
- `data/*.velo` dosyalarını repo dışında veya ignored path altında tut

## Lisans

MIT
