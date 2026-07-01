import { open, save } from "@tauri-apps/plugin-dialog";

import { ipc } from "../ipc.ts";

/** File extensions MarkItDown can convert, used for the open dialog filter. */
const SUPPORTED_EXTENSIONS = [
  "pdf", "docx", "pptx", "xlsx", "xls", "epub", "html", "htm",
  "csv", "json", "xml", "txt", "ipynb", "msg",
];

interface PanelEls {
  dropzone: HTMLButtonElement;
  selected: HTMLElement;
  url: HTMLInputElement;
  convert: HTMLButtonElement;
  result: HTMLElement;
  output: HTMLTextAreaElement;
  copy: HTMLButtonElement;
  saveBtn: HTMLButtonElement;
  status: HTMLElement;
}

/**
 * "To Markdown" page. Wraps the MarkItDown backend command: the user picks a
 * local file or pastes a URL, converts it to Markdown, then copies or saves the
 * result. Mirrors the behavior of the standalone MarkItDown GUI.
 */
export function initMarkitdownPanel(): void {
  const els = collectElements();
  if (!els) return;

  // Active conversion target. `isFile` only affects the default save filename.
  let currentInput = "";
  let isFile = false;

  const setStatus = (text: string): void => {
    els.status.textContent = text;
  };

  const setInput = (value: string, fromFile: boolean): void => {
    currentInput = value.trim();
    isFile = fromFile;
    els.convert.disabled = currentInput.length === 0;
  };

  els.dropzone.addEventListener("click", async () => {
    try {
      const picked = await open({
        multiple: false,
        directory: false,
        filters: [
          { name: "Supported documents", extensions: SUPPORTED_EXTENSIONS },
          { name: "All files", extensions: ["*"] },
        ],
      });
      if (typeof picked !== "string") return; // cancelled

      setInput(picked, true);
      els.url.value = "";
      els.selected.textContent = `✓  ${fileName(picked)}`;
      els.selected.classList.remove("hidden");
      setStatus("");
    } catch (err) {
      setStatus(`Could not open file picker: ${errorMessage(err)}`);
    }
  });

  els.url.addEventListener("input", () => {
    const value = els.url.value.trim();
    if (value.length > 0) {
      setInput(value, false);
      els.selected.classList.add("hidden");
    } else if (!isFile) {
      setInput("", false);
    }
  });

  els.convert.addEventListener("click", async () => {
    if (!currentInput) return;

    els.convert.disabled = true;
    els.convert.textContent = "Converting…";
    setStatus("Processing…");

    try {
      const markdown = await ipc.convertMarkitdown(currentInput);
      els.output.value = markdown;
      els.result.classList.remove("hidden");
      setStatus(markdown.trim().length > 0 ? "Done." : "No content extracted.");
    } catch (err) {
      setStatus(`Conversion failed: ${errorMessage(err)}`);
    } finally {
      els.convert.disabled = false;
      els.convert.textContent = "Convert";
    }
  });

  els.copy.addEventListener("click", async () => {
    const content = els.output.value;
    if (!content) return;
    try {
      await navigator.clipboard.writeText(content);
      setStatus("Copied to clipboard.");
    } catch {
      // Fallback for environments without clipboard API access.
      els.output.select();
      document.execCommand("copy");
      setStatus("Copied to clipboard.");
    }
  });

  els.saveBtn.addEventListener("click", async () => {
    const content = els.output.value;
    if (!content) return;
    try {
      const path = await save({
        defaultPath: defaultSaveName(currentInput, isFile),
        filters: [{ name: "Markdown", extensions: ["md"] }],
      });
      if (typeof path !== "string") return; // cancelled

      await ipc.saveMarkdown(path, content);
      setStatus(`Saved to ${path}`);
    } catch (err) {
      setStatus(`Could not save file: ${errorMessage(err)}`);
    }
  });
}

function collectElements(): PanelEls | null {
  const dropzone = document.getElementById("md-dropzone") as HTMLButtonElement | null;
  const selected = document.getElementById("md-selected");
  const url = document.getElementById("md-url") as HTMLInputElement | null;
  const convert = document.getElementById("md-convert") as HTMLButtonElement | null;
  const result = document.getElementById("md-result");
  const output = document.getElementById("md-output") as HTMLTextAreaElement | null;
  const copy = document.getElementById("md-copy") as HTMLButtonElement | null;
  const saveBtn = document.getElementById("md-save") as HTMLButtonElement | null;
  const status = document.getElementById("md-status");

  if (!dropzone || !selected || !url || !convert || !result || !output || !copy || !saveBtn || !status) {
    return null;
  }
  return { dropzone, selected, url, convert, result, output, copy, saveBtn, status };
}

function fileName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

function defaultSaveName(input: string, isFile: boolean): string {
  if (isFile) {
    const base = fileName(input).replace(/\.[^.]+$/, "");
    return `${base || "output"}.md`;
  }
  return "output.md";
}

function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return "Unexpected error";
}
