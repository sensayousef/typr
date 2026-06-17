// Forever fix for the whisper-rs-sys / bindgen build on Windows.
//
// whisper-rs-sys runs bindgen against ggml.h at build time. On Windows that
// needs three things, all of which used to be hardcoded (with version numbers
// that rot on every toolchain update):
//   1. A libclang that bindgen 0.71 can actually parse with (clang >= 20 emits
//      a broken opaque `whisper_full_params`, so we cap the major version).
//   2. clang's own resource headers (stdbool.h, stddef.h, ...) — these live
//      inside the LLVM install, not the MSVC/SDK trees.
//   3. The Windows SDK CRT headers (ucrt/um/shared) so the system #includes in
//      ggml.h resolve.
// The MSVC toolset headers are deliberately NOT required: bindgen only parses
// the C header, and whisper.cpp's C++ compile finds MSVC through cmake itself.
//
// This module DISCOVERS all of that at run time — globbing the newest installed
// SDK and a compatible LLVM, and locating cmake via vswhere — then writes the
// result into src-tauri/.cargo/config.toml. Nothing here is version-pinned, so
// upgrading Visual Studio, the Windows SDK, or LLVM-18.x just works.

import { existsSync, readdirSync, readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import os from "node:os";

const HERE = dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = dirname(HERE);
const CARGO_CONFIG = join(PROJECT_ROOT, "src-tauri", ".cargo", "config.toml");

const PF = process.env["ProgramFiles"] || "C:\\Program Files";
const PF86 = process.env["ProgramFiles(x86)"] || "C:\\Program Files (x86)";
const HOME = os.homedir();
const APPDATA = process.env["APPDATA"] || join(HOME, "AppData", "Roaming");

// bindgen 0.71 mis-parses whisper_full_params (opaque `_address: u8`) when
// driven by libclang >= 20. Cap to the newest known-good major. Raise this once
// whisper-rs / bindgen support a newer clang.
const MAX_LIBCLANG_MAJOR = 19;

// Cargo target dir is redirected out of Downloads/ because Windows Application
// Control blocks freshly compiled build-script executables that run from there.
const TARGET_DIR = toFwd(join(APPDATA, "typr-build"));

function toFwd(p) {
  return p.replace(/\\/g, "/");
}

function listDirs(p) {
  try {
    return readdirSync(p, { withFileTypes: true })
      .filter((d) => d.isDirectory())
      .map((d) => d.name);
  } catch {
    return [];
  }
}

// Descending sort for dotted/dashed version folder names (e.g. 10.0.26100.0).
function byVersionDesc(a, b) {
  const pa = a.split(/[.\-]/).map((n) => parseInt(n, 10) || 0);
  const pb = b.split(/[.\-]/).map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const diff = (pb[i] || 0) - (pa[i] || 0);
    if (diff) return diff;
  }
  return 0;
}

// Find a libclang whose major version bindgen can handle AND that ships its
// resource headers (stdbool.h). Returns { libDir, resourceInclude, version }.
function findLibclang() {
  const candidates = [
    join(PF, "LLVM"),
    join(PF86, "LLVM"),
    ...listDirs(HOME)
      .filter((d) => /^(\.?)(llvm|libclang)/i.test(d))
      .map((d) => join(HOME, d)),
  ];

  let best = null;
  for (const root of candidates) {
    const dll = [join(root, "bin", "libclang.dll"), join(root, "libclang.dll")].find(existsSync);
    if (!dll) continue;
    const clangRoot = join(root, "lib", "clang");
    for (const ver of listDirs(clangRoot).sort(byVersionDesc)) {
      const major = parseInt(ver, 10);
      if (!Number.isFinite(major) || major > MAX_LIBCLANG_MAJOR) continue;
      const include = join(clangRoot, ver, "include");
      if (!existsSync(join(include, "stdbool.h"))) continue;
      if (!best || major > best.major) {
        best = { libDir: dirname(dll), resourceInclude: include, major, version: ver };
      }
    }
  }
  return best;
}

