// The result of one question, laid out as a small critical edition: the reading (the merged answer)
// first, then each source as a witness tagged with its siglum, then the sources that were not heard.

import { api } from "./api.js";
import { append, announce, breakable, clear, copyButton, h, icon } from "./dom.js";
import { explainFailure, explainSkip } from "./explain.js";
import { clock, plural, seconds } from "./format.js";
import { renderMarkdown } from "./markdown.js";
import { assignSigla, expandRefs, markNames } from "./siglum.js";
import type { TabId } from "./store.js";
import type { Answer, AskResponse, ExportFormat, ExportReport } from "./types.js";

export interface WitnessAnswer {
  ok: boolean;
  text: string;
  /** Milliseconds from asking to this answer arriving. */
  ms: number;
}

export interface EditionData {
  id: string;
  prompt: string;
  askedAt: number;
  finishedAt: number | null;
  /** All source names in config order, which fixes each one's siglum. */
  order: string[];
  models: Record<string, string | null>;
  mergeRequested: boolean;
  phase: "asking" | "merging" | "done" | "error";
  /** Sources actually asked, in config order. */
  asked: string[];
  answers: Record<string, WitnessAnswer>;
  skipped: { provider: string; raw: string }[];
  notices: string[];
  merged: { provider: string; text: string } | null;
  mergeError: string | null;
  fatal: string | null;
}

export function newEdition(
  prompt: string,
  order: string[],
  models: Record<string, string | null>,
  mergeRequested: boolean,
  now = Date.now(),
): EditionData {
  return {
    id: `${now}-${Math.random().toString(36).slice(2, 8)}`,
    prompt,
    askedAt: now,
    finishedAt: null,
    order,
    models,
    mergeRequested,
    phase: "asking",
    asked: [],
    answers: {},
    skipped: [],
    notices: [],
    merged: null,
    mergeError: null,
    fatal: null,
  };
}

function sortByOrder(names: string[], order: string[]): string[] {
  return [...names].sort((a, b) => order.indexOf(a) - order.indexOf(b));
}

function takeWarnings(data: EditionData, warnings: string[]): void {
  for (const w of warnings) {
    const skip = explainSkip(w);
    if (skip) {
      if (!data.skipped.some((s) => s.provider === skip.provider)) data.skipped.push({ provider: skip.provider, raw: w });
    } else if (!data.notices.includes(w)) data.notices.push(w);
  }
}

export function applyStarted(data: EditionData, providers: string[], warnings: string[]): void {
  data.asked = sortByOrder(providers, data.order);
  takeWarnings(data, warnings);
}

export function applyAnswer(data: EditionData, answer: Answer, now = Date.now()): void {
  data.answers[answer.provider] = { ok: answer.ok, text: answer.text, ms: now - data.askedAt };
  if (data.mergeRequested && data.asked.length && data.asked.every((n) => data.answers[n])) data.phase = "merging";
}

export function applyResponse(data: EditionData, response: AskResponse, now = Date.now()): void {
  // The response carries every answer, so one that was missed as an event is still shown.
  for (const a of response.answers) {
    if (!data.answers[a.provider]) data.answers[a.provider] = { ok: a.ok, text: a.text, ms: now - data.askedAt };
  }
  takeWarnings(data, response.warnings);
  data.merged = response.merged;
  data.mergeError = response.merge_error;
  data.phase = "done";
  data.finishedAt = now;
}

export function applyFailure(data: EditionData, message: string, now = Date.now()): void {
  data.fatal = message;
  data.phase = "error";
  data.finishedAt = now;
}

export function answeredCount(data: EditionData): number {
  return data.asked.filter((n) => data.answers[n]?.ok).length;
}

/** The edition in the shape the backend renders for copying and saving. */
export function toReport(data: EditionData): ExportReport {
  return {
    prompt: data.prompt,
    answers: data.asked.flatMap((name) => {
      const a = data.answers[name];
      return a ? [{ provider: name, ok: a.ok, text: a.text }] : [];
    }),
    merged: data.merged,
    merge_error: data.mergeError,
  };
}

