# Claude Code Plugin Yazma Rehberi

Bu rehberde sıfırdan bir Claude Code plugin'i yazıyoruz: **`ideas`**, bir fikir havuzu. Her bölüm bir kavram öğretir ve plugin'e bir parça ekler. Sonunda elinizde şunları yapan çalışan bir plugin olur:

| Bileşen | Ne yapar | Nasıl kullanılır |
| :- | :- | :- |
| Skill `add` | Fikri `ideas.json`'a kaydeder | `/ideas:add Fikrim` |
| Skill `list` | Fikirleri tablo olarak gösterir | `/ideas:list` |
| Agent `idea-reviewer` | Bir fikri değerlendirir | "idea-reviewer ile son fikri değerlendir" |
| Hook `SessionStart` | Oturum başında fikir sayısını Claude'a bildirir | Otomatik |

Bitmiş plugin bu repoda `plugins/ideas/` altında. Rehberi okurken dosyaları kendiniz sıfırdan yazabilir veya hazır olanları inceleyebilirsiniz.

**Gereksinimler:** Claude Code (bu rehber 2.1.301 ile denendi) ve Node.js (hook script'i için). Komutlar repo kökünden (`IdeasPool/`) çalıştırılır.

---

## 0. Plugin nedir, ne zaman gerekir?

Claude Code'u genişletmenin dört temel yolu var:

- **Skill**: Claude'a talimat veren bir `SKILL.md` dosyası. Hem `/komut` olarak çalıştırılabilir hem de Claude ilgili olduğunda kendisi kullanabilir.
- **Agent (subagent)**: Claude'un iş devredebileceği, kendi talimatları ve araç kısıtları olan yardımcı.
- **Hook**: Claude Code'un yaşam döngüsündeki belirli anlarda (oturum başı, dosya düzenlemesi sonrası vb.) çalışan komut.
- **MCP sunucusu**: Claude'a yeni araçlar (tool) veren bir süreç.

Bunların hepsi **plugin olmadan da** çalışır; örneğin `~/.claude/skills/` altına koyduğunuz bir skill her projede kullanılabilir. **Plugin**, bunları tek bir birim olarak paketlemenin yoludur. Bir kerede kurulur, sürümlenir, takım arkadaşlarıyla paylaşılır ve bir marketplace üzerinden dağıtılabilir.

Plugin'e geçince değişen iki şey var:

1. **Konum**: dosyalar plugin'in kendi klasörüne (plugin kökü) taşınır.
2. **İsimlendirme**: plugin'deki her skill ve agent, plugin adını **önek** olarak alır. `add` skill'i `/add` değil `/ideas:add` olur. Böylece iki farklı plugin'in `add` skill'i çakışmaz.

> **Bağlam maliyeti:** Etkin bir plugin her oturumun parçasıdır. Claude'un kendi başına çağırabildiği her skill ve agent'ın adı ve açıklaması her turda Claude'un bağlamında bulunur. Açıklamaları kısa tutmak bu yüzden önemlidir.

---

## 1. En küçük plugin: manifest

Bir plugin, içinde `.claude-plugin/plugin.json` dosyası (manifest) bulunan bir klasördür.

```bash
mkdir -p plugins/ideas/.claude-plugin
```

**`plugins/ideas/.claude-plugin/plugin.json`**

```json
{
  "name": "ideas",
  "description": "Fikir havuzu: fikirleri kaydet, listele ve değerlendir",
  "version": "0.1.0",
  "author": {
    "name": "Burak Selim Şenyurt"
  }
}
```

| Alan | Açıklama |
| :- | :- |
| `name` | **Zorunlu.** Plugin'in kimliği ve tüm skill/agent'ların öneki. Boşluk içermez. |
| `description` | `/plugin` ekranında görünen açıklama. |
| `version` | İsteğe bağlı. Belirtirseniz kullanıcılar siz değiştirene kadar bu sürümde kalır. |
| `author` | `name` zorunlu; `email` ve `url` isteğe bağlı. |

Doğrulayın:

```bash
claude plugin validate ./plugins/ideas
```

Çıktının sonunda `✔ Validation passed` görmelisiniz. Hata varsa, sonuç satırının üstündeki her satır düzeltilecek alanı söyler.

> ⚠️ **En sık yapılan hata:** `.claude-plugin/` klasörüne **yalnızca** `plugin.json` girer. `skills/`, `agents/`, `hooks/` gibi klasörler plugin köküne, yani `.claude-plugin/`'in **yanına** konur. İçine konan bileşenler sessizce yüklenmez.

**Ne öğrendik:** Plugin = klasör + manifest. `name` alanı her şeyin önekidir.

---

## 2. İlk skill: `/ideas:add`

Her skill, `skills/` altında kendi klasörüne sahiptir ve içinde bir `SKILL.md` bulunur. Klasör yapısı skill'in adını belirler.

```bash
mkdir -p plugins/ideas/skills/add
```

**`plugins/ideas/skills/add/SKILL.md`**

```markdown
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
```

Dosyanın iki parçası var:

**Frontmatter** (`---` arası, YAML):

| Alan | Anlamı |
| :- | :- |
| `name` | Skill adı. Komut `/<plugin>:<name>` olur, burada `/ideas:add`. |
| `description` | Skill'in ne yaptığı. Claude'un skill'i ne zaman kullanacağına bu metne bakarak karar verir. |
| `argument-hint` | Komutu yazarken otomatik tamamlamada görünen ipucu. |
| `disable-model-invocation: true` | Claude bu skill'i **kendi başına çalıştırmaz**; yalnızca siz `/ideas:add` yazınca çalışır. Dosyaya yazan bir işlem olduğu için bunu istiyoruz. |

**Gövde**: Skill çalıştığında Claude'a verilen talimat. `$ARGUMENTS`, komuttan sonra yazdığınız metinle değiştirilir: `/ideas:add Kahve takip uygulaması` → `Kullanıcının fikri: Kahve takip uygulaması`.

Burada önemli bir fikir var: **skill kod değil, talimattır.** JSON okuma/yazma işini Claude kendi araçlarıyla (Read, Write) yapar. Siz yalnızca *ne* yapılacağını ve kuralları tarif edersiniz.

### Çalıştırın

```bash
claude plugin validate ./plugins/ideas
claude --plugin-dir ./plugins/ideas
```

`--plugin-dir`, plugin'i **kurmadan**, yalnızca bu oturum için yükler. Ayarlarınıza hiçbir şey yazılmaz. Oturumda şunu deneyin:

```text
/ideas:add Kahve takip uygulaması - günde kaç fincan içtiğimi kaydetsin
```

Claude dosyaya yazmak için izin isteyecek; onaylayın. Repo kökünde bir `ideas.json` oluşur:

```json
[
  {
    "id": 1,
    "title": "Kahve takip uygulaması",
    "description": "günde kaç fincan içtiğimi kaydetsin",
    "createdAt": "2026-10-16",
    "status": "new"
  }
]
```

**Ne öğrendik:** Skill = frontmatter + talimat. Plugin skill'leri `/<plugin>:<skill>` adıyla çağrılır. `$ARGUMENTS` komut argümanlarını taşır.

---

## 3. İkinci skill: `/ideas:list` ve geliştirme döngüsü

Claude Code oturumunu **kapatmadan** yeni bir skill ekleyelim.

**`plugins/ideas/skills/list/SKILL.md`**

```markdown
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
```

Açık oturumda:

```text
/reload-plugins
/ideas:list
```

`/reload-plugins`, diskteki değişiklikleri yeniden yükler ve kaç bileşen yüklendiğini `Reloaded:` satırında gösterir. **Geliştirme döngünüz** bundan ibaret: *dosyayı düzenle → `/reload-plugins` → dene.*

### Fark: `disable-model-invocation` yok

`add`'in aksine `list` bu satırı içermiyor. Yani Claude bu skill'i **kendi başına** da kullanabilir. Komut yazmadan, sadece şunu söylemeyi deneyin:

```text
Hangi fikirlerim vardı?
```

Claude, `description` alanındaki *"Kullanıcı kayıtlı fikirlerini görmek istediğinde kullan"* ifadesini eşleştirip `ideas:list` skill'ini çağırır. İyi bir `description` hem **ne yaptığını** hem de **ne zaman kullanılacağını** söyler.

| | Kullanıcı `/komut` ile çağırabilir | Claude kendisi çağırabilir | Açıklaması her turda bağlamda |
| :- | :- | :- | :- |
| Varsayılan | ✔ | ✔ | ✔ |
| `disable-model-invocation: true` | ✔ | ✘ | ✘ |

Kural olarak, yan etkisi olan (dosya yazan, deploy eden, mesaj gönderen) skill'lerde model çağrısını kapatın, salt okuyan skill'lerde açık bırakın.

**Ne öğrendik:** `/reload-plugins` ile oturumu kapatmadan geliştirme yapılır. `description`, Claude'un skill'i ne zaman seçeceğini belirler.

---

## 4. Agent: `idea-reviewer`

Skill, Claude'a *ana konuşmanın içinde* talimat verir. **Agent (subagent)** ise ayrı bir yardımcıdır: kendi bağlam penceresinde çalışır, kendi sistem talimatı ve araç kısıtları vardır, işini bitirince ana konuşmaya yalnızca sonucu döndürür. Uzun veya odaklı işleri ana konuşmayı kalabalıklaştırmadan yaptırmak için kullanılır.

Agent'lar `agents/` altında **tek bir Markdown dosyasıdır** (skill'ler gibi klasör değil).

