const invoke = window.__TAURI__.core.invoke;

const state = {
  workspacePath: "",
  entry: null,
  blocks: [],
  sidebarCollapsed: false,
  autoCommitOnSave: false,
  autoPushOnSave: false,
  pendingFocus: null,
  zoomLevel: 1,
};

const elements = {
  shell: document.querySelector(".shell"),
  entryDate: document.querySelector("#entryDate"),
  reloadButton: document.querySelector("#reloadButton"),
  saveButton: document.querySelector("#saveButton"),
  pushButton: document.querySelector("#pushButton"),
  gitStatusButton: document.querySelector("#gitStatusButton"),
  status: document.querySelector("#status"),
  editor: document.querySelector("#editor"),
  workspacePath: document.querySelector("#workspacePath"),
  workspacePathInput: document.querySelector("#workspacePathInput"),
  workspaceBrowseButton: document.querySelector("#workspaceBrowseButton"),
  workspaceSaveButton: document.querySelector("#workspaceSaveButton"),
  autoCommitToggle: document.querySelector("#autoCommitToggle"),
  autoPushToggle: document.querySelector("#autoPushToggle"),
  sidebarToggleButton: document.querySelector("#sidebarToggleButton"),
  sidebarToggleIcon: document.querySelector(".sidebar-toggle-icon"),
  sidebarReopenButton: document.querySelector("#sidebarReopenButton"),
  commitMessage: document.querySelector("#commitMessage"),
  gitStatusOutput: document.querySelector("#gitStatusOutput"),
  zoomLabel: document.querySelector("#zoomLabel"),
  addLineButton: document.querySelector("#addLineButton"),
};

const SIDEBAR_KEY = "work-log:sidebar-collapsed";
const ZOOM_KEY = "work-log:zoom-level";
const HEADING_LEVELS = {
  heading1: 1,
  heading2: 2,
  heading3: 3,
};

function todayIso() {
  const now = new Date();
  const offset = now.getTimezoneOffset() * 60_000;
  return new Date(now.getTime() - offset).toISOString().slice(0, 10);
}

function createBlock(type = "paragraph", text = "", extras = {}) {
  return {
    id: crypto.randomUUID(),
    type,
    text,
    indent: extras.indent || 0,
    checked: extras.checked || "todo",
  };
}

function starterMarkdown(date) {
  return `# ${date}\n\n## ✅ 今日やること\n\n### 🚨 今日必達\n\n### 🐻 必達以外\n\n## 📝 メモ / 気づき\n\n## 🐕 保留\n`;
}

function emptyEntry(date) {
  return {
    date,
    markdownSource: starterMarkdown(date),
  };
}

function setStatus(message, kind = "") {
  elements.status.textContent = message;
  elements.status.className = `status${kind ? ` ${kind}` : ""}`;
}

function defaultCommitMessage(date) {
  return `chore: daily log ${date}`;
}

function normalizeMarkdown(text) {
  const normalized = String(text ?? "").replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  return normalized.endsWith("\n") ? normalized : `${normalized}\n`;
}

