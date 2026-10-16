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
