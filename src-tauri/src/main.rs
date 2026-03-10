use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TodayItem {
    task: String,
    checked: bool,
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
fn load_entry(date: String) -> Result<LoadEntryResponse, String> {
    let workspace = workspace_root()?;
    let state_path = entry_state_path(&workspace, &date);

    if !state_path.exists() {
        return Ok(LoadEntryResponse {
            workspace_path: workspace.display().to_string(),
            entry: None,
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
fn save_entry(entry: EntryPayload) -> Result<SaveEntryResponse, String> {
    let workspace = workspace_root()?;
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
fn git_status() -> Result<GitStatusResponse, String> {
    let workspace = workspace_root()?;
    let branch = run_git_command(&workspace, &["status", "--short", "--branch"])?;
    let remote = run_git_command(&workspace, &["remote", "-v"])?;
    Ok(GitStatusResponse {
        status_text: format!("{branch}\n\n{remote}").trim().to_string(),
    })
}

#[tauri::command]
fn git_commit_and_push(commit_message: String) -> Result<GitPushResponse, String> {
    let workspace = workspace_root()?;
    let status_before = run_git_command(&workspace, &["status", "--short"])?;

    if !status_before.trim().is_empty() {
        run_git_command(
            &workspace,
            &[
                "add",
                "daily",
                "achievements",
                "reviews",
                "weekly",
                "tech-notes",
            ],
        )?;

        let staged = run_git_command(&workspace, &["diff", "--cached", "--name-only"])?;
        if !staged.trim().is_empty() {
            run_git_command(&workspace, &["commit", "-m", commit_message.trim()])?;
        }
    }

    let push_output = run_git_command(&workspace, &["push", "origin", "main"])?;
    let status_after = run_git_command(&workspace, &["status", "--short", "--branch"])?;

    Ok(GitPushResponse {
        status_text: format!("{status_after}\n\n{push_output}").trim().to_string(),
        summary: "反映しました。".to_string(),
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
            lines.push(format!("- task: {}", item.task.trim()));
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

fn workspace_root() -> Result<PathBuf, String> {
    let src_tauri_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    src_tauri_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "ワークスペースのパス解決に失敗しました。".to_string())
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
            load_entry,
            render_markdown,
            save_entry,
            git_status,
            git_commit_and_push
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
