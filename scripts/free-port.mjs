// Frees the dev server port before launch. Tauri can orphan the Vite
// process when cargo crashes, leaving the port occupied and blocking
// the next `tauri dev` with "Port 9174 is already in use".
import { execSync } from "node:child_process";

const PORT = 9174;

function findListenerPids() {
  if (process.platform === "win32") {
    // No -p filter: vite may bind IPv6 (::1), which "-p tcp" omits
    const out = execSync("netstat -ano", { encoding: "utf8" });
    const pids = new Set();
    for (const line of out.split("\n")) {
      if (line.includes(`:${PORT} `) && line.includes("LISTENING")) {
        const pid = line.trim().split(/\s+/).pop();
        if (pid && pid !== "0") pids.add(pid);
      }
    }
    return [...pids];
  }
  try {
    const out = execSync(`lsof -ti tcp:${PORT} -s tcp:listen`, {
      encoding: "utf8",
    });
    return out.split("\n").filter(Boolean);
  } catch {
    return []; // lsof exits non-zero when nothing is listening
  }
}

function killPid(pid) {
  const cmd =
    process.platform === "win32" ? `taskkill /PID ${pid} /F` : `kill -9 ${pid}`;
  execSync(cmd, { stdio: "ignore" });
}

try {
  for (const pid of findListenerPids()) {
    killPid(pid);
    console.log(`[free-port] Killed stale process ${pid} on port ${PORT}`);
  }
} catch (error) {
  // Never block the dev server over cleanup; vite will report the port
  // conflict itself if one remains.
  console.warn(`[free-port] Cleanup skipped: ${error.message}`);
}
