use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct EntryPayload {
    date: String,
    markdown_source: String,
}

impl EntryPayload {
    fn empty(date: &str) -> Self {
        Self {
            date: date.to_string(),
            markdown_source: starter_markdown(date),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    workspace_path: Option<String>,
    #[serde(default)]
    auto_commit_on_save: bool,
    #[serde(default)]
    auto_push_on_save: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceSettingsResponse {
    workspace_path: String,
    configured: bool,
    auto_commit_on_save: bool,
    auto_push_on_save: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoadEntryResponse {
    workspace_path: String,
    entry: Option<EntryPayload>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveEntryResponse {
    workspace_path: String,
    markdown_path: String,
    state_path: String,
    markdown: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitStatusResponse {
    status_text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPushResponse {
    status_text: String,
    summary: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LegacyTodayItem {
    task: String,
    #[serde(default)]
    checked: bool,
    #[serde(default)]
    must_do: bool,
    #[serde(default)]
    impact: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TaskStatus {
    #[default]
    Todo,
    Doing,
    Done,
}

impl TaskStatus {
    fn from_token(token: char) -> Self {
        match token {
            '/' => Self::Doing,
            'x' | 'X' => Self::Done,
            _ => Self::Todo,
        }
    }

    fn token(self) -> char {
        match self {
            Self::Todo => ' ',
            Self::Doing => '/',
            Self::Done => 'x',
        }
    }
}

#[derive(Debug, Clone, Default)]
struct TaskNode {
    text: String,
    status: TaskStatus,
    children: Vec<TaskNode>,
    notes: Vec<String>,
}

impl TaskNode {
    fn carry_over(&self) -> Option<Self> {
        let children = self
            .children
            .iter()
            .filter_map(TaskNode::carry_over)
            .collect::<Vec<_>>();
        if self.status == TaskStatus::Done && children.is_empty() {
            return None;
        }
        Some(Self {
            text: self.text.trim().to_string(),
            status: if self.status == TaskStatus::Done {
                TaskStatus::Doing
            } else {
                self.status
            },
            children,
            notes: self.notes.clone(),
        })
    }
}

#[derive(Debug, Clone, Default)]
struct EntryDoc {
    must_do_tasks: Vec<TaskNode>,
    queued_tasks: Vec<TaskNode>,
    pending_tasks: Vec<TaskNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseSection {
    MustDo,
    Queued,
    Pending,
    LegacyToday,
    LegacyDone,
    LegacyImpact,
    LegacySupport,
    LegacyImprovements,
    LegacyLearning,
    LegacyNotes,
    Other,
}

#[tauri::command]
fn get_workspace_settings(app: AppHandle) -> Result<WorkspaceSettingsResponse, String> {
    let settings = load_settings(&app)?;
    let workspace_path = settings.workspace_path.unwrap_or_default();
    Ok(WorkspaceSettingsResponse {
        configured: !workspace_path.is_empty(),
        workspace_path,
        auto_commit_on_save: settings.auto_commit_on_save,
        auto_push_on_save: settings.auto_push_on_save,
    })
}

#[tauri::command]
fn save_app_settings(
    app: AppHandle,
    workspace_path: String,
    auto_commit_on_save: bool,
    auto_push_on_save: bool,
) -> Result<WorkspaceSettingsResponse, String> {
    let normalized = normalize_workspace_path(&workspace_path)?;
    initialize_workspace(&normalized)?;
    save_settings(
        &app,
        &AppSettings {
            workspace_path: Some(normalized.display().to_string()),
            auto_commit_on_save: auto_commit_on_save || auto_push_on_save,
            auto_push_on_save,
        },
    )?;
    Ok(WorkspaceSettingsResponse {
        configured: true,
        workspace_path: normalized.display().to_string(),
        auto_commit_on_save: auto_commit_on_save || auto_push_on_save,
        auto_push_on_save,
    })
}

#[tauri::command]
fn pick_workspace_path() -> Result<String, String> {
    let output = Command::new("osascript")
        .args([
            "-e",
            "POSIX path of (choose folder with prompt \"ログ用の保存先を選んでください\")",
        ])
        .output()
        .map_err(|err| format!("フォルダ選択の起動に失敗しました: {err}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("-128") {
            Err("選択を取り消しました。".to_string())
        } else {
            Err(format!("フォルダを選べませんでした: {}", stderr.trim()))
        }
    }
}

#[tauri::command]
fn load_entry(app: AppHandle, date: String) -> Result<LoadEntryResponse, String> {
    let workspace = workspace_root(&app)?;
    let entry = if let Some(entry) = read_saved_entry(&workspace, &date)? {
        Some(entry)
    } else {
        carry_over_entry(&workspace, &date)?
    };

    Ok(LoadEntryResponse {
        workspace_path: workspace.display().to_string(),
        entry,
    })
}

#[tauri::command]
fn save_entry(app: AppHandle, entry: EntryPayload) -> Result<SaveEntryResponse, String> {
    let workspace = workspace_root(&app)?;
    let normalized = EntryPayload {
        date: entry.date.trim().to_string(),
        markdown_source: normalize_line_endings(&entry.markdown_source),
    };
    let state_path = entry_state_path(&workspace, &normalized.date);
    let markdown_path = markdown_output_path(&workspace, &normalized.date)?;

    ensure_parent(&state_path)?;
    ensure_parent(&markdown_path)?;

    let serialized = serde_json::to_string_pretty(&normalized)
        .map_err(|err| format!("編集中データの変換に失敗しました: {err}"))?;
    fs::write(&state_path, serialized)
        .map_err(|err| format!("編集中データの保存に失敗しました: {err}"))?;
    fs::write(&markdown_path, &normalized.markdown_source)
        .map_err(|err| format!("Markdown の保存に失敗しました: {err}"))?;

    Ok(SaveEntryResponse {
        workspace_path: workspace.display().to_string(),
        markdown_path: markdown_path.display().to_string(),
        state_path: state_path.display().to_string(),
        markdown: normalized.markdown_source,
    })
}

#[tauri::command]
fn git_status(app: AppHandle) -> Result<GitStatusResponse, String> {
    let workspace = workspace_root(&app)?;
    let branch = run_git_command(&workspace, &["status", "--short", "--branch"])?;
    let remote = run_git_command(&workspace, &["remote", "-v"])?;
    Ok(GitStatusResponse {
        status_text: format!("{branch}\n\n{remote}").trim().to_string(),
    })
}

#[tauri::command]
fn git_commit_changes(
    app: AppHandle,
    commit_message: String,
    push: bool,
) -> Result<GitPushResponse, String> {
    let workspace = workspace_root(&app)?;
    let push_output = sync_git_changes(&workspace, commit_message.trim(), push)?;
    let status_after = run_git_command(&workspace, &["status", "--short", "--branch"])?;

    Ok(GitPushResponse {
        status_text: format!("{status_after}\n\n{push_output}")
            .trim()
            .to_string(),
        summary: if push {
            "反映しました。".to_string()
        } else {
            "commit しました。".to_string()
        },
    })
}

fn read_saved_entry(workspace: &Path, date: &str) -> Result<Option<EntryPayload>, String> {
    let state_path = entry_state_path(workspace, date);
    if state_path.exists() {
        let content = fs::read_to_string(&state_path)
            .map_err(|err| format!("編集中データの読み込みに失敗しました: {err}"))?;
        if let Ok(entry) = parse_entry_json(&content, date) {
            return Ok(Some(entry));
        }
    }

    let markdown_path = markdown_output_path(workspace, date)?;
    if markdown_path.exists() {
        let content = fs::read_to_string(&markdown_path)
            .map_err(|err| format!("Markdown の読み込みに失敗しました: {err}"))?;
        return Ok(Some(EntryPayload {
            date: date.to_string(),
            markdown_source: normalize_line_endings(&content),
        }));
    }

    Ok(None)
}

fn parse_entry_json(content: &str, date_hint: &str) -> Result<EntryPayload, String> {
    let value: Value = serde_json::from_str(content)
        .map_err(|err| format!("編集中データの解析に失敗しました: {err}"))?;

    if value.get("markdownSource").is_some() {
        let mut entry: EntryPayload = serde_json::from_value(value)
            .map_err(|err| format!("編集中データの解析に失敗しました: {err}"))?;
        if entry.date.trim().is_empty() {
            entry.date = date_hint.to_string();
        }
        entry.markdown_source = normalize_line_endings(&entry.markdown_source);
        return Ok(entry);
    }

    if let Some(markdown) = value.get("markdownPreview").and_then(Value::as_str) {
        return Ok(EntryPayload {
            date: value
                .get("date")
                .and_then(Value::as_str)
                .unwrap_or(date_hint)
                .to_string(),
            markdown_source: normalize_line_endings(markdown),
        });
    }

    if value.get("today").is_some() {
        return parse_legacy_entry(value, date_hint);
    }

    Err("編集中データの形式に対応していません。".to_string())
}

fn parse_legacy_entry(value: Value, date_hint: &str) -> Result<EntryPayload, String> {
    let date = value
        .get("date")
        .and_then(Value::as_str)
        .filter(|date| !date.trim().is_empty())
        .unwrap_or(date_hint)
        .to_string();
    let today_items = value
        .get("today")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let today: Vec<LegacyTodayItem> = serde_json::from_value(today_items)
        .map_err(|err| format!("旧形式データの解析に失敗しました: {err}"))?;

    let mut lines = vec![
        format!("# {date}"),
        String::new(),
        "## ✅ 今日やること".to_string(),
        String::new(),
        "### 🚨 今日必達".to_string(),
    ];

    for item in today.iter().filter(|item| item.must_do) {
        lines.push(format!(
            "- [{}] {}",
            if item.checked { "x" } else { " " },
            item.task.trim()
        ));
        if !item.impact.trim().is_empty() {
            lines.push(format!("  - impact: {}", item.impact.trim()));
        }
    }

    lines.push(String::new());
    lines.push("### 🐻 必達以外".to_string());
    for item in today.iter().filter(|item| !item.must_do) {
        lines.push(format!(
            "- [{}] {}",
            if item.checked { "x" } else { " " },
            item.task.trim()
        ));
        if !item.impact.trim().is_empty() {
            lines.push(format!("  - impact: {}", item.impact.trim()));
        }
    }

    lines.push(String::new());
    lines.push("## 📝 メモ / 気づき".to_string());
    lines.extend(prefixed_legacy_lines(&value, "support", "support")?);
    lines.extend(prefixed_legacy_lines(
        &value,
        "improvements",
        "improvement",
    )?);
    lines.extend(prefixed_legacy_lines(&value, "learning", "learning")?);
    lines.extend(prefixed_legacy_lines(&value, "notes", "note")?);
    lines.push(String::new());
    lines.push("## 🐕 保留".to_string());

    Ok(EntryPayload {
        date,
        markdown_source: lines.join("\n") + "\n",
    })
}

fn prefixed_legacy_lines(value: &Value, key: &str, prefix: &str) -> Result<Vec<String>, String> {
    let items = value
        .get(key)
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let parsed: Vec<String> = serde_json::from_value(items)
        .map_err(|err| format!("旧形式データの解析に失敗しました ({key}): {err}"))?;
    Ok(parsed
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .map(|item| format!("- {prefix}: {item}"))
        .collect())
}

fn carry_over_entry(workspace: &Path, date: &str) -> Result<Option<EntryPayload>, String> {
    let previous_date = previous_date_string(date)?;
    let Some(previous_entry) = read_saved_entry(workspace, &previous_date)? else {
        return Ok(Some(EntryPayload::empty(date)));
    };

    let previous_doc = parse_markdown_to_doc(&previous_entry.markdown_source);
    let must_do_tasks = previous_doc
        .must_do_tasks
        .iter()
        .filter_map(TaskNode::carry_over)
        .collect::<Vec<_>>();
    let queued_tasks = previous_doc
        .queued_tasks
        .iter()
        .filter_map(TaskNode::carry_over)
        .collect::<Vec<_>>();
    let pending_tasks = previous_doc
        .pending_tasks
        .iter()
        .filter_map(TaskNode::carry_over)
        .collect::<Vec<_>>();

    let markdown_source = render_doc(
        date,
        &EntryDoc {
            must_do_tasks,
            queued_tasks,
            pending_tasks,
        },
    );
    Ok(Some(EntryPayload {
        date: date.to_string(),
        markdown_source,
    }))
}

fn starter_markdown(date: &str) -> String {
    format!(
        "# {date}\n\n## ✅ 今日やること\n\n### 🚨 今日必達\n\n### 🐻 必達以外\n\n## 📝 メモ / 気づき\n\n## 🐕 保留\n"
    )
}

fn render_doc(date: &str, doc: &EntryDoc) -> String {
    let mut lines = vec![
        format!("# {date}"),
        String::new(),
        "## ✅ 今日やること".to_string(),
        String::new(),
        "### 🚨 今日必達".to_string(),
    ];
    render_tasks(&doc.must_do_tasks, 0, &mut lines);
    lines.push(String::new());
    lines.push("### 🐻 必達以外".to_string());
    render_tasks(&doc.queued_tasks, 0, &mut lines);
    lines.push(String::new());
    lines.push("## 📝 メモ / 気づき".to_string());
    lines.push(String::new());
    lines.push("## 🐕 保留".to_string());
    render_tasks(&doc.pending_tasks, 0, &mut lines);
    lines.join("\n") + "\n"
}

fn render_tasks(tasks: &[TaskNode], depth: usize, lines: &mut Vec<String>) {
    for task in tasks {
        let indent = "  ".repeat(depth);
        lines.push(format!(
            "{indent}- [{}] {}",
            task.status.token(),
            task.text.trim()
        ));
        for note in &task.notes {
            lines.push(format!("{indent}  - {}", note.trim()));
        }
        render_tasks(&task.children, depth + 1, lines);
    }
}

fn parse_markdown_to_doc(markdown: &str) -> EntryDoc {
    let mut doc = EntryDoc::default();
    let mut current_section = ParseSection::Other;
    let mut stack: Vec<(usize, Vec<usize>)> = Vec::new();

    for raw_line in markdown.lines() {
        let line = raw_line.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        if let Some(heading) = parse_heading(line) {
            current_section = normalize_section_name(&heading);
            stack.clear();
            continue;
        }

        let (indent, text) = parse_content_line(line);
        if text.is_empty() {
            continue;
        }

        match current_section {
            ParseSection::MustDo
            | ParseSection::Queued
            | ParseSection::Pending
            | ParseSection::LegacyToday
            | ParseSection::LegacyDone => {
                if let Some((status, body)) = parse_checkbox(&text) {
                    append_task(
                        &mut doc,
                        current_section,
                        &mut stack,
                        indent,
                        TaskNode {
                            text: body,
                            status,
                            children: Vec::new(),
                            notes: Vec::new(),
                        },
                    );
                    continue;
                }

                match current_section {
                    ParseSection::LegacyDone if indent == 0 => {
                        append_task(
                            &mut doc,
                            current_section,
                            &mut stack,
                            indent,
                            TaskNode {
                                text: strip_task_prefix(&text),
                                status: TaskStatus::Done,
                                children: Vec::new(),
                                notes: Vec::new(),
                            },
                        );
                    }
                    ParseSection::LegacyToday if indent == 0 => {
                        append_task(
                            &mut doc,
                            current_section,
                            &mut stack,
                            indent,
                            TaskNode {
                                text: strip_task_prefix(&text),
                                status: TaskStatus::Todo,
                                children: Vec::new(),
                                notes: Vec::new(),
                            },
                        );
                    }
                    ParseSection::LegacyImpact => {}
                    _ => attach_note(&mut doc, current_section, &stack, indent, &text),
                }
            }
            ParseSection::LegacyImpact
            | ParseSection::LegacySupport
            | ParseSection::LegacyImprovements
            | ParseSection::LegacyLearning
            | ParseSection::LegacyNotes
            | ParseSection::Other => {}
        }
    }

    doc
}

fn parse_heading(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let hash_count = trimmed.chars().take_while(|ch| *ch == '#').count();
    if hash_count == 0 {
        return None;
    }
    let rest = trimmed[hash_count..].trim();
    if rest.is_empty() {
        None
    } else {
        Some(normalize_whitespace(rest))
    }
}

fn normalize_section_name(name: &str) -> ParseSection {
    if name.contains("今日必達") {
        return ParseSection::MustDo;
    }
    if name.contains("必達以外") {
        return ParseSection::Queued;
    }
    if name.contains("保留") {
        return ParseSection::Pending;
    }

    let key = name
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .collect::<String>()
        .to_ascii_lowercase();
    match key.as_str() {
        "today" | "todo" | "tasks" | "task" => ParseSection::LegacyToday,
        "done" => ParseSection::LegacyDone,
        "impact" => ParseSection::LegacyImpact,
        "support" => ParseSection::LegacySupport,
        "improvement" | "improvements" => ParseSection::LegacyImprovements,
        "learning" => ParseSection::LegacyLearning,
        "notes" => ParseSection::LegacyNotes,
        _ => ParseSection::Other,
    }
}

fn parse_content_line(line: &str) -> (usize, String) {
    let trimmed = line.trim_start_matches(' ');
    let indent = (line.len() - trimmed.len()) / 2;
    if let Some(rest) = trimmed.strip_prefix("- ") {
        (indent, normalize_whitespace(rest))
    } else {
        (indent, normalize_whitespace(trimmed))
    }
}

fn parse_checkbox(text: &str) -> Option<(TaskStatus, String)> {
    let rest = text.strip_prefix('[')?;
    let token = rest.chars().next()?;
    let rest = rest.get(1..)?;
    let body = rest.strip_prefix("] ")?.trim().to_string();
    Some((TaskStatus::from_token(token), body))
}

fn append_task(
    doc: &mut EntryDoc,
    section: ParseSection,
    stack: &mut Vec<(usize, Vec<usize>)>,
    indent: usize,
    task: TaskNode,
) {
    while stack.last().is_some_and(|(level, _)| *level >= indent) {
        stack.pop();
    }

    let parent_path = stack
        .last()
        .map(|(_, path)| path.clone())
        .unwrap_or_default();
    let tasks = task_bucket_mut(doc, section);
    let index = push_task(tasks, &parent_path, task);
    let mut next_path = parent_path;
    next_path.push(index);
    stack.push((indent, next_path));
}

fn attach_note(
    doc: &mut EntryDoc,
    section: ParseSection,
    stack: &[(usize, Vec<usize>)],
    indent: usize,
    text: &str,
) {
    let Some(path) = note_target_path(stack, indent) else {
        return;
    };
    let tasks = task_bucket_mut(doc, section);
    if let Some(node) = get_task_mut(tasks, path) {
        node.notes.push(text.to_string());
    }
}

fn task_bucket_mut(doc: &mut EntryDoc, section: ParseSection) -> &mut Vec<TaskNode> {
    match section {
        ParseSection::MustDo => &mut doc.must_do_tasks,
        ParseSection::Pending => &mut doc.pending_tasks,
        ParseSection::Queued
        | ParseSection::LegacyToday
        | ParseSection::LegacyDone
        | ParseSection::Other
        | ParseSection::LegacyImpact
        | ParseSection::LegacySupport
        | ParseSection::LegacyImprovements
        | ParseSection::LegacyLearning
        | ParseSection::LegacyNotes => &mut doc.queued_tasks,
    }
}

fn push_task(tasks: &mut Vec<TaskNode>, path: &[usize], task: TaskNode) -> usize {
    let list = task_list_mut(tasks, path);
    let index = list.len();
    list.push(task);
    index
}

fn task_list_mut<'a>(tasks: &'a mut Vec<TaskNode>, path: &[usize]) -> &'a mut Vec<TaskNode> {
    let mut current = tasks;
    for index in path {
        current = &mut current[*index].children;
    }
    current
}

fn get_task_mut<'a>(tasks: &'a mut Vec<TaskNode>, path: &[usize]) -> Option<&'a mut TaskNode> {
    let (first, rest) = path.split_first()?;
    let node = tasks.get_mut(*first)?;
    if rest.is_empty() {
        return Some(node);
    }
    get_task_mut(&mut node.children, rest)
}

fn note_target_path(stack: &[(usize, Vec<usize>)], indent: usize) -> Option<&[usize]> {
    if stack.is_empty() {
        return None;
    }
    let preferred_indent = indent.saturating_sub(1);
    for (level, path) in stack.iter().rev() {
        if *level <= preferred_indent {
            return Some(path.as_slice());
        }
    }
    Some(stack[0].1.as_slice())
}

fn strip_task_prefix(text: &str) -> String {
    if text.to_ascii_lowercase().starts_with("task:") {
        text.split_once(':')
            .map(|(_, value)| value.trim().to_string())
            .unwrap_or_else(|| text.trim().to_string())
    } else {
        text.trim().to_string()
    }
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_line_endings(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    if normalized.ends_with('\n') {
        normalized
    } else {
        format!("{normalized}\n")
    }
}

fn previous_date_string(date: &str) -> Result<String, String> {
    let mut parts = date.split('-');
    let mut year: i32 = parts
        .next()
        .ok_or_else(|| format!("日付形式が不正です: {date}"))?
        .parse()
        .map_err(|_| format!("日付形式が不正です: {date}"))?;
    let mut month: i32 = parts
        .next()
        .ok_or_else(|| format!("日付形式が不正です: {date}"))?
        .parse()
        .map_err(|_| format!("日付形式が不正です: {date}"))?;
    let mut day: i32 = parts
        .next()
        .ok_or_else(|| format!("日付形式が不正です: {date}"))?
        .parse()
        .map_err(|_| format!("日付形式が不正です: {date}"))?;

    day -= 1;
    if day == 0 {
        month -= 1;
        if month == 0 {
            year -= 1;
            month = 12;
        }
        day = days_in_month(year, month);
    }

    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 30,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn workspace_root(app: &AppHandle) -> Result<PathBuf, String> {
    let settings = load_settings(app)?;
    let raw = settings
        .workspace_path
        .ok_or_else(|| "保存先を設定してください。".to_string())?;
    normalize_workspace_path(&raw)
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|err| format!("設定ディレクトリの取得に失敗しました: {err}"))?;
    Ok(dir.join("settings.json"))
}

fn load_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let content =
        fs::read_to_string(&path).map_err(|err| format!("設定の読み込みに失敗しました: {err}"))?;
    serde_json::from_str(&content).map_err(|err| format!("設定の解析に失敗しました: {err}"))
}

fn save_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    ensure_parent(&path)?;
    let content = serde_json::to_string_pretty(settings)
        .map_err(|err| format!("設定の変換に失敗しました: {err}"))?;
    fs::write(&path, content).map_err(|err| format!("設定の保存に失敗しました: {err}"))
}

fn normalize_workspace_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("保存先のパスを入力してください。".to_string());
    }
    let expanded = if let Some(stripped) = trimmed.strip_prefix("~/") {
        let home = std::env::var("HOME")
            .map_err(|_| "ホームディレクトリを取得できませんでした。".to_string())?;
        PathBuf::from(home).join(stripped)
    } else {
        PathBuf::from(trimmed)
    };
    if !expanded.exists() {
        return Err(format!("保存先が見つかりません: {}", expanded.display()));
    }
    if !expanded.is_dir() {
        return Err(format!(
            "保存先はフォルダを指定してください: {}",
            expanded.display()
        ));
    }
    expanded
        .canonicalize()
        .map_err(|err| format!("保存先の解決に失敗しました: {err}"))
}

fn initialize_workspace(workspace: &Path) -> Result<(), String> {
    for name in [
        "daily",
        "achievements",
        "reviews",
        "weekly",
        "tech-notes",
        ".work-log-state",
    ] {
        fs::create_dir_all(workspace.join(name)).map_err(|err| {
            format!(
                "保存先の初期化に失敗しました ({}): {err}",
                workspace.join(name).display()
            )
        })?;
    }
    let ignore_path = workspace.join(".work-log-state").join(".gitignore");
    if !ignore_path.exists() {
        fs::write(&ignore_path, "*\n!.gitignore\n")
            .map_err(|err| format!(".gitignore の作成に失敗しました: {err}"))?;
    }
    Ok(())
}

fn sync_git_changes(workspace: &Path, commit_message: &str, push: bool) -> Result<String, String> {
    let status_before = run_git_command(workspace, &["status", "--short"])?;

    if !status_before.trim().is_empty() {
        run_git_command(
            workspace,
            &[
                "add",
                "daily",
                "achievements",
                "reviews",
                "weekly",
                "tech-notes",
            ],
        )?;

        let staged = run_git_command(workspace, &["diff", "--cached", "--name-only"])?;
        if !staged.trim().is_empty() {
            run_git_command(workspace, &["commit", "-m", commit_message])?;
        }
    }

    if push {
        run_git_command(workspace, &["push", "origin", "main"])
    } else {
        Ok(String::new())
    }
}

fn entry_state_path(workspace: &Path, date: &str) -> PathBuf {
    let year = year_from_date(date).unwrap_or_else(|_| "unknown".to_string());
    workspace
        .join(".work-log-state")
        .join("entries")
        .join(year)
        .join(format!("{date}.json"))
}

fn markdown_output_path(workspace: &Path, date: &str) -> Result<PathBuf, String> {
    let year = year_from_date(date)?;
    Ok(workspace
        .join("daily")
        .join(year)
        .join(format!("{date}.md")))
}

fn year_from_date(date: &str) -> Result<String, String> {
    let year = date
        .split('-')
        .next()
        .filter(|value| value.len() == 4)
        .ok_or_else(|| format!("日付形式が不正です: {date}"))?;
    Ok(year.to_string())
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("親ディレクトリが取得できませんでした: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|err| {
        format!(
            "ディレクトリ作成に失敗しました ({}): {err}",
            parent.display()
        )
    })
}

fn run_git_command(workspace: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()
        .map_err(|err| format!("git コマンドの起動に失敗しました: {err}"))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stdout.is_empty() {
            Ok(stderr)
        } else if stderr.is_empty() {
            Ok(stdout)
        } else {
            Ok(format!("{stdout}\n{stderr}"))
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        Err(format!("git {} に失敗しました: {}", args.join(" "), detail))
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_workspace_settings,
            save_app_settings,
            pick_workspace_path,
            load_entry,
            save_entry,
            git_status,
            git_commit_changes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_legacy_json_into_markdown_source() {
        let content = r#"{
          "date": "2026-03-09",
          "today": [
            {"task": "Review PR", "checked": false, "mustDo": true, "impact": ""},
            {"task": "Settings screen fix", "checked": true, "mustDo": false, "impact": "UX improvement"}
          ],
          "support": ["Helped Wang with verification"]
        }"#;

        let entry = parse_entry_json(content, "2026-03-09").unwrap();
        assert!(entry.markdown_source.contains("### 🚨 今日必達"));
        assert!(entry.markdown_source.contains("- [ ] Review PR"));
        assert!(entry.markdown_source.contains("- [x] Settings screen fix"));
        assert!(entry
            .markdown_source
            .contains("- support: Helped Wang with verification"));
    }

    #[test]
    fn carries_over_incomplete_tasks_only() {
        let markdown = r#"# 2026-03-12

## ✅ 今日やること

### 🚨 今日必達
- [ ] 週次ミーティングの準備
  - [x] アジェンダ整理
  - [ ] 共有メモ更新
  - 関連リンクをまとめておく

### 🐻 必達以外
- [x] 完了タスク
- [/] 進行中タスク

## 📝 メモ / 気づき

## 🐕 保留
- [ ] コピペのダイアログ消す
"#;

        let doc = parse_markdown_to_doc(markdown);
        let next = render_doc(
            "2026-03-13",
            &EntryDoc {
                must_do_tasks: doc
                    .must_do_tasks
                    .iter()
                    .filter_map(TaskNode::carry_over)
                    .collect(),
                queued_tasks: doc
                    .queued_tasks
                    .iter()
                    .filter_map(TaskNode::carry_over)
                    .collect(),
                pending_tasks: doc
                    .pending_tasks
                    .iter()
                    .filter_map(TaskNode::carry_over)
                    .collect(),
            },
        );

        assert!(next.contains("- [ ] 週次ミーティングの準備"));
        assert!(next.contains("- [ ] 共有メモ更新"));
        assert!(!next.contains("アジェンダ整理"));
        assert!(!next.contains("完了タスク"));
        assert!(next.contains("- [/] 進行中タスク"));
        assert!(next.contains("- [ ] コピペのダイアログ消す"));
    }

    #[test]
    fn keeps_starter_when_no_tasks_to_carry() {
        let markdown = starter_markdown("2026-03-12");
        let doc = parse_markdown_to_doc(&markdown);
        let next = render_doc(
            "2026-03-13",
            &EntryDoc {
                must_do_tasks: doc
                    .must_do_tasks
                    .iter()
                    .filter_map(TaskNode::carry_over)
                    .collect(),
                queued_tasks: doc
                    .queued_tasks
                    .iter()
                    .filter_map(TaskNode::carry_over)
                    .collect(),
                pending_tasks: doc
                    .pending_tasks
                    .iter()
                    .filter_map(TaskNode::carry_over)
                    .collect(),
            },
        );
        assert!(next.contains("## ✅ 今日やること"));
        assert!(next.contains("## 🐕 保留"));
    }
}
