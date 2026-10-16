---
name: list
description: Fikir havuzundaki fikirleri listeler. Kullanıcı kayıtlı fikirlerini görmek istediğinde kullan.
argument-hint: "[durum: new | reviewed | done]"
---

Proje kökündeki `ideas.json` dosyasını oku ve fikirleri bir Markdown tablosu olarak göster.

- Sütunlar: `#` (id), Başlık, Durum, Tarih.
- `$ARGUMENTS` bir durum içeriyorsa (`new`, `reviewed`, `done`) yalnızca o durumdaki fikirleri göster.
- En yeni fikir en üstte olsun.
- Tablonun altına toplam fikir sayısını yaz.

Dosya yoksa veya boşsa, fikir havuzunun boş olduğunu söyle ve `/ideas:add` ile fikir eklenebileceğini hatırlat.
