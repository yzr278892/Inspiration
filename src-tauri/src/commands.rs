use crate::db::{Database, Idea, Tag};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

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
        .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".into());
    let model = db
        .get_setting("ai_model")?
        .unwrap_or_else(|| "gpt-4o-mini".into());
    Ok(AIConfig {
        api_key,
        endpoint,
        model,
    })
}

async fn call_ai_api(config: &AIConfig, content: &str, existing_tags: &[String]) -> Result<RewriteResult, String> {
    let client = reqwest::Client::new();

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
        .post(&config.endpoint)
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
                .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".into());
            let model = db
                .get_setting("ai_model")?
                .unwrap_or_else(|| "gpt-4o-mini".into());
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