**`plugins/ideas/agents/idea-reviewer.md`**

```markdown
---
name: idea-reviewer
description: Fikir havuzundaki bir fikri fizibilite, risk ve ilk adım açısından değerlendirir. Kullanıcı bir fikrin değerlendirilmesini veya yorumlanmasını istediğinde kullan.
tools: Read, Edit
model: sonnet
---

Sen deneyimli bir ürün ve yazılım danışmanısın. Görevin, proje kökündeki `ideas.json` dosyasındaki bir fikri değerlendirmek.

1. `ideas.json` dosyasını oku. Sana bir `id` verildiyse o fikri, verilmediyse en yüksek `id`'li fikri seç.
2. Fikri şu başlıklarla değerlendir, her biri en fazla üç madde:
   - **Değer**: kime, hangi sorunu çözüyor?
   - **Fizibilite**: tek bir geliştirici için zorluk (düşük / orta / yüksek) ve nedeni
   - **Riskler**: en önemli belirsizlikler
   - **İlk adım**: bir günde yapılabilecek somut bir başlangıç
3. Fikrin `status` alanını `"reviewed"` yap ve diğer alanlara dokunmadan dosyayı kaydet.
4. Değerlendirmeyi kısa ve dürüst tut; zayıf bir fikri zayıf olarak söyle.
```

