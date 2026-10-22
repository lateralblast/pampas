---
name: pampas
description: A critical edition of AI answers, with a merged reading, sources as witnesses tagged with boxed italic sigla, and a Not heard list.
colors:
  ground: "#f4f5f7"
  surface: "#ffffff"
  ink: "#1a1e24"
  ink-2: "#444c58"
  ink-3: "#5b6471"
  rule: "#dadee4"
  rule-strong: "#b4bbc6"
  ink-blue: "#1d4ed8"
  ink-blue-on: "#ffffff"
  reading-wash: "#e8eefc"
  ok: "#17683f"
  ok-wash: "#e4f3eb"
  warn: "#7a4f00"
  warn-wash: "#fbf1dc"
  bad: "#a8261d"
  bad-wash: "#fbe9e7"
  ground-dark: "#121519"
  surface-dark: "#191d23"
  ink-dark: "#e8eaee"
  ink-2-dark: "#b7bec9"
  ink-3-dark: "#9aa3af"
  rule-dark: "#2a3039"
  rule-strong-dark: "#3d4551"
  ink-blue-dark: "#8fb0ff"
  ink-blue-on-dark: "#0b1426"
  reading-wash-dark: "#1c2842"
  ok-dark: "#74d3a3"
  ok-wash-dark: "#15301f"
  warn-dark: "#e6b86a"
  warn-wash-dark: "#33280f"
  bad-dark: "#ff9d93"
  bad-wash-dark: "#3a1c1a"
typography:
  question:
    fontFamily: "ui-serif, Iowan Old Style, Palatino Linotype, Palatino, Charter, Georgia, Times New Roman, serif"
    fontSize: "1.5rem"
    fontWeight: 600
    lineHeight: 1.3
  answer:
    fontFamily: "ui-serif, Iowan Old Style, Palatino Linotype, Palatino, Charter, Georgia, Times New Roman, serif"
    fontSize: "1.0625rem"
    fontWeight: 400
    lineHeight: 1.6
  page-title:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "1.25rem"
    fontWeight: 650
    lineHeight: 1.25
  section-title:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "1.0625rem"
    fontWeight: 650
    lineHeight: 1.25
  body:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "0.9375rem"
    fontWeight: 400
    lineHeight: 1.5
  small:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 650
    lineHeight: 1.25
    letterSpacing: "0.06em"
  siglum:
    fontFamily: "ui-serif, Iowan Old Style, Palatino Linotype, Palatino, Charter, Georgia, Times New Roman, serif"
    fontSize: "0.82em"
    fontWeight: 600
    lineHeight: 1
  mono:
    fontFamily: "ui-monospace, SF Mono, Menlo, Consolas, Liberation Mono, monospace"
    fontSize: "0.875rem"
    fontWeight: 600
    lineHeight: 1.3
rounded:
  sm: "2px"
  md: "4px"
  full: "50%"
spacing:
  xs: "0.25rem"
  sm: "0.5rem"
  md: "1rem"
  lg: "1.5rem"
  gap: "2rem"
  gutter: "11.5rem"
  measure: "72ch"
components:
  button:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "0.3rem 0.8rem"
    height: "2rem"
  button-hover:
    backgroundColor: "{colors.reading-wash}"
  button-primary:
    backgroundColor: "{colors.ink-blue}"
    textColor: "{colors.ink-blue-on}"
    rounded: "{rounded.md}"
    padding: "0.3rem 0.8rem"
    height: "2.4rem"
  button-quiet:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    rounded: "{rounded.md}"
  button-danger:
    textColor: "{colors.bad}"
    rounded: "{rounded.md}"
  siglum:
    textColor: "{colors.ink-blue}"
    rounded: "{rounded.sm}"
    typography: "{typography.siglum}"
    height: "1.45em"
    padding: "0 0.3em"
  siglum-hover:
    backgroundColor: "{colors.ink-blue}"
    textColor: "{colors.ink-blue-on}"
  reading-row:
    backgroundColor: "{colors.reading-wash}"
    rounded: "{rounded.md}"
    padding: "1.1rem 0.85rem"
  input:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "0.4rem 0.6rem"
    height: "2.1rem"
  editor:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.md}"
    padding: "1rem 1.1rem 1.1rem"
  save-bar:
    backgroundColor: "{colors.warn-wash}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "0.6rem 0.9rem"
  notice-error:
    backgroundColor: "{colors.bad-wash}"
    rounded: "{rounded.md}"
    padding: "0.75rem 1rem"
