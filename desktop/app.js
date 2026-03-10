const invoke = window.__TAURI__.core.invoke;

const state = {
  workspacePath: "",
  entry: null,
};

const elements = {
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
};

function todayIso() {
  const now = new Date();
  const offset = now.getTimezoneOffset() * 60_000;
  return new Date(now.getTime() - offset).toISOString().slice(0, 10);
}

function defaultTodayItem() {
  return { task: "", checked: false, impact: "" };
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

function renderTodayItems() {
  elements.todayItems.innerHTML = "";
  state.entry.today.forEach((item, index) => {
    const row = document.createElement("div");
    row.className = `today-row${item.checked ? " done" : ""}`;

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

    const taskInput = document.createElement("textarea");
    taskInput.className = "today-input";
    taskInput.rows = 2;
    taskInput.placeholder = "今日やること / 完了した作業";
    taskInput.value = item.task;
    taskInput.addEventListener("input", () => {
      item.task = taskInput.value;
      refreshPreview();
    });

    const impactInput = document.createElement("textarea");
    impactInput.className = "impact-input";
    impactInput.rows = 2;
    impactInput.placeholder = "impact: どんな価値があったか";
    impactInput.value = item.impact;
    impactInput.disabled = !item.checked;
    impactInput.addEventListener("input", () => {
      item.impact = impactInput.value;
      refreshPreview();
    });

    const removeButton = document.createElement("button");
    removeButton.type = "button";
    removeButton.className = "remove-button";
    removeButton.textContent = "削除";
    removeButton.disabled = state.entry.today.length === 1;
    removeButton.addEventListener("click", () => {
      state.entry.today.splice(index, 1);
      if (state.entry.today.length === 0) {
        state.entry.today.push(defaultTodayItem());
      }
      renderTodayItems();
      refreshPreview();
    });

    row.append(checkboxWrap, taskInput, impactInput, removeButton);
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
    setStatus(`プレビュー生成に失敗しました: ${error}`, "error");
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
    setStatus(`新規日報として開始します: ${error}`, "error");
  }
}

async function saveEntry() {
  syncEntryFromInputs();
  setStatus("保存中です...");
  try {
    const result = await invoke("save_entry", { entry: state.entry });
    state.workspacePath = result.workspacePath;
    elements.workspacePath.textContent = result.workspacePath;
    state.entry.markdownPreview = result.markdown;
    elements.preview.textContent = result.markdown;
    await refreshGitStatus();
    setStatus(`保存しました: ${result.markdownPath}`, "success");
    return true;
  } catch (error) {
    setStatus(`保存に失敗しました: ${error}`, "error");
    return false;
  }
}

async function refreshGitStatus() {
  try {
    const result = await invoke("git_status");
    elements.gitStatusOutput.textContent = result.statusText;
  } catch (error) {
    elements.gitStatusOutput.textContent = `Git 状態の取得に失敗しました: ${error}`;
  }
}

async function saveAndPush() {
  const saved = await saveEntry();
  if (!saved || !state.entry) {
    return;
  }

  setStatus("commit と push を実行しています...");
  try {
    const result = await invoke("git_commit_and_push", {
      commitMessage:
        elements.commitMessage.value.trim() || defaultCommitMessage(state.entry.date),
    });
    elements.gitStatusOutput.textContent = result.statusText;
    setStatus(result.summary, "success");
  } catch (error) {
    setStatus(`push に失敗しました: ${error}`, "error");
    await refreshGitStatus();
  }
}

function bindEvents() {
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
}

window.addEventListener("DOMContentLoaded", async () => {
  bindEvents();
  const date = todayIso();
  elements.entryDate.value = date;
  await loadEntry(date);
});
