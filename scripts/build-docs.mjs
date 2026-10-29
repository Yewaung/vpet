// Builds docs-review.html: every product document in one self-contained
// page with a sidebar, search and print. Pictures are embedded, so the file
// works on its own.   npm run docs   (or: node scripts/build-docs.mjs)

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { marked } from "marked";

const root = process.argv[2] ?? ".";
const out = process.argv[3] ?? join(root, "docs-review.html");
const docs = [
  ["ROADMAP.md", "Roadmap & USP"],
  ["docs/LIFECYCLE.md", "Life cycle: 12 stages"],
  ["README.md", "README"],
  ["overview.md", "Product Spec"],
  ["docs/RELEASING.md", "Releasing"],
  ["docs/LOCALIZATION.md", "Localization plan"],
  ["THIRD_PARTY_NOTICES.md", "Third-party notices"],
];

const slug = (s) => s.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
/** Links between documents point at their page, whatever folder they're in. */
const pageOf = new Map(docs.map(([file]) => [basename(file).toLowerCase(), slug(basename(file))]));

/** Pictures become data URIs so the page stands alone. */
function embed(html, file) {
  return html.replace(/<img src="([^":]+\.png)"/g, (m, src) => {
    const path = join(root, dirname(file), src);
    if (!existsSync(path)) return m;
    return `<img src="data:image/png;base64,${readFileSync(path).toString("base64")}"`;
  });
}

