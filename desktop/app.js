const invoke = window.__TAURI__.core.invoke;

const state = {
  workspacePath: "",
  entry: null,
  sidebarCollapsed: false,
};

const elements = {
  shell: document.querySelector(".shell"),
  sidebar: document.querySelector("#sidebar"),
  entryDate: document.querySelector("#entryDate"),
  reloadButton: document.querySelector("#reloadButton"),
  saveButton: document.querySelector("#saveButton"),
  pushButton: document.querySelector("#pushButton"),
  gitStatusButton: document.querySelector("#gitStatusButton"),
  addTodayItemButton: document.querySelector("#addTodayItemButton"),
  todayItems: document.querySelector("#todayItems"),
  supportInput: document.querySelector("#supportInput"),
  improvementsInput: document.querySelector("#improvementsInput"),
  learningInput: document.querySelector("#learningInput"),
  notesInput: document.querySelector("#notesInput"),
  commitMessage: document.querySelector("#commitMessage"),
  gitStatusOutput: document.querySelector("#gitStatusOutput"),
  preview: document.querySelector("#preview"),
  status: document.querySelector("#status"),
  workspacePath: document.querySelector("#workspacePath"),
  sidebarToggleButton: document.querySelector("#sidebarToggleButton"),
  sidebarToggleIcon: document.querySelector(".sidebar-toggle-icon"),
  sidebarReopenButton: document.querySelector("#sidebarReopenButton"),
};

const SIDEBAR_KEY = "work-log:sidebar-collapsed";

function todayIso() {
  const now = new Date();
  const offset = now.getTimezoneOffset() * 60_000;
  return new Date(now.getTime() - offset).toISOString().slice(0, 10);
}

function defaultTodayItem() {
  return { task: "", checked: false, mustDo: false, impact: "" };
}

function emptyEntry(date) {
  return {
    date,
    today: [defaultTodayItem()],
    support: [],
    improvements: [],
    learning: [],
    notes: [],
    markdownPreview: `# ${date}\n`,
  };
}

function linesFromTextarea(value) {
  return value
    .split("\n")
    .map((item) => item.trim())
    .filter(Boolean);
}

function setStatus(message, kind = "") {
  elements.status.textContent = message;
  elements.status.className = `status${kind ? ` ${kind}` : ""}`;
}

