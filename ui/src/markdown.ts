// A small, safe markdown renderer for model answers. Everything is HTML-escaped before any markup is
// added, so text from a model can never inject elements, and links are never real anchors (a click
// would navigate the app window): a link is shown as its text with the address beside it.

const ESC: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };

export function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, (c) => ESC[c] ?? c);
}

/** Only plain web addresses are ever shown. */
function safeUrl(url: string): string | null {
  const u = url.trim();
  return /^https?:\/\/[^\s<>"']+$/i.test(u) ? u : null;
}

/** Inline markup: code, bold, italic, strikethrough, links and images. */
export function renderInline(src: string): string {
  const codes: string[] = [];
  // Code spans first, so their contents are never read as markup.
  let text = src.replace(/(`+)([^`]|[^`][\s\S]*?[^`])\1(?!`)/g, (_m, _ticks: string, code: string) => {
    codes.push(`<code>${escapeHtml(code.trim())}</code>`);
    return `\u0000${codes.length - 1}\u0000`;
  });
  text = escapeHtml(text);
  // Images and links become text; the address is kept visible when it is a plain web address.
  text = text.replace(/!\[([^\]]*)\]\(([^)\s]*)[^)]*\)/g, (_m, alt: string) => alt);
  text = text.replace(/\[([^\]]+)\]\(([^)\s]+)[^)]*\)/g, (_m, label: string, url: string) => {
    const shown = safeUrl(url.replace(/&amp;/g, "&"));
    return shown
      ? `<span class="mdlink" title="${escapeHtml(shown)}">${label}</span> <span class="mdurl">(${escapeHtml(shown)})</span>`
      : label;
  });
  text = text
    .replace(/\*\*([^*\n]+?)\*\*/g, "<strong>$1</strong>")
    .replace(/(^|[^\w])__([^_\n]+?)__(?!\w)/g, "$1<strong>$2</strong>")
    .replace(/(^|[^*\w])\*([^*\s][^*\n]*?)\*(?!\*)/g, "$1<em>$2</em>")
    .replace(/(^|[^\w])_([^_\s][^_\n]*?)_(?!\w)/g, "$1<em>$2</em>")
    .replace(/~~([^~\n]+?)~~/g, "<del>$1</del>");
  return text.replace(/\u0000(\d+)\u0000/g, (_m, i: string) => codes[Number(i)] ?? "");
}

interface Options {
  /** Added to the heading level, so an answer's `#` becomes a small heading inside its card. */
  headingOffset?: number;
}

const LIST_RE = /^(\s*)([-*+]|\d{1,9}[.)])\s+(.*)$/;
const FENCE_RE = /^\s{0,3}(```+|~~~+)\s*([\w+-]*)\s*$/;
const HR_RE = /^\s{0,3}([-*_])(\s*\1){2,}\s*$/;
const TABLE_SEP_RE = /^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$/;

function splitRow(line: string): string[] {
  let row = line.trim();
  if (row.startsWith("|")) row = row.slice(1);
  if (row.endsWith("|")) row = row.slice(0, -1);
  return row.split("|").map((c) => c.trim());
}

function renderList(lines: string[]): string {
  interface Item {
    indent: number;
    ordered: boolean;
    text: string[];
  }
  const items: Item[] = [];
  for (const line of lines) {
    const m = LIST_RE.exec(line);
    if (m) {
      items.push({ indent: (m[1] ?? "").replace(/\t/g, "    ").length, ordered: /\d/.test(m[2] ?? ""), text: [m[3] ?? ""] });
    } else if (items.length) {
      items[items.length - 1]?.text.push(line.trim());
    }
  }
  function build(start: number, indent: number): [string, number] {
    const first = items[start];
    if (!first) return ["", start];
    const tag = first.ordered ? "ol" : "ul";
    let html = `<${tag}>`;
    let i = start;
    while (i < items.length) {
      const item = items[i];
      // A shallower item closes this list; a deeper one belongs to the item before it.
      if (!item || item.indent !== indent) break;
      let body = renderInline(item.text.join(" "));
      i++;
      const next = items[i];
      if (next && next.indent > indent) {
        const [sub, after] = build(i, next.indent);
        body += sub;
        i = after;
      }
      html += `<li>${body}</li>`;
    }
    return [`${html}</${tag}>`, i];
  }
  let out = "";
  let i = 0;
  while (i < items.length) {
    const [html, next] = build(i, items[i]?.indent ?? 0);
    out += html;
    i = next > i ? next : i + 1;
  }
  return out;
}

/** Block markup: headings, paragraphs, lists, quotes, code blocks, rules and tables. */
export function renderMarkdown(src: string, opts: Options = {}): string {
  const offset = opts.headingOffset ?? 2;
  const lines = src.replace(/\r\n?/g, "\n").split("\n");
  const out: string[] = [];
  let para: string[] = [];
  const flush = () => {
    if (!para.length) return;
    const html = para
      .map((l, i) => (i < para.length - 1 && / {2,}$/.test(l) ? renderInline(l.trimEnd()) + "<br>" : renderInline(l.trim())))
      .join(" ");
    out.push(`<p>${html.replace(/<br> /g, "<br>")}</p>`);
    para = [];
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i] ?? "";
    const fence = FENCE_RE.exec(line);
    if (fence) {
      flush();
      const marker = (fence[1] ?? "```")[0] ?? "`";
      const code: string[] = [];
      i++;
      while (i < lines.length && !new RegExp(`^\\s{0,3}${marker}{3,}\\s*$`).test(lines[i] ?? "")) code.push(lines[i++] ?? "");
      out.push(`<pre><code${fence[2] ? ` data-lang="${escapeHtml(fence[2])}"` : ""}>${escapeHtml(code.join("\n"))}</code></pre>`);
      continue;
    }
    if (!line.trim()) {
      flush();
      continue;
    }
    const heading = /^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$/.exec(line);
    if (heading) {
      flush();
      const level = Math.min(6, (heading[1]?.length ?? 1) + offset);
      out.push(`<h${level}>${renderInline(heading[2] ?? "")}</h${level}>`);
      continue;
    }
    if (HR_RE.test(line)) {
      flush();
      out.push("<hr>");
      continue;
    }
    if (/^\s{0,3}>/.test(line)) {
      flush();
      const quote: string[] = [];
      while (i < lines.length && /^\s{0,3}>/.test(lines[i] ?? "")) quote.push((lines[i++] ?? "").replace(/^\s{0,3}>\s?/, ""));
      i--;
      out.push(`<blockquote>${renderMarkdown(quote.join("\n"), opts)}</blockquote>`);
      continue;
    }
    if (LIST_RE.test(line)) {
      flush();
      const list: string[] = [];
      while (i < lines.length) {
        const l = lines[i] ?? "";
        if (LIST_RE.test(l)) list.push(l);
        else if (l.trim() && /^\s{2,}\S/.test(l) && list.length) list.push(l);
        else break;
        i++;
      }
      i--;
      out.push(renderList(list));
      continue;
    }
    if (line.includes("|") && TABLE_SEP_RE.test(lines[i + 1] ?? "")) {
      flush();
      const head = splitRow(line);
      const align = splitRow(lines[i + 1] ?? "").map((c) => (/^:-+:$/.test(c) ? "center" : /-+:$/.test(c) ? "right" : ""));
      i += 2;
      const rows: string[][] = [];
      while (i < lines.length && (lines[i] ?? "").includes("|") && (lines[i] ?? "").trim()) rows.push(splitRow(lines[i++] ?? ""));
      i--;
      const cell = (tag: string, text: string, n: number) =>
        `<${tag}${align[n] ? ` style="text-align:${align[n]}"` : ""}>${renderInline(text)}</${tag}>`;
      out.push(
        `<div class="table-wrap"><table><thead><tr>${head.map((c, n) => cell("th", c, n)).join("")}</tr></thead><tbody>${rows
          .map((r) => `<tr>${head.map((_c, n) => cell("td", r[n] ?? "", n)).join("")}</tr>`)
          .join("")}</tbody></table></div>`,
      );
      continue;
    }
    para.push(line);
  }
  flush();
  return out.join("\n");
}
