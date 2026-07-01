// Launch the built, headless Robin app — no terminal attached.
//
// `npm run tauri dev` necessarily keeps a terminal open (it hosts the vite dev
// server + cargo watcher and streams build output). That's for development.
// For everyday use you want the *compiled* app, which is a GUI-subsystem binary
// and opens with no console window at all.
//
// This script finds that compiled binary and starts it fully detached, so even
// if you launch it from a shell the app outlives the shell and nothing stays
// tethered to a terminal. Build it first with `npm run app:build`.

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import os from "node:os";

const APPDATA = process.env["APPDATA"] || join(os.homedir(), "AppData", "Roaming");

// Mirrors TARGET_DIR in scripts/build-env.mjs (redirected out of Downloads/ for
// Windows Application Control), with the in-tree default as a fallback.
const candidates = [
  join(APPDATA, "robin-build", "release", "robin.exe"),
  join(APPDATA, "robin-build", "debug", "robin.exe"),
  join(process.cwd(), "src-tauri", "target", "release", "robin.exe"),
];

const exe = candidates.find(existsSync);

if (!exe) {
  console.error(
    "Built Robin app not found. Build it once with:\n  npm run app:build\n" +
      "then re-run `npm run app`.",
  );
  process.exit(1);
}

const child = spawn(exe, [], { detached: true, stdio: "ignore" });
child.unref();
console.log(`Launched ${exe} (detached, no terminal).`);
