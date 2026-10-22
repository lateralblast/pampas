// Each source gets a siglum, a short letter (a, b, c ... z, aa ...) as in a critical edition, kept in
// the order of the config so a source keeps its letter from one question to the next.

export function siglum(index: number): string {
  let n = index;
  let out = "";
  do {
    out = String.fromCharCode(97 + (n % 26)) + out;
    n = Math.floor(n / 26) - 1;
  } while (n >= 0);
  return out;
}

/** A map from source name to siglum, in the given order. */
export function assignSigla(names: string[]): Map<string, string> {
  return new Map(names.map((name, i) => [name, siglum(i)]));
}

const OPEN = "";
const CLOSE = "";

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * Mark each mention of a source's name in `text` with a sentinel, longest names first and only whole
 * names (so `groq` does not match inside `groq-120b`), ignoring case. `expandRefs` turns the sentinels
 * into siglum marks after the text has been rendered.
 */
export function markNames(text: string, sigla: Map<string, string>): string {
  const names = [...sigla.keys()].sort((a, b) => b.length - a.length);
  if (!names.length) return text;
  const re = new RegExp(`(?<![\\w-])(${names.map(escapeRegExp).join("|")})(?![\\w-])`, "gi");
  const byLower = new Map([...sigla.keys()].map((n) => [n.toLowerCase(), n]));
  return text.replace(re, (match) => {
    const name = byLower.get(match.toLowerCase());
    return name ? `${OPEN}${name}${CLOSE}${match}${OPEN}${CLOSE}` : match;
  });
}

/** Replace the sentinels left by `markNames` with marks: a button carrying the siglum that jumps to that source's row. */
export function expandRefs(html: string, sigla: Map<string, string>): string {
  return html.replace(new RegExp(`${OPEN}([^${CLOSE}]*)${CLOSE}([\\s\\S]*?)${OPEN}${CLOSE}`, "g"), (_m, name: string, shown: string) => {
    const s = sigla.get(name);
    return s
      ? `<span class="ref">${shown}<button type="button" class="sig" data-ref="${name.replace(/"/g, "&quot;")}" aria-label="Show the answer from ${name.replace(/"/g, "&quot;")}">${s}</button></span>`
      : shown;
  });
}