| Alan | Anlamı |
| :- | :- |
| `name` | Agent adı. Plugin içinde `ideas:idea-reviewer` olur. |
| `description` | Claude'un bu agent'a ne zaman iş devredeceğini belirler. |
| `tools` | Agent'ın kullanabileceği araçlar. Yazılmazsa ana konuşmanın tüm araçlarını alır. Burada yalnızca okuma ve düzenleme veriyoruz; komut çalıştıramaz. |
| `model` | Agent'ın kullanacağı model (`sonnet`, `opus`, `haiku` veya `inherit`). |

Gövde, agent'ın **sistem talimatıdır**: agent kendini bu metinle tanımlar.

### Deneyin

```text
/reload-plugins
ideas:idea-reviewer agent'ını kullanarak son fikrimi değerlendir
```

Agent'ın ayrı bir görev olarak çalıştığını, sonra değerlendirmeyi döndürdüğünü göreceksiniz. Ardından `/ideas:list reviewed` ile fikrin durumunun değiştiğini kontrol edin.

**Ne öğrendik:** Agent = ayrı bağlamda çalışan, araçları kısıtlanabilen yardımcı. Skill "nasıl yapılır" der, agent "kim yapar" sorusuna cevaptır.

---

## 5. Hook: oturum başında özet

Skill ve agent'ları **Claude** çalıştırır. **Hook**'ları ise **Claude Code'un kendisi** belirli olaylarda, Claude'a sormadan çalıştırır. Bu yüzden "her zaman olsun" dediğiniz davranışlar için doğru araçtır.

Bazı olaylar:

| Olay | Ne zaman |
| :- | :- |
| `SessionStart` | Oturum başladığında |
| `UserPromptSubmit` | Kullanıcı mesaj gönderdiğinde |
| `PreToolUse` / `PostToolUse` | Claude bir aracı kullanmadan önce / sonra (`matcher` ile araç seçilir, örn. `"Write\|Edit"`) |
| `Stop` | Claude yanıtını bitirdiğinde |

Bizim hook'umuz `SessionStart`'ta çalışıp fikir sayısını Claude'a bildirecek. `SessionStart` hook'unun **stdout'a yazdığı metin Claude'un bağlamına eklenir**. Yani Claude, siz sormadan fikir havuzunun durumunu bilir.

### Hook yapılandırması