/** The edition as markdown, used when the backend cannot be asked. */
export function toMarkdown(data: EditionData): string {
  const out = [`# ${data.prompt.trim()}`, ""];
  if (data.merged) out.push(`## Reading (merged by ${data.merged.provider})`, "", data.merged.text.trim(), "");
  for (const name of data.asked) {
    const a = data.answers[name];
    if (a) out.push(`## ${name}${a.ok ? "" : " (did not answer)"}`, "", a.text.trim(), "");
  }
  return out.join("\n").trimEnd() + "\n";
}

export interface EditionContext {
  expanded: Set<string>;
  /** Called when a row is opened or closed; the caller re-renders. */
  onToggle(name: string): void;
  /** Open a source's answer and move to it (from a siglum mark in the reading). */
  onReveal(name: string): void;
  onNavigate(tab: TabId): void;
  /** Tell the user something happened (a file was saved) or went wrong. */
  onMessage(kind: "notice" | "error", text: string): void;
  /** Present on a saved edition. */
  onAskAgain?: () => void;
  onExpandAll?: (expand: boolean) => void;
  saved: boolean;
}

/** A row of the edition. `sig` changes whenever what the row shows changes, so unchanged rows can be left alone. */
function row(kind: string, gutter: (Node | string | false | null)[], body: (Node | string | false | null)[], key: string, sig: string): HTMLElement {
  return h(
    "div",
    { class: `row ${kind}`, role: "listitem", dataset: { key, sig } },
    h("div", { class: "gutter" }, ...gutter),
    h("div", { class: "body" }, ...body),
  );
}

function fixButton(fix: { label: string; tab: TabId } | undefined, ctx: EditionContext): HTMLElement | null {
  return fix ? h("button", { class: "linklike", type: "button", onclick: () => ctx.onNavigate(fix.tab) }, fix.label) : null;
}

function plainBlock(raw: string, kind: "failure" | "skip", ctx: EditionContext): HTMLElement {
  const plain = kind === "failure" ? explainFailure(raw) : (explainSkip(raw)?.plain ?? { summary: "It was left out.", detail: raw });
  return h(
    "div",
    { class: "plain" },
    h("p", null, plain.summary, " ", fixButton(plain.fix, ctx)),
    plain.detail && plain.detail !== plain.summary
      ? h("details", { class: "technical" }, h("summary", null, "Technical detail"), h("pre", null, plain.detail))
      : null,
  );
}

function copyRow(text: string): HTMLElement {
  return h("div", { class: "row-actions" }, copyButton("Copy the reading", "Copied the reading.", () => text));
}

const SAVE_FORMATS: { format: ExportFormat; label: string }[] = [
  { format: "markdown", label: "Markdown (.md)" },
  { format: "text", label: "Plain text (.txt)" },
  { format: "json", label: "JSON (.json)" },
];

/** "Save as…": a small menu of formats; the system's save dialog does the rest. */
function saveMenu(data: EditionData, ctx: EditionContext): HTMLElement {
  const menu = h("details", { class: "menu" });
  const close = () => menu.removeAttribute("open");
  menu.append(
    h("summary", { class: "button-like" }, icon("down"), "Save as…"),
    h(
      "div",
      { class: "menu-list", role: "group", "aria-label": "Save the answers as" },
      ...SAVE_FORMATS.map(({ format, label }) =>
        h(
          "button",
          {
            type: "button",
            class: "quiet",
            onclick: async () => {
              close();
              try {
                const path = await api.saveEdition(toReport(data), format);
                if (path) {
                  announce(`Saved to ${path}.`);
                  ctx.onMessage("notice", `Saved to ${path}`);
                }
              } catch (e) {
                ctx.onMessage("error", `Could not save: ${e instanceof Error ? e.message : String(e)}`);
              }
            },
          },
          label,
        ),
      ),
    ),
  );
  menu.addEventListener("keydown", (ev) => {
    if (ev.key === "Escape") {
      close();
      menu.querySelector("summary")?.focus();
    }
  });
  menu.addEventListener("focusout", (ev) => {
    if (!menu.contains(ev.relatedTarget as Node | null)) close();
  });
  return menu;
}

