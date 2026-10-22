// A few helpers for building elements without a framework.

export type Child = Node | string | number | null | undefined | false;
type Props = Record<string, unknown> | null;

/** Create an element. `class`, `dataset`, `on<event>` handlers, and DOM properties are understood; anything else is an attribute. */
export function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props?: Props,
  ...children: Child[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [key, value] of Object.entries(props ?? {})) {
    if (value === undefined || value === null || value === false) continue;
    if (key === "class") el.className = String(value);
    else if (key === "dataset") Object.assign(el.dataset, value as Record<string, string>);
    else if (key.startsWith("on") && typeof value === "function") {
      el.addEventListener(key.slice(2).toLowerCase(), value as EventListener);
    } else if (key in el && !key.includes("-") && key !== "list" && key !== "form") {
      (el as unknown as Record<string, unknown>)[key] = value;
    } else el.setAttribute(key, value === true ? "" : String(value));
  }
  append(el, children);
  return el;
}

export function append(el: Element, children: Child[]): void {
  for (const c of children) {
    if (c === null || c === undefined || c === false) continue;
    el.append(c instanceof Node ? c : document.createTextNode(String(c)));
  }
}

export function clear(el: Element): void {
  el.replaceChildren();
}

const ICONS: Record<string, string> = {
  check: "M3.5 8.5l3 3 6-7",
  cross: "M4 4l8 8M12 4l-8 8",
  clock: "M8 4.5V8l2.5 1.5M8 14a6 6 0 1 1 0-12 6 6 0 0 1 0 12z",
  key: "M10.5 5.5a3 3 0 1 1-1.2 2.4L3 14.2V12.5H4.7V11H6.4L8.1 9.3M11.5 6.5h.01",
  plus: "M8 3v10M3 8h10",
  trash: "M3 4.5h10M6.5 4.5V3h3v1.5M5 4.5l.5 8.5h5l.5-8.5",
  up: "M4 9.5l4-4 4 4",
  down: "M4 6.5l4 4 4-4",
  copy: "M5.5 5.5h7v8h-7zM3.5 10.5v-8h7",
  chevron: "M6 3.5l4.5 4.5L6 12.5",
  warn: "M8 2.5l6 10.5H2zM8 7v3M8 11.5h.01",
  edit: "M3 13l.7-3L11 2.7 13.3 5 6 12.3zM9.5 4.2l2.3 2.3",
  dot: "M8 8h.01",
};

/** A small drawn icon (one stroke weight, 16px grid), hidden from screen readers. */
export function icon(name: keyof typeof ICONS | string): SVGSVGElement {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 16 16");
  svg.setAttribute("width", "16");
  svg.setAttribute("height", "16");
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("focusable", "false");
  svg.setAttribute("class", "icon");
  const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
  path.setAttribute("d", ICONS[name] ?? "");
  path.setAttribute("fill", "none");
  path.setAttribute("stroke", "currentColor");
  path.setAttribute("stroke-width", "1.5");
  path.setAttribute("stroke-linecap", "round");
  path.setAttribute("stroke-linejoin", "round");
  svg.append(path);
  return svg;
}

/** Say something to screen readers, politely, or assertively for errors. */
export function announce(message: string, assertive = false): void {
  const el = document.getElementById(assertive ? "live-assertive" : "live-polite");
  if (!el) return;
  el.textContent = "";
  // A change of text is what is announced; clear first so repeated messages are read again.
  window.setTimeout(() => (el.textContent = message), 30);
}

/** Text with line-break opportunities after . / - : _ so a long model or host name wraps between parts, not mid-word. */
export function breakable(text: string): DocumentFragment {
  const frag = document.createDocumentFragment();
  for (const part of text.split(/(?<=[./\-:_])/)) {
    frag.append(document.createTextNode(part), document.createElement("wbr"));
  }
  return frag;
}

/** Copy text to the clipboard; returns whether it worked. */
export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const area = h("textarea", { value: text, readOnly: true, style: "position:fixed;opacity:0" });
    document.body.append(area);
    area.select();
    let ok = false;
    try {
      ok = document.execCommand("copy");
    } catch {
      ok = false;
    }
    area.remove();
    return ok;
  }
}

/**
 * A button that copies `getText()` to the clipboard and says so: its label becomes "Copied" for a
 * moment (or "Could not copy"), and screen readers are told.
 */
export function copyButton(label: string, announcement: string, getText: () => string | Promise<string>, className = "quiet"): HTMLButtonElement {
  const button = h("button", { type: "button", class: className });
  const rest = () => {
    clear(button);
    append(button, [icon("copy"), label]);
  };
  rest();
  button.addEventListener("click", async () => {
    let ok = false;
    try {
      ok = await copyText(await getText());
    } catch {
      ok = false;
    }
    clear(button);
    append(button, [icon(ok ? "check" : "warn"), ok ? "Copied" : "Could not copy"]);
    announce(ok ? announcement : "Could not copy.", !ok);
    window.setTimeout(rest, 1800);
  });
  return button;
}

/** The platform's modifier key as the user knows it: ⌘ on macOS, Ctrl elsewhere. */
export function modKey(): string {
  return /Mac|iPhone|iPad/.test(navigator.platform) ? "⌘" : "Ctrl";
}