**`plugins/ideas/hooks/hooks.json`**

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "node \"${CLAUDE_PLUGIN_ROOT}/scripts/session-summary.js\""
          }
        ]
      }
    ]
  }
}
```

Yapı, `settings.json` içindeki `hooks` nesnesiyle birebir aynı. Olay adı → matcher grupları listesi → her grupta çalışacak komutlar.

**Yol değişkenleri.** Plugin'iniz kullanıcının makinesinde nereye kurulacak bilemezsiniz, bu yüzden sabit yol yazmayın:

| Değişken | Anlamı |
| :- | :- |
| `${CLAUDE_PLUGIN_ROOT}` | Plugin'in kurulu olduğu klasör. Her sürümde değişir, **buraya veri yazmayın**. |
| `${CLAUDE_PLUGIN_DATA}` | Güncellemelerde silinmeyen, plugin'e özel veri klasörü (`~/.claude/plugins/data/<id>/`). Önbellek, `node_modules` vb. için. |
| `${CLAUDE_PROJECT_DIR}` | Kullanıcının o anki proje kökü. |

> ⚠️ **Tırnaklama:** `command` bir kabukta çalışır. Yolda boşluk olabileceği için (`C:\Users\Ad Soyad\...`) `${CLAUDE_PLUGIN_ROOT}/...` yolunu çift tırnak içine alın. JSON içinde bu `\"` olarak yazılır.

### Script

Script'i Node ile yazıyoruz; Windows, macOS ve Linux'ta aynı çalışır (`bash`/`jq` her makinede yoktur).

**`plugins/ideas/scripts/session-summary.js`**

```js
// SessionStart hook: ideas.json'daki fikirleri özetler.
// Bu script'in stdout'a yazdığı metin, oturumun başında Claude'un bağlamına eklenir.
const fs = require("fs");
const path = require("path");

const projectDir = process.env.CLAUDE_PROJECT_DIR || process.cwd();
const file = path.join(projectDir, "ideas.json");

if (!fs.existsSync(file)) process.exit(0);

let ideas;
try {
  ideas = JSON.parse(fs.readFileSync(file, "utf8"));
} catch {
  console.log("Fikir havuzu: ideas.json okunamadı (geçersiz JSON).");
  process.exit(0);
}
if (!Array.isArray(ideas) || ideas.length === 0) process.exit(0);

const counts = {};
for (const idea of ideas) counts[idea.status] = (counts[idea.status] || 0) + 1;
const summary = Object.entries(counts)
  .map(([status, n]) => `${n} ${status}`)
  .join(", ");

console.log(`Fikir havuzu: ${ideas.length} fikir (${summary}). Listelemek için /ideas:list.`);
```

Dikkat edilecekler:

