// Compiles src/*.ts to dist/ and copies the static files beside it. Works the same on macOS, Windows and Linux.
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const dist = join(here, "dist");

// Empty the folder but keep it: the Rust build reads it, and it must never be missing.
mkdirSync(dist, { recursive: true });
for (const entry of readdirSync(dist)) rmSync(join(dist, entry), { recursive: true, force: true });

const tsc = require.resolve("typescript/bin/tsc");
const run = spawnSync(process.execPath, [tsc, "-p", join(here, "tsconfig.json")], { stdio: "inherit" });
if (run.status !== 0) process.exit(run.status ?? 1);

for (const file of ["index.html", "styles.css"]) copyFileSync(join(here, file), join(dist, file));
copyFileSync(join(here, "..", "src-tauri", "icons", "128x128.png"), join(dist, "icon.png"));
console.log("built ui/dist");