export function renderEdition(data: EditionData, ctx: EditionContext): HTMLElement {
  const sigla = assignSigla(data.order);
  const sig = (name: string, large = false) =>
    h("span", { class: large ? "sig sig-lg" : "sig", "aria-hidden": "true" }, sigla.get(name) ?? "?");
  const root = h("section", { class: "edition", "aria-labelledby": `ed-${data.id}`, dataset: { id: data.id } });

  // Header: the question, then a line of facts and the actions.
  const total = data.asked.length;
  const heard = answeredCount(data);
  const facts: string[] = [`Asked ${clock(data.askedAt)}`];
  if (data.phase === "asking") facts.push(total ? `${data.asked.filter((n) => data.answers[n]).length} of ${total} answered so far` : "starting");
  else if (data.phase === "merging") facts.push(`${heard} of ${total} answered`, "writing the reading");
  else if (data.phase === "done") {
    facts.push(`${heard} of ${plural(total, "source")} answered`);
    if (data.finishedAt) facts.push(seconds(data.finishedAt - data.askedAt));
  }
  const hasLong = data.asked.some((n) => data.answers[n]?.ok);
  const head = h(
    "header",
    { class: "edition-head", dataset: { key: "head", sig: `${facts.join("|")}|${ctx.saved}|${ctx.expanded.size === 0}|${hasLong}` } },
    h("h2", { id: `ed-${data.id}`, class: "question", tabindex: -1 }, data.prompt),
    h(
      "div",
      { class: "edition-meta" },
      h("p", { class: "facts" }, facts.join(" · ")),
      h(
        "div",
        { class: "actions" },
        ctx.saved && ctx.onAskAgain ? h("button", { type: "button", onclick: ctx.onAskAgain }, "Ask again") : null,
        hasLong && ctx.onExpandAll
          ? h("button", { type: "button", onclick: () => ctx.onExpandAll?.(ctx.expanded.size === 0) }, ctx.expanded.size === 0 ? "Open all answers" : "Close all answers")
          : null,
        copyButton("Copy everything", "Copied to the clipboard.", async () => {
          try {
            return await api.renderEdition(toReport(data), "markdown");
          } catch {
            return toMarkdown(data);
          }
        }, ""),
        saveMenu(data, ctx),
      ),
    ),
  );
  root.append(head);

  if (data.fatal) {
    root.append(h("div", { class: "notice error", role: "alert", dataset: { key: "fatal", sig: data.fatal } }, h("p", null, data.fatal)));
  }

  // The reading.
  if (data.mergeRequested) {
    const gutter: (Node | string | false | null)[] = [h("h3", { class: "label" }, "Reading")];
    let body: (Node | string)[];
    if (data.merged) {
      gutter.push(h("p", { class: "gmeta" }, "merged by ", h("span", { class: "nowrap" }, data.merged.provider, " ", sig(data.merged.provider))));
      const html = expandRefs(renderMarkdown(markNames(data.merged.text, sigla)), sigla);
      const text = h("div", { class: "text prose reading-text", innerHTML: html });
      // The boxed letters in the reading are buttons: each one takes you to that source's answer.
      text.addEventListener("click", (ev) => {
        const target = (ev.target as Element).closest<HTMLElement>("[data-ref]");
        if (target?.dataset.ref) ctx.onReveal(target.dataset.ref);
      });
      body = [text, h("p", { class: "legend" }, "Select a boxed letter to jump to that source's answer."), copyRow(data.merged.text)];
    } else if (data.phase === "asking") {
      body = [h("p", { class: "muted" }, "The reading is written once the answers are in.")];
    } else if (data.phase === "merging") {
      body = [h("p", { class: "muted pending-dots" }, "Writing the reading")];
    } else if (data.mergeError) {
      body = [h("div", { class: "plain" }, h("p", null, "The answers could not be merged. The separate answers are below."), h("details", { class: "technical" }, h("summary", null, "Technical detail"), h("pre", null, data.mergeError)))];
    } else if (data.notices.some((n) => n.includes("nothing to merge"))) {
      body = [h("p", { class: "muted" }, heard === 1 ? "Only one source answered, so there is nothing to merge." : "No source answered, so there is nothing to merge.")];
    } else {
      body = [h("p", { class: "muted" }, "There is no merged reading for this question.")];
    }
    const readingRow = h("div", { class: "row reading", dataset: { key: "reading", sig: `${data.phase}|${data.merged?.text.length ?? 0}|${data.mergeError ?? ""}|${heard}` } }, h("div", { class: "gutter" }, ...gutter), h("div", { class: "body" }, ...body));
    root.append(readingRow);
  }

  // Sources still waiting or answered, in config order.
  const answered = data.asked.filter((n) => data.answers[n]?.ok || !data.answers[n]);
  if (answered.length) {
    const group = h("div", { class: "witnesses", role: "list", "aria-label": "Answers from each source", dataset: { key: "sources", group: "" } });
    for (const name of answered) {
      const a = data.answers[name];
      const model = data.models[name];
      const open = ctx.expanded.has(name);
      const status = !a
        ? h("p", { class: "wstate" }, icon("clock"), "Waiting ", h("span", { class: "elapsed", dataset: { since: String(data.askedAt) } }))
        : h("p", { class: "wstate ok" }, icon("check"), `Answered in ${seconds(a.ms)}`);
      const body: (Node | string | null)[] = !a
        ? [h("p", { class: "muted pending-dots" }, "Waiting for an answer")]
        : [
            h("div", { class: `text prose clamp${open ? " open" : ""}`, id: `ans-${data.id}-${name}`, innerHTML: renderMarkdown(a.text) }),
            h(
              "div",
              { class: "row-actions" },
              h("button", { type: "button", class: "toggle", hidden: !open, "aria-expanded": String(open), "aria-controls": `ans-${data.id}-${name}`, dataset: { focusKey: `toggle-${name}` }, onclick: () => ctx.onToggle(name) }, open ? "Show less" : "Read the full answer"),
              copyButton("Copy", `Copied the answer from ${name}.`, () => a.text),
            ),
          ];
      group.append(row(`witness ${a ? "ok" : "pending"}`, [sig(name, true), h("h3", { class: "wname", tabindex: -1 }, name), model ? h("p", { class: "gmeta" }, breakable(model)) : null, status], body, name, `${a ? "ok" : "pending"}|${a?.text.length ?? 0}|${a?.ms ?? 0}|${open}`));
    }
    root.append(h("h3", { class: "section-label", dataset: { key: "label-sources", sig: "1" } }, "Sources"), group);
  }

  // Not heard: sources that failed after being asked, and those left out before asking.
  const failed = data.asked.filter((n) => data.answers[n] && !data.answers[n]?.ok);
  const notHeard = sortByOrder([...failed, ...data.skipped.map((x) => x.provider)], data.order);
  if (notHeard.length) {
    const group = h("div", { class: "witnesses", role: "list", "aria-label": "Sources that were not heard", dataset: { key: "notheard", group: "" } });
    for (const name of notHeard) {
      const a = data.answers[name];
      const skip = data.skipped.find((x) => x.provider === name);
      const model = data.models[name];
      const left = !a && skip;
      const block = a ? plainBlock(a.text, "failure", ctx) : plainBlock(skip?.raw ?? "", "skip", ctx);
      group.append(
        row(
          `witness ${left ? "skipped" : "failed"}`,
          [sig(name, true), h("h3", { class: "wname", tabindex: -1 }, name), model ? h("p", { class: "gmeta" }, breakable(model)) : null, h("p", { class: "wstate bad" }, icon("warn"), left ? "Left out" : "Did not answer")],
          [block],
          left ? `skip-${name}` : name,
          `${left ? "skip" : "fail"}|${(a?.text ?? skip?.raw ?? "").length}`,
        ),
      );
    }
    root.append(h("h3", { class: "section-label", dataset: { key: "label-notheard", sig: "1" } }, "Not heard"), group);
  }

  const other = data.notices.filter((n) => !n.includes("nothing to merge"));
  if (other.length) root.append(h("div", { class: "notice", dataset: { key: "notices", sig: other.join("|") } }, ...other.map((n) => h("p", null, n))));
  return root;
}

