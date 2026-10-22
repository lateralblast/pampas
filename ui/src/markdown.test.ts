import assert from "node:assert/strict";
import { test } from "node:test";
import { renderInline, renderMarkdown } from "./markdown.js";
import { assignSigla, expandRefs, markNames, siglum } from "./siglum.js";

test("text from a model can never inject markup", () => {
  const html = renderMarkdown('<script>alert(1)</script> <img src=x onerror="alert(1)"> **bold**');
  assert.ok(!html.includes("<script"), html);
  assert.ok(!html.includes("<img"), html);
  assert.ok(html.includes("&lt;script&gt;") && html.includes("<strong>bold</strong>"));
});

test("links are never anchors, and only web addresses are shown", () => {
  const ok = renderInline("see [the docs](https://example.com/a?b=1&c=2)");
  assert.ok(!ok.includes("<a "), ok);
  assert.ok(ok.includes("the docs") && ok.includes("https://example.com/a?b=1&amp;c=2"));
  const bad = renderInline("[click](javascript:alert(1)) and [x](data:text/html,hi)");
  assert.ok(!bad.includes("javascript") && !bad.includes("data:"), bad);
  assert.ok(bad.includes("click") && bad.includes("x"));
  assert.equal(renderInline("![a picture](https://x.test/i.png)"), "a picture");
});

test("inline code is literal and emphasis works around it", () => {
  assert.equal(renderInline("use `a *b* c` and *this*"), "use <code>a *b* c</code> and <em>this</em>");
  assert.equal(renderInline("snake_case_name is not italic"), "snake_case_name is not italic");
  assert.equal(renderInline("2 * 3 * 4"), "2 * 3 * 4");
});

test("headings shift down, lists nest, code blocks are escaped", () => {
  const html = renderMarkdown("# Title\n\n- one\n  - nested\n- two\n\n1. first\n2. second\n\n```js\nif (a < b) {}\n```");
  assert.ok(html.includes("<h3>Title</h3>"), html);
  assert.ok(html.includes("<ul><li>one<ul><li>nested</li></ul></li><li>two</li></ul>"), html);
  assert.ok(html.includes("<ol><li>first</li><li>second</li></ol>"), html);
  assert.ok(html.includes('<pre><code data-lang="js">if (a &lt; b) {}</code></pre>'), html);
});

test("tables, quotes and hard breaks", () => {
  const html = renderMarkdown("| a | b |\n|---|--:|\n| 1 | 2 |\n\n> quoted *text*\n\nline one  \nline two");
  assert.ok(html.includes("<table>") && html.includes("<th>a</th>") && html.includes('<td style="text-align:right">2</td>'), html);
  assert.ok(html.includes("<blockquote><p>quoted <em>text</em></p></blockquote>"), html);
  assert.ok(html.includes("line one<br>line two"), html);
});

test("sigla run a to z then aa", () => {
  assert.equal(siglum(0), "a");
  assert.equal(siglum(25), "z");
  assert.equal(siglum(26), "aa");
  assert.equal(siglum(27), "ab");
});

test("source names in text become siglum marks, longest and whole names only", () => {
  const sigla = assignSigla(["groq", "groq-120b", "gemini"]);
  const html = expandRefs(renderMarkdown(markNames("**Groq** and groq-120b agree; Gemini differs, but groqs is not a source.", sigla)), sigla);
  assert.ok(html.includes('<strong><span class="ref">Groq<button type="button" class="sig" data-ref="groq" aria-label="Show the answer from groq">a</button></span></strong>'), html);
  assert.ok(html.includes('<span class="ref">groq-120b<button type="button" class="sig" data-ref="groq-120b" aria-label="Show the answer from groq-120b">b</button></span>'), html);
  assert.ok(html.includes('<span class="ref">Gemini<button type="button" class="sig" data-ref="gemini" aria-label="Show the answer from gemini">c</button></span>'), html);
  assert.ok(html.includes("groqs is not a source"), html);
});