- Hook'lar `CLAUDE_PROJECT_DIR`, `CLAUDE_PLUGIN_ROOT` ve `CLAUDE_PLUGIN_DATA`'yı **ortam değişkeni** olarak da alır.
- **Çıkış kodu önemlidir.** `0` başarı demektir. `2` engelleyici hata demektir (örneğin `PreToolUse`'ta aracı durdurur). Bu yüzden dosya yokken bile sessizce `0` ile çıkıyoruz; bir özet hook'u oturumu asla bozmamalı.

### Deneyin

Önce script'i tek başına çalıştırın. Hook hata ayıklamanın en kolay yolu budur:

```bash
node plugins/ideas/scripts/session-summary.js
# Fikir havuzu: 1 fikir (1 reviewed). Listelemek için /ideas:list.
```

Sonra **yeni bir oturum** açın (`SessionStart` yalnızca oturum başında tetiklenir):

```bash
claude --plugin-dir ./plugins/ideas
```

```text
Fikir havuzumda kaç fikir var?
```

Claude, hiçbir araç çağırmadan cevap verebilmeli; bilgi hook'tan geldi. Hook'un çalışıp çalışmadığını görmek için `claude --debug --plugin-dir ./plugins/ideas` ile başlatın. Debug log'u hangi hook'un eşleştiğini, çıkış kodunu ve çıktısını gösterir.

**Ne öğrendik:** Hook'lar deterministiktir; Claude değil Claude Code çalıştırır. Plugin dosyalarına `${CLAUDE_PLUGIN_ROOT}` ile, tırnak içinde başvurulur.

---

## 6. Test ve hata ayıklama kontrol listesi

Bir değişiklik görünmüyorsa sırayla:

1. **`claude plugin validate ./plugins/ideas --strict`**: manifest'i ve tüm skill/agent frontmatter'larını kontrol eder. `--strict` uyarılarda da başarısız olur.
2. **`/reload-plugins`**: diskteki değişiklikleri yükledi mi? `Reloaded:` satırındaki sayılara bakın.
3. **`/plugin`**: **Installed** sekmesinde plugin'in ayrıntılarında bulunan bileşenler listelenir. **Errors** sekmesi neyin neden yüklenmediğini söyler.
4. **`claude --plugin-dir ./plugins/ideas plugin list`**: kabuktan durum kontrolü. `Status: ✔ loaded` görmelisiniz.
5. **Hook'lar için** `claude --debug` ve script'i tek başına çalıştırma.

Sık hatalar:

| Belirti | Neden |
| :- | :- |
| Plugin yükleniyor ama skill'ler yok | `skills/` klasörü `.claude-plugin/` içine konmuş. Plugin köküne taşıyın. |
| `... path not found` hatası | Manifest'te bir bileşen yolu yanlış veya klasör yok. |
| `--plugin-dir` hiçbir şey yüklemiyor | Plugin klasörü yerine bir marketplace kökü verilmiş. `--plugin-dir` doğrudan `plugin.json`'ı içeren klasörü ister. |
| Skill `/add` olarak çalışmıyor | Plugin skill'leri önekli çağrılır: `/ideas:add`. |
| Hook iki kez çalışıyor | Aynı hook hem `settings.json`'da hem plugin'de tanımlı. Hook'ların öneki yoktur. |

Plugin hatasız yüklenip yine de Claude'u istediğiniz gibi yönlendirmiyorsa, bunu ölçmek için `claude plugin eval` var: test senaryolarını plugin'li ve plugin'siz çalıştırıp farkı puanlar.

---

## 7. Sonraki adımlar: paylaşma ve daha fazlası

**Plugin'i her oturumda yüklemek.** `--plugin-dir`'i her seferinde yazmak istemiyorsanız, plugin klasörünü `~/.claude/skills/ideas/` altına koyun (içinde `.claude-plugin/plugin.json` olduğu sürece her oturumda yüklenir). Bunun yerine bir projede herkes için yüklenmesini istiyorsanız `<proje>/.claude/skills/ideas/` altına koyun. `claude plugin init <ad>` bu yapıyı sizin için iskeletler.

**Paylaşmak.** Üç yol var:

1. **Doğrudan**: klasörü veya `.zip`'ini gönderin. Karşı taraf `claude --plugin-dir ./ideas.zip` ile kullanır.
2. **Kendi marketplace'iniz**: bir git reposuna `.claude-plugin/marketplace.json` koyup plugin'leri listeleyin. Takım arkadaşlarınız `/plugin marketplace add <repo>` ile ekler ve `/plugin install ideas@<marketplace>` ile kurar. Güncellemelerinizi de böyle alırlar.
3. **Anthropic dizini**: inceleme sonrası claude.ai ve Cowork kullanıcılarına da ulaşır.

**Henüz değinmediğimiz bileşenler:**

- **MCP sunucusu** (`.mcp.json`): Claude'a gerçek araçlar ekler. Örneğin `ideas.json` yerine bir veritabanına yazan `add_idea` aracı. Talimat yerine kod çalıştırmanız gereken her yerde bir sonraki adım budur.
- **`userConfig`**: kurulumda kullanıcıdan ayar istemek (API anahtarı, dosya yolu vb.).
- **LSP sunucuları, monitor'lar, bağımlılıklar**: bileşenler sayfasında.

**Yardımcı plugin:** Anthropic'in resmi marketplace'indeki `plugin-dev` plugin'i, plugin tasarlama, iskeletleme ve doğrulama için skill ve agent'lar içerir. `/plugin` → Discover sekmesinden kurun, sonra `/plugin-dev:create-plugin <ne istediğiniz>` ile başlayın.

### Alıştırmalar

1. Bir fikrin durumunu değiştiren `/ideas:done <id>` skill'i yazın.
2. `idea-reviewer`'a, değerlendirmeyi fikrin içine `review` alanı olarak kaydetmesini söyleyin.
3. `ideas.json` her düzenlendiğinde geçerli JSON olup olmadığını kontrol eden bir `PostToolUse` hook'u ekleyin (`matcher: "Write|Edit"`). İpucu: hook, aracın girdisini stdin'den JSON olarak alır; `tool_input.file_path` alanına bakın.

### Kaynaklar (resmi dokümanlar)

- Plugin'lere genel bakış: https://code.claude.com/docs/en/plugins
- İlk plugin'i oluşturma: https://code.claude.com/docs/en/plugins/create
- Tüm bileşenler ve örnekleri: https://code.claude.com/docs/en/plugins/components
- Manifest referansı: https://code.claude.com/docs/en/plugins/manifest-reference
- Skill'ler: https://code.claude.com/docs/en/skills
- Hook'lar: https://code.claude.com/docs/en/hooks
- Yayınlama: https://code.claude.com/docs/en/plugins/publish
- Örnek plugin'ler: https://github.com/anthropics/claude-code/tree/main/plugins