const sections = docs.filter(([file]) => existsSync(join(root, file))).map(([file, title]) => {
  const md = readFileSync(join(root, file), "utf8");
  const id = slug(basename(file));
  const headings = [];
  const renderer = new marked.Renderer();
  renderer.heading = function ({ tokens, depth }) {
    const text = this.parser.parseInline(tokens);
    const hid = `${id}--${slug(text.replace(/<[^>]+>/g, ""))}-${headings.length}`;
    if (depth <= 2) headings.push({ hid, text: text.replace(/<[^>]+>/g, ""), depth });
    return `<h${depth} id="${hid}">${text}</h${depth}>\n`;
  };
  const html = marked.parse(md, { renderer, gfm: true })
    .replace(/<a href="([^"#:]+\.md)">/g, (m, f) => { const p = pageOf.get(basename(f).toLowerCase()); return p ? `<a href="#${p}" data-doc="${p}">` : m; })
    .replace(/<li>\[ \]/g, '<li class="task">☐')
    .replace(/<li>\[x\]/gi, '<li class="task done">☑');
  return { id, title, file, html: embed(html, file), headings };
});

const nav = sections.map((s) => `
  <div class="doc">
    <button class="doc-btn" data-doc="${s.id}">${s.title}<small>${s.file}</small></button>
    <ul class="toc" data-for="${s.id}">
      ${s.headings.map((h) => `<li class="d${h.depth}"><a href="#${h.hid}" data-doc="${s.id}">${h.text}</a></li>`).join("")}
    </ul>
  </div>`).join("");

const body = sections.map((s) => `<article id="${s.id}" class="page">${s.html}</article>`).join("");

const page = `<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>VPet – Project Docs Review</title>
<style>
:root{--bg:#f7f5f0;--panel:#fff;--ink:#1f2328;--muted:#6b7280;--line:#e5e1d8;--accent:#7c3aed;--code:#f1eee7}
@media (prefers-color-scheme:dark){:root{--bg:#15161a;--panel:#1d1f24;--ink:#e6e6e6;--muted:#9aa0a6;--line:#2c2f36;--accent:#a78bfa;--code:#262930}}
*{box-sizing:border-box}
body{margin:0;font:16px/1.65 system-ui,-apple-system,"Segoe UI",sans-serif;background:var(--bg);color:var(--ink);display:grid;grid-template-columns:300px 1fr;height:100vh}
nav{border-right:1px solid var(--line);background:var(--panel);overflow:auto;padding:16px}
nav h1{font-size:15px;margin:0 0 4px}nav p{margin:0 0 16px;color:var(--muted);font-size:12px}
#q{width:100%;padding:8px 10px;border:1px solid var(--line);border-radius:8px;background:var(--bg);color:var(--ink);margin-bottom:12px}
.doc-btn{display:block;width:100%;text-align:left;border:0;background:none;color:var(--ink);font:600 14px system-ui;padding:8px 10px;border-radius:8px;cursor:pointer}
.doc-btn small{display:block;font-weight:400;color:var(--muted);font-size:11px}
.doc-btn.active{background:color-mix(in srgb,var(--accent) 14%,transparent);color:var(--accent)}
.toc{list-style:none;margin:2px 0 10px;padding:0 0 0 10px;display:none;font-size:13px}
.toc.open{display:block}.toc li.d2{padding-left:12px}
.toc a{color:var(--muted);text-decoration:none;display:block;padding:2px 4px;border-radius:4px}
.toc a:hover{color:var(--accent)}
main{overflow:auto;padding:32px 48px}
.page{display:none;max-width:900px;margin:0 auto;background:var(--panel);border:1px solid var(--line);border-radius:14px;padding:36px 48px}
.page.show{display:block}
h1,h2,h3{line-height:1.25;scroll-margin-top:16px}
h1{border-bottom:2px solid var(--line);padding-bottom:8px}
h2{border-bottom:1px solid var(--line);padding-bottom:6px;margin-top:2em}
a{color:var(--accent)}
code{background:var(--code);padding:1px 5px;border-radius:4px;font:13.5px ui-monospace,Consolas,monospace}
pre{background:var(--code);padding:14px;border-radius:8px;overflow:auto}pre code{padding:0;background:none}
table{border-collapse:collapse;width:100%;margin:1em 0;font-size:14px;display:block;overflow:auto}
th,td{border:1px solid var(--line);padding:7px 10px;text-align:left;vertical-align:top}
th{background:var(--code)}tr:nth-child(even) td{background:color-mix(in srgb,var(--code) 45%,transparent)}
blockquote{margin:1em 0;padding:10px 16px;border-left:4px solid var(--accent);background:color-mix(in srgb,var(--accent) 8%,transparent);border-radius:0 8px 8px 0}
img{max-width:100%;image-rendering:pixelated;vertical-align:middle}td img{width:96px}li.task{list-style:none;margin-left:-1.2em}li.task.done{color:var(--muted)}
mark{background:#fde68a;color:#000}
.toolbar{max-width:900px;margin:0 auto 12px;display:flex;justify-content:flex-end;gap:8px}
.toolbar .btn{border:1px solid var(--line);background:var(--panel);color:var(--ink);border-radius:8px;padding:6px 12px;text-decoration:none;font-size:14px}.toolbar button{border:1px solid var(--line);background:var(--panel);color:var(--ink);border-radius:8px;padding:6px 12px;cursor:pointer}
@media print{body{display:block;height:auto}nav,.toolbar{display:none}main{overflow:visible;padding:0}.page{display:block!important;border:0;page-break-after:always}}
@media (max-width:800px){body{grid-template-columns:1fr}nav{display:none}main{padding:12px}.page{padding:20px}}
</style></head>
<body>
<nav>
  <h1>VPet docs review</h1>
  <p>Generated ${new Date().toISOString().slice(0, 10)} from the project's Markdown files</p>
  <input id="q" type="search" placeholder="Search in current doc…">
  ${nav}
</nav>
<main>
  <div class="toolbar">${existsSync(join(root, "roadmap.html")) ? `<a class="btn" href="roadmap.html" target="_blank">Roadmap dashboard ↗</a>` : ""}<button onclick="window.print()">Print / Save PDF</button></div>
  ${body}
</main>
<script>
const show = (id, hash) => {
  document.querySelectorAll('.page').forEach(p => p.classList.toggle('show', p.id === id));
  document.querySelectorAll('.doc-btn').forEach(b => b.classList.toggle('active', b.dataset.doc === id));
  document.querySelectorAll('.toc').forEach(t => t.classList.toggle('open', t.dataset.for === id));
  if (hash) document.getElementById(hash)?.scrollIntoView(); else document.querySelector('main').scrollTop = 0;
  highlight();
};
document.addEventListener('click', e => {
  const el = e.target.closest('[data-doc]'); if (!el) return;
  e.preventDefault();
  const href = el.getAttribute('href');
  show(el.dataset.doc, href && href.includes('--') ? href.slice(1) : null);
});
const originals = new Map();
const highlight = () => {
  const q = document.getElementById('q').value.trim();
  const page = document.querySelector('.page.show');
  if (!originals.has(page.id)) originals.set(page.id, page.innerHTML);
  page.innerHTML = originals.get(page.id);
  if (q.length < 2) return;
  const re = new RegExp(q.replace(/[.*+?^\${}()|[\\]\\\\]/g, '\\\\$&'), 'gi');
  const walker = document.createTreeWalker(page, NodeFilter.SHOW_TEXT);
  const nodes = []; while (walker.nextNode()) if (re.test(walker.currentNode.nodeValue)) nodes.push(walker.currentNode);
  nodes.forEach(n => { const s = document.createElement('span'); s.innerHTML = n.nodeValue.replace(/[&<>]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;'}[c])).replace(re, m => '<mark>' + m + '</mark>'); n.replaceWith(s); });
  page.querySelector('mark')?.scrollIntoView({block: 'center'});
};
document.getElementById('q').addEventListener('input', highlight);
// A link like docs-review.html#lifecycle-md opens that document.
const start = location.hash.slice(1);
show(document.querySelector('.page#' + CSS.escape(start || 'x')) ? start : '${sections[0].id}');
</script>
</body></html>`;

writeFileSync(out, page);
console.log("wrote", out, page.length, "bytes");
