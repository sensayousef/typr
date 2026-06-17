import { ipc } from "../ipc.ts";
import type { HistoryEntry } from "../types.ts";

function formatTime(timestamp: number): string {
  if (timestamp === 0) return "";
  const d = new Date(timestamp * 1000);
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function renderEntries(entries: HistoryEntry[], list: HTMLElement, empty: HTMLElement): void {
  list.innerHTML = "";
  if (entries.length === 0) {
    empty.classList.remove("hidden");
    return;
  }
  empty.classList.add("hidden");

  for (const entry of entries) {
    const item = document.createElement("div");
    item.className = "history-item";

    const meta = document.createElement("div");
    meta.className = "history-meta";

    const time = document.createElement("span");
    time.className = "history-time";
    time.textContent = formatTime(entry.timestamp);

    const engine = document.createElement("span");
    engine.className = "history-engine";
    engine.textContent = entry.engine === "cloud" ? "Groq" : "Local";

    meta.append(time, engine);

    const text = document.createElement("p");
    text.className = "history-text";
    text.textContent = entry.text;

    const copyBtn = document.createElement("button");
    copyBtn.className = "history-copy-btn";
    copyBtn.title = "Copy to clipboard";
    copyBtn.innerHTML = `<svg width="14" height="14" viewBox="0 0 16 16" fill="none">
      <rect x="5.5" y="5.5" width="8" height="9" rx="1.5" stroke="currentColor" stroke-width="1.2"/>
      <path d="M10.5 5.5V3.5A1.5 1.5 0 009 2H3.5A1.5 1.5 0 002 3.5V10A1.5 1.5 0 003.5 11.5H5.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
    </svg>`;

    copyBtn.addEventListener("click", async (e) => {
      e.stopPropagation();
      await navigator.clipboard.writeText(entry.text);
      copyBtn.classList.add("copied");
      setTimeout(() => copyBtn.classList.remove("copied"), 1200);
    });

    item.addEventListener("click", async () => {
      await navigator.clipboard.writeText(entry.text);
      item.classList.add("flash");
      setTimeout(() => item.classList.remove("flash"), 500);
    });

    item.append(meta, text, copyBtn);
    list.appendChild(item);
  }
}

export async function initHistoryPanel(): Promise<void> {
  const list = document.getElementById("history-list")!;
  const empty = document.getElementById("history-empty")!;
  const clearBtn = document.getElementById("history-clear-btn") as HTMLButtonElement;

  let entries = await ipc.getHistory();
  renderEntries(entries, list, empty);

  // Refresh list when history is updated (event fires after the entry is saved)
  await ipc.onHistoryUpdated(async () => {
    entries = await ipc.getHistory();
    renderEntries(entries, list, empty);
  });

  clearBtn.addEventListener("click", async () => {
    await ipc.clearHistory();
    entries = [];
    renderEntries(entries, list, empty);
  });
}
