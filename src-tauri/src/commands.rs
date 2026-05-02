use crate::db::{Database, Idea, Tag};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewriteResult {
    pub rewritten: String,
    pub suggested_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebdavConfig {
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub api_key: String,
    pub endpoint: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub uploaded: usize,
    pub downloaded: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPayload {
    pub ideas: Vec<Idea>,
    pub tags: Vec<Tag>,
    pub idea_tags: Vec<(i64, i64)>,
    pub exported_at: String,
}

#[tauri::command]
pub fn add_idea(content: String, db: State<Database>) -> Result<Idea, String> {
    db.add_idea(&content)
}

#[tauri::command]
pub fn get_ideas(
    search: Option<String>,
    tag_ids: Option<Vec<i64>>,
    sort_desc: Option<bool>,
    db: State<Database>,
) -> Result<Vec<Idea>, String> {
    db.get_ideas(search.as_deref(), &tag_ids.unwrap_or_default(), sort_desc.unwrap_or(true))
}

#[tauri::command]
pub fn update_idea(id: i64, content: String, db: State<Database>) -> Result<Idea, String> {
    db.update_idea(id, &content)
}

#[tauri::command]
pub fn delete_idea(id: i64, db: State<Database>) -> Result<(), String> {
    db.soft_delete_idea(id)
}

#[tauri::command]
pub fn toggle_todo(id: i64, db: State<Database>) -> Result<Idea, String> {
    db.toggle_todo(id)
}

#[tauri::command]
pub fn toggle_todo_done(id: i64, db: State<Database>) -> Result<Idea, String> {
    db.toggle_todo_done(id)
}

#[tauri::command]
pub fn add_tag(idea_id: i64, name: String, db: State<Database>) -> Result<Tag, String> {
    db.add_tag(idea_id, &name)
}

#[tauri::command]
pub fn remove_tag(idea_id: i64, tag_id: i64, db: State<Database>) -> Result<(), String> {
    db.remove_tag(idea_id, tag_id)
}

#[tauri::command]
pub fn get_tags(db: State<Database>) -> Result<Vec<Tag>, String> {
    db.get_tags()
}

#[tauri::command]
pub async fn rewrite_idea(id: i64, db: State<'_, Database>) -> Result<RewriteResult, String> {
    let idea = db.get_idea(id)?;
    let tags = db.get_tags()?;
    let existing_tags: Vec<String> = tags.iter().map(|t| t.name.clone()).collect();

    let ai_config = get_ai_config_from_db(&db)?;

    call_ai_api(&ai_config, &idea.content, &existing_tags).await
}

fn get_ai_config_from_db(db: &Database) -> Result<AIConfig, String> {
    let api_key = db
        .get_setting("ai_api_key")?
        .ok_or("AI API key not configured")?;
    let endpoint = db
        .get_setting("ai_endpoint")?
        .unwrap_or_else(|| "https://api.deepseek.com/v1".into());
    let model = db
        .get_setting("ai_model")?
        .unwrap_or_else(|| "deepseek-v4-flash".into());
    Ok(AIConfig {
        api_key,
        endpoint,
        model,
    })
}

async fn call_ai_api(config: &AIConfig, content: &str, existing_tags: &[String]) -> Result<RewriteResult, String> {
    let client = reqwest::Client::new();

    let endpoint = if config.endpoint.contains("/chat/completions") {
        config.endpoint.clone()
    } else {
        format!("{}/chat/completions", config.endpoint.trim_end_matches('/'))
    };

    let prompt = format!(
        r#"Rewrite the following idea to be clearer while STRICTLY preserving the original meaning and the author's voice.
Do NOT make it sound like AI wrote it. Do NOT add formal language, buzzwords, or corporate tone.
ONLY fix grammar errors, improve sentence flow, and make the text easier to read.
Keep the same length and style. This is a personal memo, not a business document.

Also suggest 1-3 relevant tags from this existing list: {tags}
Only suggest tags that truly fit. If none fit, suggest new short (1-2 word) lowercase tags.
Prefer existing tags over creating new ones.

Respond ONLY with valid JSON (no markdown, no explanation):
{{"rewritten": "...rewritten text...", "suggested_tags": ["tag1", "tag2"]}}

Original text:
{content}"#,
        tags = existing_tags.join(", "),
        content = content
    );

    let body = serde_json::json!({
        "model": config.model,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.3,
        "response_format": { "type": "json_object" }
    });

    let resp = client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("AI API request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("AI API returned {}: {}", status, text));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("AI response parse failed: {}", e))?;

    let text = data["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| format!("Unexpected AI response format: {}", data))?;

    let result: RewriteResult =
        serde_json::from_str(text).map_err(|e| format!("AI JSON parse failed: {} - raw: {}", e, text))?;

    Ok(result)
}

#[tauri::command]
pub fn save_webdav_config(url: String, username: String, password: String, db: State<Database>) -> Result<(), String> {
    db.save_setting("webdav_url", &url)?;
    db.save_setting("webdav_username", &username)?;
    db.save_setting("webdav_password", &password)?;
    Ok(())
}

#[tauri::command]
pub fn get_webdav_config(db: State<Database>) -> Result<Option<WebdavConfig>, String> {
    let url = db.get_setting("webdav_url")?;
    let username = db.get_setting("webdav_username")?;
    let password = db.get_setting("webdav_password")?;
    match (url, username, password) {
        (Some(url), Some(username), Some(password)) => Ok(Some(WebdavConfig {
            url,
            username,
            password,
        })),
        _ => Ok(None),
    }
}

#[tauri::command]
pub fn save_ai_config(
    api_key: String,
    endpoint: Option<String>,
    model: Option<String>,
    db: State<Database>,
) -> Result<(), String> {
    db.save_setting("ai_api_key", &api_key)?;
    if let Some(e) = endpoint {
        db.save_setting("ai_endpoint", &e)?;
    }
    if let Some(m) = model {
        db.save_setting("ai_model", &m)?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_ai_config(db: State<Database>) -> Result<Option<AIConfig>, String> {
    let api_key = db.get_setting("ai_api_key")?;
    match api_key {
        Some(key) => {
            let endpoint = db
                .get_setting("ai_endpoint")?
                .unwrap_or_else(|| "https://api.deepseek.com/v1".into());
            let model = db
                .get_setting("ai_model")?
                .unwrap_or_else(|| "deepseek-v4-flash".into());
            Ok(Some(AIConfig {
                api_key: key,
                endpoint,
                model,
            }))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn sync_now(db: State<'_, Database>) -> Result<SyncResult, String> {
    let config = match get_webdav_config_from_db(&db)? {
        Some(c) => c,
        None => return Err("WebDAV not configured".into()),
    };

    crate::sync::sync_with_webdav(&config, &db).await
}

fn get_webdav_config_from_db(db: &Database) -> Result<Option<WebdavConfig>, String> {
    let url = db.get_setting("webdav_url")?;
    let username = db.get_setting("webdav_username")?;
    let password = db.get_setting("webdav_password")?;
    match (url, username, password) {
        (Some(url), Some(username), Some(password)) => Ok(Some(WebdavConfig {
            url,
            username,
            password,
        })),
        _ => Ok(None),
    }
}

#[tauri::command]
pub fn hide_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| format!("{}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn save_setting(key: String, value: String, db: State<Database>) -> Result<(), String> {
    db.save_setting(&key, &value)
}

#[tauri::command]
pub fn change_shortcut(
    shortcut: String,
    app: tauri::AppHandle,
    db: State<Database>,
    shortcut_state: State<crate::ShortcutState>,
) -> Result<String, String> {
    let new_shortcut = shortcut.trim().to_string();
    if new_shortcut.is_empty() {
        return Err("Shortcut cannot be empty".into());
    }
    let mut current = shortcut_state.current.lock().map_err(|e| format!("Lock: {}", e))?;
    // Unregister old shortcut
    app.global_shortcut().unregister(current.as_str()).ok();
    // Register new
    app.global_shortcut()
        .register(new_shortcut.as_str())
        .map_err(|e| format!("Register: {}", e))?;
    // Save
    db.save_setting("shortcut", &new_shortcut)?;
    *current = new_shortcut.clone();
    Ok(new_shortcut)
}

#[tauri::command]
pub fn get_shortcut(db: State<Database>) -> Result<String, String> {
    Ok(db
        .get_setting("shortcut")
        .unwrap_or(None)
        .unwrap_or_else(|| "Ctrl+Shift+I".to_string()))
}

#[tauri::command]
pub fn get_setting(key: String, db: State<Database>) -> Result<Option<String>, String> {
    db.get_setting(&key)
}

#[tauri::command]
pub fn get_autostart(db: State<Database>) -> Result<bool, String> {
    Ok(db.get_setting("autostart")?.unwrap_or_else(|| "false".into()) == "true")
}

#[tauri::command]
pub fn set_autostart(enabled: bool, app: tauri::AppHandle) -> Result<bool, String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("{}", e))?;
    let exe_str = exe_path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if enabled {
            let output = Command::new("reg")
                .args(["add", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                       "/v", "Inspiration", "/t", "REG_SZ",
                       "/d", &exe_str, "/f"])
                .output().map_err(|e| format!("reg add: {}", e))?;
            if !output.status.success() {
                return Err(format!("reg add failed: {:?}", output));
            }
        } else {
            let output = Command::new("reg")
                .args(["delete", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                       "/v", "Inspiration", "/f"])
                .output().map_err(|e| format!("reg delete: {}", e))?;
            if !output.status.success() {
                // Ignore if key doesn't exist
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let autostart_dir = std::path::PathBuf::from(home).join(".config/autostart");
        std::fs::create_dir_all(&autostart_dir).ok();
        let desktop_file = autostart_dir.join("inspiration.desktop");
        if enabled {
            let content = format!(
                "[Desktop Entry]\nType=Application\nName=Inspiration\nExec={}\nX-GNOME-Autostart-enabled=true\n",
                exe_str
            );
            std::fs::write(&desktop_file, content).map_err(|e| format!("{}", e))?;
        } else {
            std::fs::remove_file(&desktop_file).ok();
        }
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let launch_agents = std::path::PathBuf::from(home).join("Library/LaunchAgents");
        std::fs::create_dir_all(&launch_agents).ok();
        let plist = launch_agents.join("com.inspiration.app.plist");
        if enabled {
            let content = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>com.inspiration.app</string>
<key>ProgramArguments</key><array><string>{}</string></array>
<key>RunAtLoad</key><true/>
</dict></plist>"#,
                exe_str
            );
            std::fs::write(&plist, content).map_err(|e| format!("{}", e))?;
        } else {
            std::fs::remove_file(&plist).ok();
        }
    }

    let _ = app; // Keep for future use
    Ok(enabled)
}

#[tauri::command]
pub fn set_data_dir(path: String, app: tauri::AppHandle) -> Result<(), String> {
    let clean = path.trim().to_string();
    let app_dir = app.path().app_data_dir().map_err(|e| format!("{}", e))?;
    let config_file = app_dir.join("db_path.txt");

    if clean.is_empty() {
        std::fs::remove_file(&config_file).ok();
        return Ok(());
    }
    let p = std::path::Path::new(&clean);
    if p.exists() && !p.is_dir() {
        return Err("Path exists but is not a directory".into());
    }
    std::fs::create_dir_all(p).map_err(|e| format!("{}", e))?;
    std::fs::write(&config_file, &clean).map_err(|e| format!("{}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| format!("{}", e))?;
    let config_file = app_dir.join("db_path.txt");
    if let Ok(contents) = std::fs::read_to_string(&config_file) {
        Ok(contents.trim().to_string())
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
pub fn get_screenshot_shortcut(db: State<Database>) -> Result<String, String> {
    Ok(db
        .get_setting("screenshot_shortcut")
        .unwrap_or(None)
        .unwrap_or_else(|| "Ctrl+Shift+S".to_string()))
}

#[tauri::command]
pub fn change_screenshot_shortcut(
    shortcut: String,
    app: tauri::AppHandle,
    db: State<Database>,
) -> Result<String, String> {
    let new_shortcut = shortcut.trim().to_string();
    if new_shortcut.is_empty() {
        return Err("Shortcut cannot be empty".into());
    }
    let current = db
        .get_setting("screenshot_shortcut")
        .unwrap_or(None)
        .unwrap_or_else(|| "Ctrl+Shift+S".to_string());
    app.global_shortcut().unregister(current.as_str()).ok();
    app.global_shortcut()
        .register(new_shortcut.as_str())
        .map_err(|e| format!("Register: {}", e))?;
    db.save_setting("screenshot_shortcut", &new_shortcut)?;
    Ok(new_shortcut)
}

#[tauri::command]
pub fn take_screenshot(
    x: i32, y: i32, width: i32, height: i32,
    app: tauri::AppHandle,
    db: State<Database>,
) -> Result<String, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| format!("{}", e))?;
    let screenshots_dir = app_dir.join("screenshots");
    std::fs::create_dir_all(&screenshots_dir).ok();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("screenshot_{}.png", timestamp);
    let filepath = screenshots_dir.join(&filename);

    #[cfg(target_os = "windows")]
    {
        let ps_script = format!(
            r#"Add-Type -AssemblyName System.Windows.Forms,System.Drawing;
$bmp = New-Object System.Drawing.Bitmap({w}, {h});
$g = [System.Drawing.Graphics]::FromImage($bmp);
$g.CopyFromScreen({x}, {y}, 0, 0, $bmp.Size);
$bmp.Save('{path}');
$g.Dispose(); $bmp.Dispose()"#,
            x = x, y = y, w = width, h = height,
            path = filepath.to_string_lossy().replace('\\', "\\\\")
        );
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
            .map_err(|e| format!("PowerShell: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Screenshot failed: {}", stderr));
        }
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("screencapture")
            .args(["-R", &format!("{},{},{},{}", x, y, width, height), &filepath.to_string_lossy()])
            .output()
            .map_err(|e| format!("screencapture: {}", e))?;
        if !output.status.success() {
            return Err("screencapture failed".into());
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Try import (ImageMagick) first, then fall back to xdg-screenshot
        let result = std::process::Command::new("import")
            .args(["-window", "root", "-crop", &format!("{}x{}+{}+{}", width, height, x, y),
                   &filepath.to_string_lossy()])
            .output();
        if result.is_err() || !result.unwrap().status.success() {
            // Fallback: use gnome-screenshot
            let output = std::process::Command::new("gnome-screenshot")
                .args(["-f", &filepath.to_string_lossy()])
                .output()
                .map_err(|e| format!("gnome-screenshot: {}", e))?;
            if !output.status.success() {
                return Err("No screenshot tool available (install imagemagick or gnome-screenshot)".into());
            }
        }
    }

    let path_str = filepath.to_string_lossy().replace('\\', "/");
    let markdown = format!("![]({})", path_str);
    let idea = db.add_idea(&markdown)?;
    Ok(serde_json::to_string(&idea).unwrap_or_default())
}
