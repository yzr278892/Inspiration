use crate::commands::{SyncPayload, SyncResult, WebdavConfig};
use crate::db::{Database, Idea, Tag};

pub async fn sync_with_webdav(config: &WebdavConfig, db: &Database) -> Result<SyncResult, String> {
    let remote_url = format!("{}/inspiration-sync.json", config.url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut errors: Vec<String> = Vec::new();

    let (local_ideas, local_tags) = db.get_all_for_sync()?;
    let local_idea_tags = db.get_all_idea_tags()?;

    let remote_payload: Option<SyncPayload> = match client
        .get(&remote_url)
        .basic_auth(&config.username, Some(&config.password))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => match resp.json().await {
            Ok(p) => Some(p),
            Err(e) => {
                errors.push(format!("Remote parse error: {}", e));
                None
            }
        },
        Ok(resp) if resp.status().as_u16() == 404 => None,
        Ok(resp) => {
            errors.push(format!("Remote fetch returned {}", resp.status()));
            None
        }
        Err(e) => {
            errors.push(format!("Remote fetch error: {}", e));
            None
        }
    };

    let mut merged_ideas: Vec<Idea> = Vec::new();

    let mut local_by_id: std::collections::HashMap<i64, Idea> = std::collections::HashMap::new();
    for idea in &local_ideas {
        local_by_id.insert(idea.id, idea.clone());
    }

    if let Some(ref remote) = remote_payload {
        let mut remote_by_id: std::collections::HashMap<i64, Idea> = std::collections::HashMap::new();
        for idea in &remote.ideas {
            remote_by_id.insert(idea.id, idea.clone());
        }

        let mut all_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
        for id in local_by_id.keys() { all_ids.insert(*id); }
        for id in remote_by_id.keys() { all_ids.insert(*id); }

        for id in all_ids {
            let merged = match (local_by_id.get(&id), remote_by_id.get(&id)) {
                (Some(local), Some(remote)) => {
                    if remote.updated_at > local.updated_at { remote.clone() }
                    else { local.clone() }
                }
                (Some(local), None) => local.clone(),
                (None, Some(remote)) => remote.clone(),
                (None, None) => continue,
            };
            merged_ideas.push(merged);
        }
    } else {
        for idea in &local_ideas {
            merged_ideas.push(idea.clone());
        }
    }

    let mut merged_tags: Vec<Tag> = Vec::new();
    let mut tag_names = std::collections::HashSet::new();
    for tag in &local_tags {
        if tag_names.insert(tag.name.clone()) {
            merged_tags.push(tag.clone());
        }
    }
    if let Some(ref remote) = remote_payload {
        for tag in &remote.tags {
            if tag_names.insert(tag.name.clone()) {
                merged_tags.push(tag.clone());
            }
        }
    }

    let mut merged_idea_tags: std::collections::HashSet<(i64, i64)> = std::collections::HashSet::new();
    for (idea_id, tag_id) in &local_idea_tags {
        merged_idea_tags.insert((*idea_id, *tag_id));
    }
    if let Some(ref remote) = remote_payload {
        for &(idea_id, tag_id) in &remote.idea_tags {
            merged_idea_tags.insert((idea_id, tag_id));
        }
    }

    let downloaded = merged_ideas.len().saturating_sub(local_ideas.len());
    let uploaded = local_ideas.len();

    let payload = SyncPayload {
        ideas: merged_ideas,
        tags: merged_tags,
        idea_tags: merged_idea_tags.into_iter().collect(),
        exported_at: chrono::Utc::now().to_rfc3339(),
    };

    let json = serde_json::to_string(&payload).map_err(|e| format!("Serialize: {}", e))?;

    match client
        .put(&remote_url)
        .basic_auth(&config.username, Some(&config.password))
        .header("Content-Type", "application/json")
        .body(json)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {}
        Ok(resp) => {
            errors.push(format!("Upload returned {}", resp.status()));
        }
        Err(e) => {
            errors.push(format!("Upload error: {}", e));
        }
    }

    // Apply remote data to local DB (newer remote items overwrite local)
    if let Some(ref remote) = remote_payload {
        db.apply_sync(&remote.ideas, &remote.tags)?;
    }

    Ok(SyncResult {
        uploaded,
        downloaded: if remote_payload.is_some() { downloaded } else { 0 },
        errors,
    })
}