---

# Design System: pampas

## Overview

**Creative North Star: "The Apparatus"**

pampas presents answers as a critical edition. The merged answer is the established reading, each AI service is a witness tagged with a boxed italic siglum (a, b, c), and services that did not answer are listed plainly under Not heard. It is a quiet, left-aligned reading document, not a chat thread and not a grid of equal cards. Chrome is system sans; answers and questions are system serif; the only decoration is hairline rules and the boxed letter.

The ground is a cool neutral that follows the OS light or dark theme. One ink-blue accent carries actions, selection, focus, links and sigla. Depth is flat: no shadows anywhere, no nested cards, structure comes from rules, one tinted wash, and a fixed two-column grid. State is always carried by an icon and a word as well as colour.

**Key Characteristics:**
- Two-column rows: an 11.5rem gutter (siglum, name, model, state) and a 72ch reading column.
- One tinted block per edition: the reading wash.
- Hairline 1px rules between rows; no shadows; 4px corners, 2px on the siglum.
- System font stacks only: serif for what is read, sans for chrome, mono for model ids and keys.
- Light and dark themes via `prefers-color-scheme`, same roles, lifted accent in dark.

## Colors

A cool grey-blue neutral ground with a single ink-blue accent and three muted status hues, each with its own wash.

### Primary
- **Ink Blue** (#1d4ed8; dark #8fb0ff): primary buttons, siglum letters and borders, links, the active-tab underline, focus ring, selection, text caret, the usage meter fill, and the READING row label. Text on it is `ink-blue-on` (#ffffff; dark #0b1426).
- **Reading Wash** (#e8eefc; dark #1c2842): the background of the reading row, and the hover fill of buttons.

### Neutral
- **Cool Ground** (#f4f5f7; dark #121519): page background and inset fills for code, table headers and the meter track base.
- **Page Surface** (#ffffff; dark #191d23): header, inputs, buttons, the editor, onboarding and empty panels.
- **Ink** (#1a1e24; dark #e8eaee): primary text.
- **Ink 2** (#444c58; dark #b7bec9): secondary text, ledes, inactive tabs.
- **Ink 3** (#5b6471; dark #9aa3af): metadata, hints, placeholders, model ids.
- **Hairline** (#dadee4; dark #2a3039): row dividers, header border, table borders.
- **Strong Hairline** (#b4bbc6; dark #3d4551): control borders, blockquote rule, step marks.

### Status
- **Answered Green** (#17683f; dark #74d3a3) with wash #e4f3eb (dark #15301f): "Answered in" state, completed steps, notices.
- **Pending Amber** (#7a4f00; dark #e6b86a) with wash #fbf1dc (dark #33280f): unsaved-changes dot, the save bar, warning notices.
- **Failure Red** (#a8261d; dark #ff9d93) with wash #fbe9e7 (dark #3a1c1a): not-heard states, danger buttons, errors, a full usage meter.

### Named Rules
**The One Blue Rule.** Ink blue is the only chromatic accent. Status hues appear only on state text, state icons and their washes.
**The Words Beside Colour Rule.** Every status colour is paired with an icon and a plain word (Answered in 0.2 s, Left out, Did not answer). Never colour alone.

## Typography

**Display/Reading Font:** system serif (`ui-serif`, Iowan Old Style, Palatino, Charter, Georgia)
**UI Font:** system sans (`system-ui`, Segoe UI, Roboto, Helvetica Neue)
**Mono Font:** system mono (`ui-monospace`, SF Mono, Menlo, Consolas)

**Character:** Editorial serif for everything a person reads or asks, plain native sans for controls. Numbers are tabular throughout. Fonts are the OS's own; nothing is bundled.

### Hierarchy
- **Question** (600, 1.5rem, 1.3, serif): the asked question heading above an edition, indented to the reading column.
- **Answer** (400, 1.0625rem, 1.6, serif): reading and witness text, and the prompt box (1.0625rem, 1.5).
- **Page title** (650, 1.25rem, 1.25, sans) and **Section title** (650, 1.0625rem): non-Ask pages and subsections.
- **Body** (400, 0.9375rem, 1.5, sans): default UI text. A **Small** step (0.875rem) serves hints, facts, state lines and tags.
- **Label** (650, 0.875rem, +0.06em, uppercase, Ink 2): row and section labels (READING, SOURCES, NOT HEARD) and field labels.
- **Siglum** (italic 600, 0.82em of its context, serif): see Components.
- **Mono** (0.875rem, 600): provider names on setup pages, model ids, keys.

Scale steps: 0.875, 0.9375, 1.0625, 1.25, 1.5rem. Reading measure 72ch; hints and ledes 68ch; onboarding step text 60ch.

### Named Rules
**The Serif Is For Reading Rule.** Serif is used for the question, answers, prompt box and history questions; sans for anything that acts or reports.

## Layout

A single centred column, `max-width` = gutter + gap + measure + 2.5rem, with 1.25rem side padding (0.75rem under 560px). Non-Ask pages cap at gutter + gap + measure. The core pattern is the row: a CSS grid of an 11.5rem gutter and a 72ch column with a 2rem column gap, 1.1rem vertical padding, a 1px hairline on top. Sources, keys and not-heard rows share it, so a siglum always sits in the same left place. Question and edition metadata are indented by gutter + gap to align with the reading column. Vertical rhythm uses 0.25, 0.5, 0.75, 1, 1.25, 1.5, 2rem.

The header is a sticky 3rem bar (Surface, bottom hairline) with brand left and text tabs; the selected tab has a 2px ink-blue underline and weight 600; unsaved tabs show an amber dot plus text. Sources are listed in a two-column list (20rem min) inside a rule-bordered disclosure. Editors use an auto-fit grid, 16rem minimum columns.

Responsive: at 860px and below, rows collapse to one column, the gutter becomes an inline wrap row (siglum, name, state side by side), gap 1rem, indents are removed, and the reading wash tightens to 0.5rem side padding. At 560px and below, the brand wordmark hides (logo stays), header padding shrinks, and the source list becomes one column. Tabs scroll horizontally.

## Elevation & Depth

Flat. There are no shadows. Depth comes from a one-step surface change (white or dark surface on the grey ground), 1px hairlines, and the single reading wash. Sticky elements (header, save bar) separate by border alone. The only motion is a 0.3s settle (4px rise from 35% opacity) when a witness arrives, a 0.15s chevron turn on disclosures, a pulsing ellipsis for pending, and all of it stops under `prefers-reduced-motion`.

### Named Rules
**The Flat Rule.** No box-shadow, no card inside a card. If something needs to stand apart, use a hairline or the surface step.

## Shapes

Restrained rectangles: 4px corners on buttons, inputs, panels, the reading wash and code blocks; 2px on sigla, meters and inline focus; fully round only on the numbered step marks and the amber dirty dot. Borders are 1px (control borders in Strong Hairline). Text-selection and focus both use ink blue; focus is a 2px outline at 2px offset.

## Components

### Siglum
The signature mark: a boxed italic serif letter (a, b, ... z, aa ...) assigned by config order so a source keeps its letter. Ink-blue text with a 1px currentColor border and 2px corners, min width and height 1.45em, centred, 0.3em side padding. In the gutter it sits at 1.05rem; inline in the reading it shrinks to 0.68em, raised slightly, as a button. Hover fills it with ink blue and swaps to the on-accent colour; activating it jumps to that witness's row. Sources named in the reading text are bolded and followed by their siglum.

### Rows, Gutter and the Reading Wash
A row is gutter plus body (see Layout). The gutter stacks siglum, the provider name (650), model id in Ink 3, then the state line. The reading row is the same grid on Reading Wash with 4px corners and 0.85rem side padding, pulled out by -0.85rem so its text still aligns; its label turns ink blue. Witness text over about 11.5rem is clamped with a fade mask and a Read the full answer button.

### State Marks
A small line under the gutter: icon plus words. Answered in X s is green; Left out and Did not answer are red; pending and skipped names use Ink 2 and a pulsing ellipsis. Not-heard rows give a plain-sentence cause with the fix beside it and a Technical detail disclosure.

### Buttons
Default is Surface with a Strong Hairline border, 4px corners, 2rem tall, 0.3rem 0.8rem padding; hover fills Reading Wash and the border turns blue. Primary is solid ink blue with white text (dark: pale blue with dark text), weight 600; Ask is 2.4rem tall, min 6.5rem; an idle primary reverts to a plain surface button. Quiet has no border or fill until hover. Danger has a red border and text. Link-like buttons are underlined ink-blue text. Disabled is 50% opacity, not-allowed. Icons are inline SVG at the text colour.

### Inputs and Editor Form
Inputs, selects and textareas: Surface, Strong Hairline, 4px, 2.1rem min height, placeholder Ink 3. The prompt box is serif at 1.0625rem, 5.5rem min. Fields stack with a 600 label above and 1rem below. Provider editing happens in an Editor panel: Surface, Strong Hairline border, 4px corners, 1rem/1.1rem padding, with an auto-fit field grid, advanced options in a disclosure, and Save and Cancel actions below. In a row it indents to the reading column. Key entry is an inline grid of a mono input, a select and a primary button.

### Tags
Small (0.875rem) Ink 2 inline text with an icon, in rows after names; `ok` is green and `bad` is red. Unframed, with no pill background.

### Save Bar
A sticky bar below the header (top 3.1rem) on Pending Amber wash with a 1px amber border, 4px corners, message left and actions right. It appears when edits are unsaved.

### Notices and Messages
Notices are Strong Hairline panels on amber wash (error: red border and wash). The top-of-page message strip is full width on red or green wash with a bottom hairline.

### Markdown Prose Block
Rendered answers use the serif Answer style with 0.85em paragraph spacing. Headings inside are sans 650 at 0.9375rem; lists indent 1.5em; inline code and pre use the Ground fill, hairline border, mono at 0.86em; blockquotes have a 1px Strong Hairline left rule and Ink 2 text; tables are hairline-bordered sans 0.875rem with Ground-filled header cells inside a horizontal scroll wrap; links underline with a Strong Hairline underline colour followed by the URL in mono.

### Usage Table and Meter
Full-width table with hairline row dividers, uppercase 0.875rem Ink 3 header, right-aligned tabular numbers. The meter is a 9rem by 0.5rem track on Hairline with an ink-blue fill; at its limit the fill is red and an adjacent word says so.

### Onboarding
A Surface panel with 4px corners listing numbered steps with hairline separators: a 1.75rem circular mark (number, or a green check when done), a 1.0625rem step title, and Ink 2 text at 60ch.

## Do's and Don'ts

### Do:
- **Do** keep every witness, key and not-heard row on the 11.5rem gutter and 72ch body grid.
- **Do** use the siglum for naming a source anywhere text refers to it, and keep its letter stable by config order.
- **Do** pair every status colour with an icon and words.
- **Do** use `var(--...)` tokens for both themes; give each new colour a light and a dark value.
- **Do** use system font stacks, serif for reading, sans for chrome, mono for ids and keys.
- **Do** keep depth to hairlines, the surface step and the single reading wash.

### Don't:
- **Don't** use shadows, nested cards or a chat-bubble thread.
- **Don't** add a second accent hue or use ink blue for decoration beyond actions, selection, focus, links, sigla and the meter.
- **Don't** hide a provider that failed; give it a Not heard row with a plain-sentence fix.
- **Don't** rely on colour alone to show state.
- **Don't** add new uppercase tracked labels above headings; the existing row labels mark edition parts and are not a pattern to extend.
