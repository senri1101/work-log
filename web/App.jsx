import { useEffect, useRef, useState } from "react";
import { BlockNoteView } from "@blocknote/mantine";
import { useCreateBlockNote } from "@blocknote/react";

const invoke = window.__TAURI__.core.invoke;
const SIDEBAR_KEY = "work-log:sidebar-collapsed";
const ZOOM_KEY = "work-log:zoom-level";

function todayIso() {
  const now = new Date();
  const offset = now.getTimezoneOffset() * 60_000;
  return new Date(now.getTime() - offset).toISOString().slice(0, 10);
}

function starterMarkdown(date) {
  return `# ${date}

## ✅ 今日やること

### 🚨 今日必達

### 🐻 必達以外

## 📝 メモ / 気づき

## 🐕 保留
`;
}

function emptyEntry(date) {
  return {
    date,
    markdownSource: starterMarkdown(date),
  };
}

function defaultCommitMessage(date) {
  return `chore: daily log ${date}`;
}

function normalizeMarkdown(text) {
  const normalized = String(text ?? "").replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  return normalized.endsWith("\n") ? normalized : `${normalized}\n`;
}

function looksStructuredPaste(text) {
  const normalized = String(text ?? "").trim();
  if (!normalized) {
    return false;
  }
  if (normalized.includes("\n")) {
    return true;
  }
  return /^([#>*-]|\d+\.)\s|^- \[[ x/]\]\s/i.test(normalized);
}

function normalizeInlinePaste(text) {
  return String(text ?? "")
    .replace(/\r\n/g, "\n")
    .replace(/\r/g, "\n")
    .replace(/\n+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function App() {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(
    localStorage.getItem(SIDEBAR_KEY) === "1",
  );
  const [zoomLevel, setZoomLevelState] = useState(Number(localStorage.getItem(ZOOM_KEY) || "1"));
  const [workspacePath, setWorkspacePath] = useState("");
  const [autoCommitOnSave, setAutoCommitOnSave] = useState(false);
  const [autoPushOnSave, setAutoPushOnSave] = useState(false);
  const [status, setStatus] = useState({ message: "", kind: "" });
  const [entry, setEntry] = useState(null);
  const [entryDate, setEntryDate] = useState(todayIso());
  const [commitMessage, setCommitMessage] = useState(defaultCommitMessage(todayIso()));
  const [gitStatus, setGitStatus] = useState("");
  const [workspaceInput, setWorkspaceInput] = useState("");
  const [editorEpoch, setEditorEpoch] = useState(0);
  const [showMetaPanel, setShowMetaPanel] = useState(false);
  const hydratingRef = useRef(false);

  const editor = useCreateBlockNote(
    {
      pasteHandler: ({ event, editor: currentEditor, defaultPasteHandler }) => {
        const plainText = event.clipboardData?.getData("text/plain") ?? "";
        const currentBlock = currentEditor.getTextCursorPosition().block;
        const inlineText = normalizeInlinePaste(plainText);

        if (currentBlock.type === "checkListItem" && inlineText && !looksStructuredPaste(plainText)) {
          event.preventDefault();
          currentEditor.insertInlineContent(inlineText);
          return true;
        }

        return defaultPasteHandler({
          prioritizeMarkdownOverHTML: true,
          plainTextAsMarkdown: true,
        });
      },
    },
    [editorEpoch],
  );

  useEffect(() => {
    document.documentElement.style.zoom = String(zoomLevel);
    localStorage.setItem(ZOOM_KEY, String(zoomLevel));
  }, [zoomLevel]);

  useEffect(() => {
    localStorage.setItem(SIDEBAR_KEY, sidebarCollapsed ? "1" : "0");
  }, [sidebarCollapsed]);

  useEffect(() => {
    if (!entry) {
      return;
    }
    hydratingRef.current = true;
    const blocks = editor.tryParseMarkdownToBlocks(normalizeMarkdown(entry.markdownSource));
    editor.replaceBlocks(editor.document, blocks);
    const currentMarkdown = normalizeMarkdown(editor.blocksToMarkdownLossy(editor.document));
    setEntry((prev) => (prev ? { ...prev, markdownSource: currentMarkdown } : prev));
    requestAnimationFrame(() => {
      hydratingRef.current = false;
    });
  }, [editor, entry?.date, editorEpoch]);

  useEffect(() => {
    return editor.onChange(() => {
      if (hydratingRef.current) {
        return;
      }
      const markdownSource = normalizeMarkdown(editor.blocksToMarkdownLossy(editor.document));
      setEntry((prev) => (prev ? { ...prev, markdownSource } : prev));
    });
  }, [editor]);

  async function refreshGitStatus() {
    try {
      const result = await invoke("git_status");
      setGitStatus(result.statusText);
    } catch (error) {
      setGitStatus(`状態を表示できませんでした: ${error}`);
    }
  }

  async function loadWorkspaceSettings() {
    try {
      const result = await invoke("get_workspace_settings");
      setWorkspacePath(result.configured ? result.workspacePath : "");
      setWorkspaceInput(result.workspacePath || "");
      setAutoCommitOnSave(result.autoCommitOnSave);
      setAutoPushOnSave(result.autoPushOnSave);
      return result.configured;
    } catch (error) {
      setStatus({ message: `保存先を読めませんでした: ${error}`, kind: "error" });
      return false;
    }
  }

  async function loadEntry(date) {
    setStatus({ message: "読み込み中です...", kind: "" });
    try {
      const payload = await invoke("load_entry", { date });
      const nextEntry = payload.entry || emptyEntry(date);
      setWorkspacePath(payload.workspacePath);
      setWorkspaceInput(payload.workspacePath);
      setEntry(nextEntry);
      setEntryDate(nextEntry.date);
      setCommitMessage(defaultCommitMessage(nextEntry.date));
      setEditorEpoch((value) => value + 1);
      await refreshGitStatus();
      setStatus({ message: "読み込みました。", kind: "success" });
    } catch (error) {
      const fallback = emptyEntry(date);
      setEntry(fallback);
      setEntryDate(date);
      setCommitMessage(defaultCommitMessage(date));
      setEditorEpoch((value) => value + 1);
      setStatus({ message: `読み込めませんでした: ${error}`, kind: "error" });
    }
  }

  async function saveEntry() {
    if (!entry) {
      return false;
    }
    setStatus({ message: "保存しています...", kind: "" });
    try {
      const result = await invoke("save_entry", {
        entry: {
          ...entry,
          markdownSource: normalizeMarkdown(editor.blocksToMarkdownLossy(editor.document)),
        },
      });
      const nextEntry = { ...entry, markdownSource: result.markdown };
      setEntry(nextEntry);
      if (autoCommitOnSave) {
        const syncResult = await invoke("git_commit_changes", {
          commitMessage: commitMessage.trim() || defaultCommitMessage(nextEntry.date),
          push: autoPushOnSave,
        });
        setGitStatus(syncResult.statusText);
        setStatus({ message: syncResult.summary, kind: "success" });
      } else {
        await refreshGitStatus();
        setStatus({ message: "保存しました。", kind: "success" });
      }
      return true;
    } catch (error) {
      setStatus({ message: `保存できませんでした: ${error}`, kind: "error" });
      return false;
    }
  }

  async function saveAndPush() {
    const saved = await saveEntry();
    if (!saved || !entry || autoPushOnSave) {
      return;
    }
    setStatus({ message: "反映しています...", kind: "" });
    try {
      const result = await invoke("git_commit_changes", {
        commitMessage: commitMessage.trim() || defaultCommitMessage(entry.date),
        push: true,
      });
      setGitStatus(result.statusText);
      setStatus({ message: result.summary, kind: "success" });
    } catch (error) {
      setStatus({ message: `反映できませんでした: ${error}`, kind: "error" });
      await refreshGitStatus();
    }
  }

  async function saveWorkspaceSettings() {
    setStatus({ message: "保存先を更新しています...", kind: "" });
    try {
      const result = await invoke("save_app_settings", {
        workspacePath: workspaceInput,
        autoCommitOnSave,
        autoPushOnSave,
      });
      setWorkspacePath(result.workspacePath);
      setWorkspaceInput(result.workspacePath);
      setAutoCommitOnSave(result.autoCommitOnSave);
      setAutoPushOnSave(result.autoPushOnSave);
      await loadEntry(entryDate || todayIso());
      setStatus({ message: "保存先を更新しました。", kind: "success" });
    } catch (error) {
      setStatus({ message: `保存先を更新できませんでした: ${error}`, kind: "error" });
    }
  }

  async function browseWorkspacePath() {
    try {
      const selectedPath = await invoke("pick_workspace_path");
      if (selectedPath) {
        setWorkspaceInput(selectedPath);
      }
    } catch (error) {
      if (String(error).includes("選択を取り消しました")) {
        return;
      }
      setStatus({ message: `保存先を選べませんでした: ${error}`, kind: "error" });
    }
  }

  useEffect(() => {
    const bootstrap = async () => {
      const configured = await loadWorkspaceSettings();
      if (configured) {
        await loadEntry(todayIso());
      } else {
        const date = todayIso();
        const nextEntry = emptyEntry(date);
        setEntry(nextEntry);
        setEntryDate(date);
        setCommitMessage(defaultCommitMessage(date));
        setEditorEpoch((value) => value + 1);
        setStatus({ message: "保存先を設定してください。", kind: "error" });
      }
    };
    bootstrap();
  }, []);

  useEffect(() => {
    const onKeyDown = (event) => {
      const usesMeta = event.metaKey || event.ctrlKey;
      if (usesMeta && event.key === "\\") {
        event.preventDefault();
        setSidebarCollapsed((value) => !value);
        return;
      }
      if (usesMeta && event.key === "-") {
        event.preventDefault();
        setZoomLevelState((value) => Math.max(0.7, Number((value - 0.1).toFixed(2))));
        return;
      }
      if (usesMeta && (event.key === "=" || event.key === "+")) {
        event.preventDefault();
        setZoomLevelState((value) => Math.min(1.6, Number((value + 0.1).toFixed(2))));
        return;
      }
      if (usesMeta && event.key === "0") {
        event.preventDefault();
        setZoomLevelState(1);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const configured = Boolean(workspacePath);

  return (
    <div className={`shell${sidebarCollapsed ? " sidebar-collapsed" : ""}`}>
      <aside className="sidebar">
        <button
          className="sidebar-toggle ghost"
          type="button"
          aria-label={sidebarCollapsed ? "サイドバーを開く" : "サイドバーを閉じる"}
          aria-expanded={!sidebarCollapsed}
          onClick={() => setSidebarCollapsed((value) => !value)}
        >
          <span className="sidebar-toggle-icon" aria-hidden="true">
            {sidebarCollapsed ? "▸" : "◂"}
          </span>
        </button>

        <p className="app-name">WORK LOG</p>

        <div className="meta-card">
          <p className="meta-label">保存先</p>
          <label className="field-label" htmlFor="workspacePathInput">
            ログ用 repo のパス
          </label>
          <div className="workspace-row">
            <input
              id="workspacePathInput"
              className="workspace-input"
              type="text"
              placeholder="~/Repositories/work-log-data"
              value={workspaceInput}
              onChange={(event) => setWorkspaceInput(event.target.value)}
            />
            <button className="ghost" type="button" onClick={browseWorkspacePath}>
              選ぶ
            </button>
          </div>
          <button className="ghost workspace-save-button" type="button" onClick={saveWorkspaceSettings}>
            保存先を更新
          </button>
          <label className="setting-check">
            <input
              type="checkbox"
              checked={autoCommitOnSave}
              onChange={(event) => {
                const checked = event.target.checked;
                setAutoCommitOnSave(checked);
                if (!checked) {
                  setAutoPushOnSave(false);
                }
              }}
            />
            <span>保存時に commit</span>
          </label>
          <label className="setting-check">
            <input
              type="checkbox"
              checked={autoPushOnSave}
              onChange={(event) => {
                const checked = event.target.checked;
                setAutoPushOnSave(checked);
                if (checked) {
                  setAutoCommitOnSave(true);
                }
              }}
            />
            <span>保存時に push</span>
          </label>
          <p className="meta-value">{configured ? workspacePath : "未設定"}</p>
        </div>

        <div className="meta-card">
          <p className="meta-label">ショートカット</p>
          <ul className="shortcut-list">
            <li>
              <code>/</code> で slash menu
            </li>
            <li>
              <code>Tab</code> / <code>Shift + Tab</code> でネスト
            </li>
            <li>
              <code>Cmd/Ctrl + -</code> で縮小
            </li>
            <li>
              <code>Cmd/Ctrl + =</code> で拡大
            </li>
          </ul>
        </div>
      </aside>

      <main className="main">
        <section className="topbar">
          <div className="topbar-left">
            <button
              className="icon-button ghost"
              type="button"
              aria-label="サイドバーを開く"
              hidden={!sidebarCollapsed}
              onClick={() => setSidebarCollapsed(false)}
            >
              <span aria-hidden="true">☰</span>
            </button>
            <div>
              <label htmlFor="entryDate">対象日</label>
              <input
                id="entryDate"
                type="date"
                value={entryDate}
                disabled={!configured}
                onChange={(event) => {
                  setEntryDate(event.target.value);
                  loadEntry(event.target.value);
                }}
              />
            </div>
          </div>

          <div className="toolbar">
            <button
              className="ghost"
              type="button"
              onClick={() => setShowMetaPanel((value) => !value)}
              title="補助操作を表示"
            >
              その他
            </button>
            <button
              className="primary secondary"
              onClick={saveAndPush}
              disabled={!configured}
              title="保存して commit / push する"
            >
              公開
            </button>
            <button
              className="primary"
              onClick={saveEntry}
              disabled={!configured}
              title="日報を保存する"
            >
              日報を保存
            </button>
          </div>
        </section>

        <div className={`status${status.kind ? ` ${status.kind}` : ""}`}>{status.message}</div>

        {showMetaPanel && (
          <section className="panel meta-panel">
            <div className="meta-panel-grid">
              <button
                className="ghost"
                onClick={() => loadEntry(entryDate)}
                disabled={!configured}
                title="保存済みの内容に戻す"
              >
                保存内容に戻す
              </button>
              <button
                className="ghost"
                onClick={refreshGitStatus}
                disabled={!configured}
                title="Git の変更状態を更新"
              >
                Git 状態を更新
              </button>
            </div>
          </section>
        )}

        <section className="panel editor-panel">
          <div className="panel-head compact">
            <div>
              <p className="panel-title">Daily Editor</p>
            </div>
          </div>

          <div className="editor-surface">
            {entry && (
              <BlockNoteView
                key={`${entry.date}-${editorEpoch}`}
                editor={editor}
                theme="light"
                editable={configured}
                slashMenu
                sideMenu
                formattingToolbar
                linkToolbar
                filePanel={false}
                tableHandles={false}
                emojiPicker={false}
                comments={false}
                className="blocknote-shell"
              />
            )}
          </div>
        </section>

        <section className="panel git-panel">
          <div className="panel-head compact">
            <div>
              <p className="panel-title">Git / GitHub</p>
              <p className="panel-copy">必要ならここで反映します。</p>
            </div>
          </div>
          <div className="git-form">
            <div>
              <label htmlFor="commitMessage">メッセージ</label>
              <input
                id="commitMessage"
                type="text"
                value={commitMessage}
                disabled={!configured}
                onChange={(event) => setCommitMessage(event.target.value)}
              />
            </div>
            <div>
              <p className="meta-label">状態</p>
              <pre className="git-output">{gitStatus}</pre>
            </div>
          </div>
        </section>
      </main>
    </div>
  );
}