// Newest Windows 10/11 SDK whose CRT headers are actually present.
function findWindowsSdk() {
  let sdkRoot;
  try {
    const out = execFileSync(
      "reg",
      [
        "query",
        "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\Microsoft SDKs\\Windows\\v10.0",
        "/v",
        "InstallationFolder",
      ],
      { encoding: "utf8" },
    );
    const m = out.match(/InstallationFolder\s+REG_SZ\s+(.+)/);
    if (m) sdkRoot = m[1].trim();
  } catch {
    /* fall through to default */
  }
  if (!sdkRoot || !existsSync(sdkRoot)) sdkRoot = join(PF86, "Windows Kits", "10");

  const includeRoot = join(sdkRoot, "Include");
  const ver = listDirs(includeRoot)
    .sort(byVersionDesc)
    .find((v) => existsSync(join(includeRoot, v, "ucrt", "stdlib.h")));
  if (!ver) return null;

  return {
    version: ver,
    ucrt: join(includeRoot, ver, "ucrt"),
    um: join(includeRoot, ver, "um"),
    shared: join(includeRoot, ver, "shared"),
  };
}

// Enumerate every installed Visual Studio root. vswhere is authoritative but
// has been observed to miss side-by-side BuildTools installs, so we also glob
// both Program Files trees. De-duplicated.
function visualStudioRoots() {
  const roots = [];
  try {
    const vswhere = join(PF86, "Microsoft Visual Studio", "Installer", "vswhere.exe");
    if (existsSync(vswhere)) {
      const out = execFileSync(
        vswhere,
        ["-all", "-prerelease", "-products", "*", "-property", "installationPath"],
        { encoding: "utf8" },
      );
      roots.push(...out.split(/\r?\n/).filter(Boolean));
    }
  } catch {
    /* fall through to glob */
  }
  for (const base of [join(PF, "Microsoft Visual Studio"), join(PF86, "Microsoft Visual Studio")]) {
    for (const year of listDirs(base)) {
      for (const edition of listDirs(join(base, year))) {
        roots.push(join(base, year, edition));
      }
    }
  }
  return [...new Set(roots)];
}

// Newest MSVC toolset include dir that actually has its CRT headers. bindgen
// needs vcruntime.h (pulled in by the SDK's corecrt.h); some installs leave a
// stub MSVC dir with no headers, so we verify vcruntime.h before accepting it.
function findMsvcInclude() {
  let best = null;
  for (const root of visualStudioRoots()) {
    const toolsRoot = join(root, "VC", "Tools", "MSVC");
    for (const ver of listDirs(toolsRoot).sort(byVersionDesc)) {
      const include = join(toolsRoot, ver, "include");
      if (!existsSync(join(include, "vcruntime.h"))) continue;
      if (!best || byVersionDesc(ver, best.version) < 0) {
        best = { include, version: ver };
      }
    }
  }
  return best;
}

// Visual Studio ships a bundled cmake; whisper-rs-sys's cmake crate needs it.
function findCmake() {
  for (const root of visualStudioRoots()) {
    const cmake = join(
      root,
      "Common7", "IDE", "CommonExtensions", "Microsoft", "CMake", "CMake", "bin", "cmake.exe",
    );
    if (existsSync(cmake)) return toFwd(cmake);
  }
  return null; // cmake crate will fall back to PATH
}