/**
 * Show the "Read the full answer" button on the rows whose answer is taller than its collapsed box.
 * Called after the edition is in the page, when heights can be measured.
 */
export function measureClamps(root: HTMLElement): void {
  for (const text of root.querySelectorAll<HTMLElement>(".text.clamp:not(.open)")) {
    const toggle = text.parentElement?.querySelector<HTMLButtonElement>(".toggle");
    const clipped = text.scrollHeight > text.clientHeight + 4;
    text.classList.toggle("clipped", clipped);
    if (toggle) toggle.hidden = !clipped;
  }
}

/** Keep the "Waiting 12 s" counters moving without re-rendering. */
export function tickElapsed(root: HTMLElement, now = Date.now()): void {
  for (const el of root.querySelectorAll<HTMLElement>(".elapsed")) {
    const since = Number(el.dataset.since);
    if (since) el.textContent = `${Math.max(0, Math.round((now - since) / 1000))} s`;
  }
}

/** Replace `container`'s content with `next`, keeping keyboard focus on the same control. */
export function swapKeepingFocus(container: HTMLElement, next: HTMLElement): void {
  const active = document.activeElement as HTMLElement | null;
  const key = active && container.contains(active) ? active.dataset.focusKey : undefined;
  clear(container);
  append(container, [next]);
  if (key) container.querySelector<HTMLElement>(`[data-focus-key="${CSS.escape(key)}"]`)?.focus();
}

