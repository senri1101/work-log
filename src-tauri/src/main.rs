use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TodayItem {
    task: String,
    checked: bool,
    #[serde(default)]
    must_do: bool,
    impact: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct EntryPayload {
    date: String,
    today: Vec<TodayItem>,
    support: Vec<String>,
    improvements: Vec<String>,
    learning: Vec<String>,
    notes: Vec<String>,
    markdown_preview: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoadEntryResponse {
    workspace_path: String,
    entry: Option<EntryPayload>,
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
    let state_path = entry_state_path(&workspace, &date);

    if !state_path.exists() {
        let carry_over = carry_over_entry(&workspace, &date)?;
        return Ok(LoadEntryResponse {
            workspace_path: workspace.display().to_string(),
            entry: carry_over,
        });
    }

    let content = fs::read_to_string(&state_path)
        .map_err(|err| format!("編集中データの読み込みに失敗しました: {err}"))?;
    let mut entry: EntryPayload = serde_json::from_str(&content)
        .map_err(|err| format!("編集中データの解析に失敗しました: {err}"))?;
    entry.markdown_preview = render_markdown_text(&entry);

    Ok(LoadEntryResponse {
        workspace_path: workspace.display().to_string(),
        entry: Some(entry),
    })
}

#[tauri::command]
fn render_markdown(entry: EntryPayload) -> Result<String, String> {
    Ok(render_markdown_text(&entry))
}

#[tauri::command]
fn save_entry(app: AppHandle, entry: EntryPayload) -> Result<SaveEntryResponse, String> {
    let workspace = workspace_root(&app)?;
    let state_path = entry_state_path(&workspace, &entry.date);
    let markdown_path = markdown_output_path(&workspace, &entry.date)?;
    let normalized = normalize_entry(entry);
    let markdown = render_markdown_text(&normalized);
    let persisted = EntryPayload {
        markdown_preview: markdown.clone(),
        ..normalized.clone()
    };

    ensure_parent(&state_path)?;
    ensure_parent(&markdown_path)?;

    let serialized = serde_json::to_string_pretty(&persisted)
        .map_err(|err| format!("編集中データの変換に失敗しました: {err}"))?;
    fs::write(&state_path, serialized)
        .map_err(|err| format!("編集中データの保存に失敗しました: {err}"))?;
    fs::write(&markdown_path, &markdown)
        .map_err(|err| format!("Markdown の保存に失敗しました: {err}"))?;

    Ok(SaveEntryResponse {
        workspace_path: workspace.display().to_string(),
        markdown_path: markdown_path.display().to_string(),
        state_path: state_path.display().to_string(),
        markdown,
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
fn git_commit_changes(app: AppHandle, commit_message: String, push: bool) -> Result<GitPushResponse, String> {
    let workspace = workspace_root(&app)?;
    let push_output = sync_git_changes(&workspace, commit_message.trim(), push)?;
    let status_after = run_git_command(&workspace, &["status", "--short", "--branch"])?;

    Ok(GitPushResponse {
        status_text: format!("{status_after}\n\n{push_output}").trim().to_string(),
        summary: if push {
            "反映しました。".to_string()
        } else {
            "commit しました。".to_string()
        },
    })
}

fn normalize_entry(mut entry: EntryPayload) -> EntryPayload {
    entry.today = entry
        .today
        .into_iter()
        .filter(|item| !item.task.trim().is_empty() || !item.impact.trim().is_empty())
        .map(|item| TodayItem {
            task: item.task.trim().to_string(),
            checked: item.checked,
            must_do: item.must_do,
            impact: item.impact.trim().to_string(),
        })
        .collect();
    entry.support = sanitize_lines(entry.support);
    entry.improvements = sanitize_lines(entry.improvements);
    entry.learning = sanitize_lines(entry.learning);
    entry.notes = sanitize_lines(entry.notes);
    entry
}

fn sanitize_lines(lines: Vec<String>) -> Vec<String> {
    lines.into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn render_markdown_text(entry: &EntryPayload) -> String {
    let mut sections: Vec<String> = Vec::new();

    let done_items: Vec<&TodayItem> = entry
        .today
        .iter()
        .filter(|item| item.checked && !item.task.trim().is_empty())
        .collect();

    if !done_items.is_empty() {
        let mut lines = vec!["## done".to_string()];
        for item in done_items {
            let task = if item.must_do {
                format!("[必達] {}", item.task.trim())
            } else {
                item.task.trim().to_string()
            };
            lines.push(format!("- task: {task}"));
            if !item.impact.trim().is_empty() {
                lines.push(format!("  impact: {}", item.impact.trim()));
            }
        }
        sections.push(lines.join("\n"));
    }

    push_section(&mut sections, "support", &entry.support);
    push_section(&mut sections, "improvements", &entry.improvements);
    push_section(&mut sections, "learning", &entry.learning);
    push_section(&mut sections, "notes", &entry.notes);

    if sections.is_empty() {
        format!("# {}\n", entry.date)
    } else {
        format!("# {}\n\n{}\n", entry.date, sections.join("\n\n"))
    }
}

fn carry_over_entry(workspace: &Path, date: &str) -> Result<Option<EntryPayload>, String> {
    let previous_date = previous_date_string(date)?;
    let previous_state_path = entry_state_path(workspace, &previous_date);

    if !previous_state_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&previous_state_path)
        .map_err(|err| format!("前日の編集中データの読み込みに失敗しました: {err}"))?;
    let previous_entry: EntryPayload = serde_json::from_str(&content)
        .map_err(|err| format!("前日の編集中データの解析に失敗しました: {err}"))?;

    let carried_today: Vec<TodayItem> = previous_entry
        .today
        .into_iter()
        .filter(|item| !item.checked && !item.task.trim().is_empty())
        .map(|item| TodayItem {
            task: item.task.trim().to_string(),
            checked: false,
            must_do: item.must_do,
            impact: String::new(),
        })
        .collect();

    if carried_today.is_empty() {
        return Ok(None);
    }

    Ok(Some(EntryPayload {
        date: date.to_string(),
        today: carried_today,
        support: Vec::new(),
        improvements: Vec::new(),
        learning: Vec::new(),
        notes: Vec::new(),
        markdown_preview: format!("# {date}\n"),
    }))
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

fn push_section(sections: &mut Vec<String>, heading: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    let mut lines = vec![format!("## {heading}")];
    for item in items {
        lines.push(format!("- {}", item.trim()));
    }
    sections.push(lines.join("\n"));
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
    let content = fs::read_to_string(&path)
        .map_err(|err| format!("設定の読み込みに失敗しました: {err}"))?;
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
        let home = std::env::var("HOME").map_err(|_| "ホームディレクトリを取得できませんでした。".to_string())?;
        PathBuf::from(home).join(stripped)
    } else {
        PathBuf::from(trimmed)
    };
    if !expanded.exists() {
        return Err(format!("保存先が見つかりません: {}", expanded.display()));
    }
    if !expanded.is_dir() {
        return Err(format!("保存先はフォルダを指定してください: {}", expanded.display()));
    }
    expanded
        .canonicalize()
        .map_err(|err| format!("保存先の解決に失敗しました: {err}"))
}

fn initialize_workspace(workspace: &Path) -> Result<(), String> {
    for name in ["daily", "achievements", "reviews", "weekly", "tech-notes", ".work-log-state"] {
        fs::create_dir_all(workspace.join(name))
            .map_err(|err| format!("保存先の初期化に失敗しました ({}): {err}", workspace.join(name).display()))?;
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
        Ok("".to_string())
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
    fs::create_dir_all(parent)
        .map_err(|err| format!("ディレクトリ作成に失敗しました ({}): {err}", parent.display()))
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
            render_markdown,
            save_entry,
            git_status,
            git_commit_changes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