function defaultCommitMessage(date) {
  return `chore: daily log ${date}`;
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

function renderTodayItems() {
  elements.todayItems.innerHTML = "";
  state.entry.today.forEach((item, index) => {
    const row = document.createElement("div");
    row.className = `today-row${item.checked ? " done" : ""}${item.mustDo ? " must-do" : ""}`;

    const checkboxWrap = document.createElement("label");
    checkboxWrap.className = "checkbox-wrap";
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = item.checked;
    checkbox.addEventListener("change", () => {
      item.checked = checkbox.checked;
      renderTodayItems();
      refreshPreview();
    });
    checkboxWrap.appendChild(checkbox);

    const mustDoButton = document.createElement("button");
    mustDoButton.type = "button";
    mustDoButton.className = `must-do-button${item.mustDo ? " active" : ""}`;
    mustDoButton.textContent = "必達";
    mustDoButton.setAttribute("aria-pressed", item.mustDo ? "true" : "false");
    mustDoButton.addEventListener("click", () => {
      item.mustDo = !item.mustDo;
      renderTodayItems();
      refreshPreview();
    });

    const taskInput = document.createElement("textarea");
    taskInput.className = "today-input";
    taskInput.rows = 2;
    taskInput.placeholder = "やること";
    taskInput.value = item.task;
    taskInput.addEventListener("input", () => {
      item.task = taskInput.value;
      refreshPreview();
    });

    const impactInput = document.createElement("textarea");
    impactInput.className = "impact-input";
    impactInput.rows = 2;
    impactInput.placeholder = "影響や価値";
    impactInput.value = item.impact;
    impactInput.disabled = !item.checked;
    impactInput.addEventListener("input", () => {
      item.impact = impactInput.value;
      refreshPreview();
    });

    const removeButton = document.createElement("button");
    removeButton.type = "button";
    removeButton.className = "remove-button icon-button";
    removeButton.textContent = "−";
    removeButton.setAttribute("aria-label", "項目を削除");
    removeButton.title = "項目を削除";
    removeButton.disabled = state.entry.today.length === 1;
    removeButton.addEventListener("click", () => {
      state.entry.today.splice(index, 1);
      if (state.entry.today.length === 0) {
        state.entry.today.push(defaultTodayItem());
      }
      renderTodayItems();
      refreshPreview();
    });

    row.append(checkboxWrap, mustDoButton, taskInput, impactInput, removeButton);
    elements.todayItems.appendChild(row);
  });
}

function syncEntryFromInputs() {
  state.entry.support = linesFromTextarea(elements.supportInput.value);
  state.entry.improvements = linesFromTextarea(elements.improvementsInput.value);
  state.entry.learning = linesFromTextarea(elements.learningInput.value);
  state.entry.notes = linesFromTextarea(elements.notesInput.value);
}

function syncInputsFromEntry() {
  elements.supportInput.value = state.entry.support.join("\n");
  elements.improvementsInput.value = state.entry.improvements.join("\n");
  elements.learningInput.value = state.entry.learning.join("\n");
  elements.notesInput.value = state.entry.notes.join("\n");
  renderTodayItems();
}

async function refreshPreview() {
  syncEntryFromInputs();
  try {
    const markdown = await invoke("render_markdown", { entry: state.entry });
    state.entry.markdownPreview = markdown;
    elements.preview.textContent = markdown;
  } catch (error) {
    setStatus(`プレビューを表示できませんでした: ${error}`, "error");
  }
}

async function loadEntry(date) {
  setStatus("読み込み中です...");
  try {
    const payload = await invoke("load_entry", { date });
    state.workspacePath = payload.workspacePath;
    state.entry = payload.entry || emptyEntry(date);
    elements.workspacePath.textContent = state.workspacePath;
    elements.entryDate.value = state.entry.date;
    elements.commitMessage.value = defaultCommitMessage(state.entry.date);
    syncInputsFromEntry();
    elements.preview.textContent = state.entry.markdownPreview || "";
    await refreshPreview();
    await refreshGitStatus();
    setStatus("読み込みました。", "success");
  } catch (error) {
    state.entry = emptyEntry(date);
    elements.commitMessage.value = defaultCommitMessage(date);
    syncInputsFromEntry();
    elements.preview.textContent = state.entry.markdownPreview;
    setStatus(`読み込めませんでした: ${error}`, "error");
  }
}

async function saveEntry() {
  syncEntryFromInputs();
  setStatus("保存しています...");
  try {
    const result = await invoke("save_entry", { entry: state.entry });
    state.workspacePath = result.workspacePath;
    elements.workspacePath.textContent = result.workspacePath;
    state.entry.markdownPreview = result.markdown;
    elements.preview.textContent = result.markdown;
    await refreshGitStatus();
    setStatus("保存しました。", "success");
    return true;
  } catch (error) {
    setStatus(`保存できませんでした: ${error}`, "error");
    return false;
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

  setStatus("反映しています...");
  try {
    const result = await invoke("git_commit_and_push", {
      commitMessage:
        elements.commitMessage.value.trim() || defaultCommitMessage(state.entry.date),
    });
    elements.gitStatusOutput.textContent = result.statusText;
    setStatus(result.summary, "success");
  } catch (error) {
    setStatus(`反映できませんでした: ${error}`, "error");
    await refreshGitStatus();
  }
}

function bindEvents() {
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
  elements.addTodayItemButton.addEventListener("click", () => {
    state.entry.today.push(defaultTodayItem());
    renderTodayItems();
  });
  [
    elements.supportInput,
    elements.improvementsInput,
    elements.learningInput,
    elements.notesInput,
  ].forEach((element) => {
    element.addEventListener("input", refreshPreview);
  });
  window.addEventListener("keydown", (event) => {
    if (event.metaKey && event.key === "\\") {
      event.preventDefault();
      toggleSidebar();
    }
  });
}

window.addEventListener("DOMContentLoaded", async () => {
  state.sidebarCollapsed = localStorage.getItem(SIDEBAR_KEY) === "1";
  applySidebarState();
  bindEvents();
  const date = todayIso();
  elements.entryDate.value = date;
  await loadEntry(date);
});