// Resolve the full build environment, or throw an actionable error.
export function resolveBuildEnv() {
  const libclang = findLibclang();
  if (!libclang) {
    throw new Error(
      `No compatible libclang found (need a full LLVM install, major <= ${MAX_LIBCLANG_MAJOR}, ` +
        `with its lib/clang/<ver>/include/stdbool.h headers).\n` +
        `  Looked in: ${join(PF, "LLVM")}, ${join(PF86, "LLVM")}, and ${HOME}\\LLVM*\n` +
        `  Fix: install LLVM 18 (e.g. to ${HOME}\\LLVM-18) from https://github.com/llvm/llvm-project/releases`,
    );
  }

  const sdk = findWindowsSdk();
  if (!sdk) {
    throw new Error(
      `No Windows SDK found with ucrt headers.\n` +
        `  Fix: install the "Windows 11 SDK" component via the Visual Studio Installer.`,
    );
  }

  const msvc = findMsvcInclude();
  if (!msvc) {
    throw new Error(
      `No MSVC toolset headers found (need vcruntime.h under VC/Tools/MSVC/<ver>/include).\n` +
        `  Fix: install the "MSVC v143 C++ build tools" component via the Visual Studio Installer.`,
    );
  }

  const includes = [libclang.resourceInclude, msvc.include, sdk.ucrt, sdk.um, sdk.shared];
  // Forward slashes + double-quoted -I args: bindgen shlex-splits this string,
  // and forward slashes avoid backslash-escaping surprises in the parser.
  const bindgenArgs = includes.map((p) => `-I"${toFwd(p)}"`).join(" ");

  return {
    LIBCLANG_PATH: toFwd(libclang.libDir),
    BINDGEN_EXTRA_CLANG_ARGS: bindgenArgs,
    CMAKE: findCmake(), // may be null
    _detail: { libclang, sdk, msvc },
  };
}

// Render the .cargo/config.toml from discovered values. force = true so a stale
// or empty value inherited from a long-running parent process can never win.
function renderCargoConfig(env) {
  const lines = [
    "# AUTO-GENERATED by scripts/build-env.mjs on `npm run tauri`.",
    "# Do not hand-edit version paths here — they are re-discovered each build.",
    "",
    "[build]",
    `target-dir = "${TARGET_DIR}"`,
    "",
    "[env]",
    `LIBCLANG_PATH = { value = "${env.LIBCLANG_PATH}", force = true }`,
    // TOML literal (single-quoted) string keeps the embedded double quotes verbatim.
    `BINDGEN_EXTRA_CLANG_ARGS = { value = '${env.BINDGEN_EXTRA_CLANG_ARGS}', force = true }`,
  ];
  if (env.CMAKE) {
    lines.push(`CMAKE = { value = "${env.CMAKE}", force = true }`);
  }
  return lines.join("\n") + "\n";
}

// Write config only when it changed, so cargo doesn't see a config churn and
// needlessly rebuild.
export function writeCargoConfig(env) {
  const next = renderCargoConfig(env);
  let current = "";
  try {
    current = readFileSync(CARGO_CONFIG, "utf8");
  } catch {
    /* file may not exist yet */
  }
  if (current === next) return false;
  mkdirSync(dirname(CARGO_CONFIG), { recursive: true });
  writeFileSync(CARGO_CONFIG, next);
  return true;
}

// Apply discovered values to the current process (for the cargo child) and
// persist them to config (for direct cargo / rust-analyzer). Also scrub the
// poisonous WHISPER_DONT_GENERATE_BINDINGS so bindgen always runs.
export function applyBuildEnv(env = resolveBuildEnv()) {
  delete process.env.WHISPER_DONT_GENERATE_BINDINGS;
  process.env.LIBCLANG_PATH = env.LIBCLANG_PATH;
  process.env.BINDGEN_EXTRA_CLANG_ARGS = env.BINDGEN_EXTRA_CLANG_ARGS;
  if (env.CMAKE) process.env.CMAKE = env.CMAKE;
  const wrote = writeCargoConfig(env);
  return { env, wrote };
}

// `node scripts/build-env.mjs` prints what it discovered — handy after a
// toolchain upgrade to confirm the build will still resolve.
if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const env = resolveBuildEnv();
  console.log("Discovered build environment:");
  console.log("  libclang:   ", env._detail.libclang.libDir, `(clang ${env._detail.libclang.version})`);
  console.log("  msvc:       ", env._detail.msvc.version);
  console.log("  windows sdk:", env._detail.sdk.version);
  console.log("  cmake:      ", env.CMAKE ?? "(not found — relying on PATH)");
  console.log("  LIBCLANG_PATH =", env.LIBCLANG_PATH);
  console.log("  BINDGEN_EXTRA_CLANG_ARGS =", env.BINDGEN_EXTRA_CLANG_ARGS);
  const wrote = writeCargoConfig(env);
  console.log(wrote ? `\nWrote ${CARGO_CONFIG}` : `\n${CARGO_CONFIG} already up to date`);
}
