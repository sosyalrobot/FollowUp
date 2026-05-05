# FollowUp

X (Twitter) üzerinde sorduğun soruların kaybolmasını engelleyen minimal takip aracı.

Birine soru sorarsın.
Cevap gelmez.
Tweet akışta kaybolur.

FollowUp bunu takip eder.

---

## Nasıl Çalışır?

Tweet atarken veya reply yaparken küçük bir tag eklersin:

```txt
@unity Yeni inovatif bir şey yok mu? #fu30
````

FollowUp bu tweet’i yakalar ve takip etmeye başlar.

---

## Tag Sistemi

```txt
#fu      → varsayılan (7 gün)
#fu7     → 7 gün takip
#fu14    → 14 gün takip
#fu30    → 30 gün takip
```

---

## Ne Takip Edilir?

* Mention attığın kullanıcı (örn: @unity)
* Tweet’e gelen cevaplar
* Thread içindeki yanıtlar

---

## Mantık

```txt
Tweet atıldı → #fu30
↓
30 gün boyunca izlenir
↓
Cevap geldi mi?
    evet → ANSWERED
    hayır → EXPIRED
```

---

## Cevap Tespiti

Bir tweet şu durumlarda "cevaplandı" sayılır:

* mention edilen kullanıcı cevap verirse
* thread’e resmi/ilişkili hesap cevap verirse
* kullanıcı manuel kapatırsa

---

## Süre Dolunca

Cevap yoksa sistem:

* durumu EXPIRED yapar
* sana hatırlatır
* isterse follow-up metni önerir

Örnek:

```txt
30 gün geçti, hâlâ cevap yok.
@unity bu konu hakkında bir güncelleme var mı?
```

---

## Kullanım Şekilleri

### 1. Sadece tag ile (en hızlı)

```txt
#fu30
```

### 2. Mevcut hesabınla

Sistem, senin hesabını (örn: @SosyalRobot) tarar ve tag’li tweetleri bulur.

---

## Dashboard

```txt
WAITING   @unity     12 gün
ANSWERED  @support   2 gün
EXPIRED   @company   30 gün
```

---

## Modlar

* Passive → sadece takip eder (önerilen MVP)
* Active → otomatik follow-up reply atar

---

## MVP Özellikleri

* Tag ile tweet yakalama
* Tweet ID kaydı
* Süre takibi
* Reply kontrolü
* Status: WAITING / ANSWERED / EXPIRED
* Basit dashboard

---

## Teknik

Önerilen stack:

* Node.js
* SQLite
* Cron job
* X API veya scraping
* Minimal web panel

---

## Veri Modeli

```txt
TrackedTweet
- id
- tweet_id
- author
- target
- tag
- status
- created_at
- expire_at
- answered_at
```

---

## Roadmap

* Otomatik reply
* Telegram / email bildirim
* "kim cevap vermiyor?" istatistikleri
* public sayfalar
* Chrome extension
* AI ile follow-up tonu

---

## Lisans

MIT