function parseMarkdownToBlocks(markdown) {
  const lines = normalizeMarkdown(markdown).split("\n");
  if (lines[lines.length - 1] === "") {
    lines.pop();
  }

  const blocks = [];
  lines.forEach((line) => {
    if (!line.trim()) {
      blocks.push(createBlock("paragraph", ""));
      return;
    }

    const heading = line.match(/^(#{1,3})\s+(.*)$/);
    if (heading) {
      blocks.push(
        createBlock(`heading${heading[1].length}`, heading[2], {
          indent: 0,
        }),
      );
      return;
    }

    const checkbox = line.match(/^(\s*)- \[( |x|X|\/)\]\s?(.*)$/);
    if (checkbox) {
      blocks.push(
        createBlock("checkbox", checkbox[3], {
          indent: Math.floor(checkbox[1].length / 2),
          checked:
            checkbox[2] === "/" ? "doing" : checkbox[2].toLowerCase() === "x" ? "done" : "todo",
        }),
      );
      return;
    }

    const bullet = line.match(/^(\s*)-\s(.*)$/);
    if (bullet) {
      blocks.push(
        createBlock("bullet", bullet[2], {
          indent: Math.floor(bullet[1].length / 2),
        }),
      );
      return;
    }

    blocks.push(createBlock("paragraph", line));
  });

  return blocks.length ? blocks : [createBlock("paragraph", "")];
}

function serializeBlocks(blocks) {
  const lines = blocks.map((block) => {
    const indent = "  ".repeat(block.indent || 0);
    if (block.type in HEADING_LEVELS) {
      return `${"#".repeat(HEADING_LEVELS[block.type])} ${block.text}`.trimEnd();
    }
    if (block.type === "checkbox") {
      const token =
        block.checked === "done" ? "x" : block.checked === "doing" ? "/" : " ";
      return `${indent}- [${token}] ${block.text}`.trimEnd();
    }
    if (block.type === "bullet") {
      return `${indent}- ${block.text}`.trimEnd();
    }
    return `${indent}${block.text}`.trimEnd();
  });
  return `${lines.join("\n")}\n`;
}

function syncMarkdownFromBlocks() {
  if (!state.entry) {
    return;
  }
  state.entry.markdownSource = serializeBlocks(state.blocks);
}

function setWorkspaceUi(
  configured,
  workspacePath = "",
  autoCommitOnSave = false,
  autoPushOnSave = false,
) {
  state.workspacePath = configured ? workspacePath : "";
  state.autoCommitOnSave = autoCommitOnSave;
  state.autoPushOnSave = autoPushOnSave;
  elements.workspacePath.textContent = configured ? workspacePath : "未設定";
  elements.workspacePathInput.value = workspacePath;
  elements.autoCommitToggle.checked = autoCommitOnSave;
  elements.autoPushToggle.checked = autoPushOnSave;
  [
    elements.entryDate,
    elements.reloadButton,
    elements.saveButton,
    elements.pushButton,
    elements.gitStatusButton,
    elements.commitMessage,
    elements.addLineButton,
  ].forEach((element) => {
    element.disabled = !configured;
  });
  elements.autoCommitToggle.disabled = !configured;
  elements.autoPushToggle.disabled = !configured;
}

function applySidebarState() {
  elements.shell.classList.toggle("sidebar-collapsed", state.sidebarCollapsed);
  elements.sidebarToggleButton.setAttribute("aria-expanded", String(!state.sidebarCollapsed));
  elements.sidebarToggleButton.setAttribute(
    "aria-label",
    state.sidebarCollapsed ? "サイドバーを開く" : "サイドバーを閉じる",
  );
  elements.sidebarToggleButton.title = state.sidebarCollapsed
    ? "サイドバーを開く"
    : "サイドバーを閉じる";
  elements.sidebarToggleIcon.textContent = state.sidebarCollapsed ? "▸" : "◂";
  elements.sidebarReopenButton.hidden = !state.sidebarCollapsed;
}

function setSidebarCollapsed(collapsed) {
  state.sidebarCollapsed = collapsed;
  localStorage.setItem(SIDEBAR_KEY, collapsed ? "1" : "0");
  applySidebarState();
}

function toggleSidebar() {
  setSidebarCollapsed(!state.sidebarCollapsed);
}

function applyZoom() {
  document.documentElement.style.zoom = String(state.zoomLevel);
  elements.zoomLabel.textContent = `${Math.round(state.zoomLevel * 100)}%`;
}

function setZoom(level) {
  state.zoomLevel = Math.max(0.7, Math.min(1.6, Number(level.toFixed(2))));
  localStorage.setItem(ZOOM_KEY, String(state.zoomLevel));
  applyZoom();
}

function setPendingFocus(id, caret = null) {
  state.pendingFocus = { id, caret };
}

function focusPendingBlock() {
  if (!state.pendingFocus) {
    return;
  }
  const target = elements.editor.querySelector(`[data-block-id="${state.pendingFocus.id}"]`);
  if (!target) {
    return;
  }
  target.focus();
  const caret = state.pendingFocus.caret ?? target.value.length;
  target.setSelectionRange(caret, caret);
  state.pendingFocus = null;
}

function autoResize(textarea) {
  textarea.style.height = "0px";
  textarea.style.height = `${Math.max(textarea.scrollHeight, 28)}px`;
}

function convertPrefix(block, value) {
  if (/^###\s+/.test(value)) {
    block.type = "heading3";
    block.text = value.replace(/^###\s+/, "");
    block.indent = 0;
    return true;
  }
  if (/^##\s+/.test(value)) {
    block.type = "heading2";
    block.text = value.replace(/^##\s+/, "");
    block.indent = 0;
    return true;
  }
  if (/^#\s+/.test(value)) {
    block.type = "heading1";
    block.text = value.replace(/^#\s+/, "");
    block.indent = 0;
    return true;
  }
  const checkbox = value.match(/^- \[( |x|X|\/)\]\s?(.*)$/);
  if (checkbox) {
    block.type = "checkbox";
    block.checked =
      checkbox[1] === "/" ? "doing" : checkbox[1].toLowerCase() === "x" ? "done" : "todo";
    block.text = checkbox[2];
    return true;
  }
  const bullet = value.match(/^- (.*)$/);
  if (bullet) {
    block.type = "bullet";
    block.text = bullet[1];
    return true;
  }
  return false;
}

function blockPlaceholder(block) {
  if (block.type === "heading1") {
    return "ページタイトル";
  }
  if (block.type === "heading2") {
    return "見出し";
  }
  if (block.type === "heading3") {
    return "小見出し";
  }
  if (block.type === "checkbox") {
    return "チェックリスト";
  }
  if (block.type === "bullet") {
    return "箇条書き";
  }
  return "空行に Markdown を書く";
}

function statusLabel(checked) {
  return checked === "done" ? "☑" : checked === "doing" ? "◪" : "☐";
}

function nextStatus(status) {
  return status === "todo" ? "doing" : status === "doing" ? "done" : "todo";
}

function renderEditor() {
  if (!state.entry) {
    elements.editor.innerHTML = "";
    return;
  }

  elements.editor.innerHTML = "";

  state.blocks.forEach((block, index) => {
    const row = document.createElement("div");
    row.className = `block-row type-${block.type}`;
    row.style.setProperty("--indent-level", block.indent || 0);

    const chrome = document.createElement("div");
    chrome.className = "block-chrome";

    if (block.type === "checkbox") {
      const checkbox = document.createElement("button");
      checkbox.type = "button";
      checkbox.className = `checkbox-toggle status-${block.checked}`;
      checkbox.textContent = statusLabel(block.checked);
      checkbox.disabled = !state.workspacePath;
      checkbox.addEventListener("click", () => {
        block.checked = nextStatus(block.checked);
        syncMarkdownFromBlocks();
        renderEditor();
      });
      chrome.appendChild(checkbox);
    } else if (block.type === "bullet") {
      const bullet = document.createElement("span");
      bullet.className = "bullet-mark";
      bullet.textContent = "•";
      chrome.appendChild(bullet);
    } else {
      const spacer = document.createElement("span");
      spacer.className = "chrome-spacer";
      chrome.appendChild(spacer);
    }

    const input = document.createElement("textarea");
    input.className = `block-input type-${block.type}`;
    input.rows = 1;
    input.value = block.text;
    input.placeholder = blockPlaceholder(block);
    input.disabled = !state.workspacePath;
    input.dataset.blockId = block.id;
    input.addEventListener("input", (event) => {
      const value = event.target.value;
      if (!convertPrefix(block, value)) {
        block.text = value;
      }
      syncMarkdownFromBlocks();
      renderEditor();
    });
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter" && !event.shiftKey) {
        event.preventDefault();
        const nextBlock = createBlock(
          block.type === "heading1" || block.type === "heading2" || block.type === "heading3"
            ? "paragraph"
            : block.type,
          "",
          {
            indent: block.indent || 0,
            checked: "todo",
          },
        );
        state.blocks.splice(index + 1, 0, nextBlock);
        syncMarkdownFromBlocks();
        setPendingFocus(nextBlock.id);
        renderEditor();
        return;
      }

      if (event.key === "Tab") {
        event.preventDefault();
        if (block.type === "heading1" || block.type === "heading2" || block.type === "heading3") {
          return;
        }
        block.indent = Math.max(0, (block.indent || 0) + (event.shiftKey ? -1 : 1));
        syncMarkdownFromBlocks();
        setPendingFocus(block.id);
        renderEditor();
        return;
      }

      if (event.key === "Backspace" && block.text === "" && state.blocks.length > 1) {
        event.preventDefault();
        state.blocks.splice(index, 1);
        syncMarkdownFromBlocks();
        setPendingFocus(state.blocks[Math.max(0, index - 1)].id);
        renderEditor();
      }
    });

    requestAnimationFrame(() => autoResize(input));

    const actions = document.createElement("div");
    actions.className = "block-actions";

    const removeButton = document.createElement("button");
    removeButton.type = "button";
    removeButton.className = "mini-icon danger";
    removeButton.textContent = "−";
    removeButton.title = "ブロックを削除";
    removeButton.disabled = !state.workspacePath;
    removeButton.addEventListener("click", () => {
      if (state.blocks.length === 1) {
        block.text = "";
        block.type = "paragraph";
        block.indent = 0;
        syncMarkdownFromBlocks();
        setPendingFocus(block.id);
        renderEditor();
        return;
      }
      state.blocks.splice(index, 1);
      syncMarkdownFromBlocks();
      setPendingFocus(state.blocks[Math.max(0, index - 1)].id);
      renderEditor();
    });
    actions.appendChild(removeButton);

    row.append(chrome, input, actions);
    elements.editor.appendChild(row);
  });

  requestAnimationFrame(focusPendingBlock);
}

async function loadEntry(date) {
  setStatus("読み込み中です...");
  try {
    const payload = await invoke("load_entry", { date });
    setWorkspaceUi(true, payload.workspacePath, state.autoCommitOnSave, state.autoPushOnSave);
    state.entry = payload.entry || emptyEntry(date);
    state.entry.markdownSource = normalizeMarkdown(state.entry.markdownSource);
    state.blocks = parseMarkdownToBlocks(state.entry.markdownSource);
    elements.entryDate.value = state.entry.date;
    elements.commitMessage.value = defaultCommitMessage(state.entry.date);
    renderEditor();
    await refreshGitStatus();
    setStatus("読み込みました。", "success");
  } catch (error) {
    state.entry = emptyEntry(date);
    state.blocks = parseMarkdownToBlocks(state.entry.markdownSource);
    elements.commitMessage.value = defaultCommitMessage(date);
    renderEditor();
    setStatus(`読み込めませんでした: ${error}`, "error");
  }
}

async function saveEntry() {
  if (!state.entry) {
    return false;
  }
  syncMarkdownFromBlocks();
  setStatus("保存しています...");
  try {
    const result = await invoke("save_entry", { entry: state.entry });
    setWorkspaceUi(true, result.workspacePath, state.autoCommitOnSave, state.autoPushOnSave);
    state.entry.markdownSource = normalizeMarkdown(result.markdown);
    state.blocks = parseMarkdownToBlocks(state.entry.markdownSource);
    renderEditor();
    if (state.autoCommitOnSave) {
      const syncResult = await invoke("git_commit_changes", {
        commitMessage:
          elements.commitMessage.value.trim() || defaultCommitMessage(state.entry.date),
        push: state.autoPushOnSave,
      });
      elements.gitStatusOutput.textContent = syncResult.statusText;
      setStatus(syncResult.summary, "success");
    } else {
      await refreshGitStatus();
      setStatus("保存しました。", "success");
    }
    return true;
  } catch (error) {
    setStatus(`保存できませんでした: ${error}`, "error");
    return false;
  }
}

async function loadWorkspaceSettings() {
  try {
    const result = await invoke("get_workspace_settings");
    setWorkspaceUi(
      result.configured,
      result.workspacePath,
      result.autoCommitOnSave,
      result.autoPushOnSave,
    );
    return result.configured;
  } catch (error) {
    setWorkspaceUi(false);
    setStatus(`保存先を読めませんでした: ${error}`, "error");
    return false;
  }
}

async function saveWorkspaceSettings() {
  setStatus("保存先を更新しています...");
  try {
    const result = await invoke("save_app_settings", {
      workspacePath: elements.workspacePathInput.value,
      autoCommitOnSave: elements.autoCommitToggle.checked,
      autoPushOnSave: elements.autoPushToggle.checked,
    });
    setWorkspaceUi(
      result.configured,
      result.workspacePath,
      result.autoCommitOnSave,
      result.autoPushOnSave,
    );
    await loadEntry(elements.entryDate.value || todayIso());
    setStatus("保存先を更新しました。", "success");
  } catch (error) {
    setStatus(`保存先を更新できませんでした: ${error}`, "error");
  }
}

async function browseWorkspacePath() {
  try {
    const selectedPath = await invoke("pick_workspace_path");
    if (selectedPath) {
      elements.workspacePathInput.value = selectedPath;
    }
  } catch (error) {
    if (String(error).includes("選択を取り消しました")) {
      return;
    }
    setStatus(`保存先を選べませんでした: ${error}`, "error");
  }
}

async function refreshGitStatus() {
  try {
    const result = await invoke("git_status");
    elements.gitStatusOutput.textContent = result.statusText;
  } catch (error) {
    elements.gitStatusOutput.textContent = `状態を表示できませんでした: ${error}`;
  }
}

async function saveAndPush() {
  const saved = await saveEntry();
  if (!saved || !state.entry) {
    return;
  }

  if (state.autoPushOnSave) {
    return;
  }

  setStatus("反映しています...");
  try {
    const result = await invoke("git_commit_changes", {
      commitMessage:
        elements.commitMessage.value.trim() || defaultCommitMessage(state.entry.date),
      push: true,
    });
    elements.gitStatusOutput.textContent = result.statusText;
    setStatus(result.summary, "success");
  } catch (error) {
    setStatus(`反映できませんでした: ${error}`, "error");
    await refreshGitStatus();
  }
}

function bindEvents() {
  elements.workspaceBrowseButton.addEventListener("click", browseWorkspacePath);
  elements.workspaceSaveButton.addEventListener("click", saveWorkspaceSettings);
  elements.autoCommitToggle.addEventListener("change", () => {
    if (!elements.autoCommitToggle.checked) {
      elements.autoPushToggle.checked = false;
    }
    saveWorkspaceSettings();
  });
  elements.autoPushToggle.addEventListener("change", () => {
    if (elements.autoPushToggle.checked) {
      elements.autoCommitToggle.checked = true;
    }
    saveWorkspaceSettings();
  });
  elements.sidebarToggleButton.addEventListener("click", toggleSidebar);
  elements.sidebarReopenButton.addEventListener("click", toggleSidebar);
  elements.entryDate.addEventListener("change", () => {
    loadEntry(elements.entryDate.value);
  });
  elements.reloadButton.addEventListener("click", () => {
    loadEntry(elements.entryDate.value);
  });
  elements.saveButton.addEventListener("click", saveEntry);
  elements.pushButton.addEventListener("click", saveAndPush);
  elements.gitStatusButton.addEventListener("click", refreshGitStatus);
  elements.addLineButton.addEventListener("click", () => {
    const block = createBlock("paragraph", "");
    state.blocks.push(block);
    syncMarkdownFromBlocks();
    setPendingFocus(block.id);
    renderEditor();
  });

  window.addEventListener("keydown", (event) => {
    const usesMeta = event.metaKey || event.ctrlKey;
    if (usesMeta && event.key === "\\") {
      event.preventDefault();
      toggleSidebar();
      return;
    }
    if (usesMeta && event.key === "-") {
      event.preventDefault();
      setZoom(state.zoomLevel - 0.1);
      return;
    }
    if (usesMeta && (event.key === "=" || event.key === "+")) {
      event.preventDefault();
      setZoom(state.zoomLevel + 0.1);
      return;
    }
    if (usesMeta && event.key === "0") {
      event.preventDefault();
      setZoom(1);
    }
  });
}

window.addEventListener("DOMContentLoaded", async () => {
  state.sidebarCollapsed = localStorage.getItem(SIDEBAR_KEY) === "1";
  state.zoomLevel = Number(localStorage.getItem(ZOOM_KEY) || "1");
  applySidebarState();
  applyZoom();
  bindEvents();
  const date = todayIso();
  elements.entryDate.value = date;
  const configured = await loadWorkspaceSettings();
  if (configured) {
    await loadEntry(date);
  } else {
    state.entry = emptyEntry(date);
    state.blocks = parseMarkdownToBlocks(state.entry.markdownSource);
    elements.commitMessage.value = defaultCommitMessage(date);
    renderEditor();
    setStatus("保存先を設定してください。", "error");
  }
});
