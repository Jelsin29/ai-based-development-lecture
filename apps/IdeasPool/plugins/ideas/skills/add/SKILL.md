---
name: add
description: Fikir havuzuna yeni bir fikir ekler
argument-hint: <fikir başlığı ve isteğe bağlı açıklama>
disable-model-invocation: true
---

Kullanıcının fikri: $ARGUMENTS

Bu fikri proje kökündeki `ideas.json` dosyasına ekle:

1. `ideas.json` dosyasını oku. Dosya yoksa içeriği `[]` olan yeni bir dosya olarak başla.
2. Dosya bir JSON dizisidir. Diziye şu alanlarla yeni bir nesne ekle:
   - `id`: mevcut en büyük `id` + 1 (dizi boşsa 1)
   - `title`: fikrin kısa başlığı (en fazla 80 karakter)
   - `description`: kullanıcı ek açıklama verdiyse o açıklama, yoksa boş metin
   - `createdAt`: bugünün tarihi, `YYYY-MM-DD` biçiminde
   - `status`: `"new"`
3. Dosyayı 2 boşluk girintiyle geri yaz. Mevcut kayıtları değiştirme.
4. Kullanıcıya eklenen fikrin `id` ve başlığını tek satırda bildir.

Fikir metni boşsa hiçbir şey yazma; kullanıcıdan fikri iste.