/**
 * Bring the page's edition up to date with `next` by changing only what differs. A row that shows the
 * same thing as before is left exactly as it is, so an opened "Technical detail", a text selection, and
 * the scroll position survive while slower sources arrive; a row that has just received its answer
 * settles in with a short motion.
 */
export function updateEdition(container: HTMLElement, next: HTMLElement): void {
  const current = container.firstElementChild as HTMLElement | null;
  if (!current || current.dataset.id !== next.dataset.id) {
    swapKeepingFocus(container, next);
    return;
  }
  const active = document.activeElement as HTMLElement | null;
  const focusKey = active && container.contains(active) ? active.dataset.focusKey : undefined;
  patchChildren(current, next);
  if (focusKey && document.activeElement !== active) container.querySelector<HTMLElement>(`[data-focus-key="${CSS.escape(focusKey)}"]`)?.focus();
}

function patchChildren(oldParent: Element, newParent: Element): void {
  const oldByKey = new Map<string, HTMLElement>();
  for (const c of Array.from(oldParent.children) as HTMLElement[]) if (c.dataset.key) oldByKey.set(c.dataset.key, c);
  const result: Element[] = [];
  for (const n of Array.from(newParent.children) as HTMLElement[]) {
    const old = n.dataset.key ? oldByKey.get(n.dataset.key) : undefined;
    if (old && n.hasAttribute("data-group")) {
      patchChildren(old, n);
      result.push(old);
    } else if (old && old.dataset.sig === n.dataset.sig) {
      result.push(old);
    } else {
      if (old?.classList.contains("pending") && !n.classList.contains("pending")) n.classList.add("arrive");
      if (old?.classList.contains("reading") && n.classList.contains("reading") && old.dataset.sig !== n.dataset.sig) n.classList.add("arrive");
      result.push(n);
    }
  }
  result.forEach((el, i) => {
    if (oldParent.children[i] !== el) oldParent.insertBefore(el, oldParent.children[i] ?? null);
  });
  for (const extra of Array.from(oldParent.children).slice(result.length)) extra.remove();
}
