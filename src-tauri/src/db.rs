use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idea {
    pub id: i64,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_todo: bool,
    pub todo_done: bool,
    pub deleted: bool,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn init(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Create dir: {}", e))?;
        }
        let conn = Connection::open(&path).map_err(|e| format!("Open DB: {}", e))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("Pragma: {}", e))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ideas (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                content     TEXT NOT NULL,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL,
                is_todo     INTEGER NOT NULL DEFAULT 0,
                todo_done   INTEGER NOT NULL DEFAULT 0,
                deleted     INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS tags (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE COLLATE NOCASE
            );
            CREATE TABLE IF NOT EXISTS idea_tags (
                idea_id INTEGER NOT NULL REFERENCES ideas(id) ON DELETE CASCADE,
                tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                PRIMARY KEY (idea_id, tag_id)
            );
            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_ideas_created ON ideas(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_ideas_deleted ON ideas(deleted);
            CREATE INDEX IF NOT EXISTS idx_idea_tags_tag ON idea_tags(tag_id);
            CREATE INDEX IF NOT EXISTS idx_idea_tags_idea ON idea_tags(idea_id);",
        )
        .map_err(|e| format!("Migrate: {}", e))?;

        Ok(Database { conn: Mutex::new(conn) })
    }

    pub fn add_idea(&self, content: &str) -> Result<Idea, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO ideas (content, created_at, updated_at) VALUES (?1, ?2, ?3)",
            params![content, now, now],
        )
        .map_err(|e| format!("Insert: {}", e))?;
        let id = conn.last_insert_rowid();
        Ok(Idea {
            id,
            content: content.to_string(),
            created_at: now.clone(),
            updated_at: now,
            is_todo: false,
            todo_done: false,
            deleted: false,
            tags: vec![],
        })
    }

    pub fn get_ideas(
        &self,
        search: Option<&str>,
        tag_ids: &[i64],
        sort_desc: bool,
    ) -> Result<Vec<Idea>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;

        let mut sql = String::from(
            "SELECT i.id, i.content, i.created_at, i.updated_at, i.is_todo, i.todo_done, i.deleted
             FROM ideas i WHERE i.deleted = 0",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(q) = search {
            if !q.is_empty() {
                sql.push_str(" AND i.content LIKE ?");
                param_values.push(Box::new(format!("%{}%", q)));
            }
        }

        if !tag_ids.is_empty() {
            let placeholders: Vec<&str> = vec!["?"; tag_ids.len()];
            sql.push_str(&format!(
                " AND i.id IN (SELECT idea_id FROM idea_tags WHERE tag_id IN ({})
                 GROUP BY idea_id HAVING COUNT(DISTINCT tag_id) = ?)",
                placeholders.join(",")
            ));
            for &tid in tag_ids {
                param_values.push(Box::new(tid));
            }
            param_values.push(Box::new(tag_ids.len() as i64));
        }

        sql.push_str(" ORDER BY i.created_at ");
        sql.push_str(if sort_desc { "DESC" } else { "ASC" });

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("Prepare: {}", e))?;

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let idea_rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(Idea {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    is_todo: row.get::<_, i32>(4)? != 0,
                    todo_done: row.get::<_, i32>(5)? != 0,
                    deleted: row.get::<_, i32>(6)? != 0,
                    tags: vec![],
                })
            })
            .map_err(|e| format!("Query: {}", e))?;

        let mut ideas: Vec<Idea> = Vec::new();
        for row in idea_rows {
            ideas.push(row.map_err(|e| format!("Row: {}", e))?);
        }

        // Batch-fetch tags for all ideas
        if !ideas.is_empty() {
            let ids: Vec<String> = ideas.iter().map(|i| i.id.to_string()).collect();
            let tag_sql = format!(
                "SELECT it.idea_id, t.id, t.name FROM idea_tags it
                 JOIN tags t ON t.id = it.tag_id
                 WHERE it.idea_id IN ({})",
                ids.join(",")
            );
            let mut tag_stmt = conn.prepare(&tag_sql).map_err(|e| format!("Tag prepare: {}", e))?;
            let tag_rows = tag_stmt
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?))
                })
                .map_err(|e| format!("Tag query: {}", e))?;

            let mut tag_map: std::collections::HashMap<i64, Vec<Tag>> = std::collections::HashMap::new();
            for row in tag_rows {
                let (idea_id, tag_id, tag_name): (i64, i64, String) =
                    row.map_err(|e| format!("Tag row: {}", e))?;
                tag_map.entry(idea_id).or_default().push(Tag {
                    id: tag_id,
                    name: tag_name,
                    count: None,
                });
            }
            for idea in &mut ideas {
                idea.tags = tag_map.remove(&idea.id).unwrap_or_default();
            }
        }

        Ok(ideas)
    }

    pub fn get_idea(&self, id: i64) -> Result<Idea, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT id, content, created_at, updated_at, is_todo, todo_done, deleted FROM ideas WHERE id=?")
            .map_err(|e| format!("Prepare: {}", e))?;
        let mut idea = stmt
            .query_row(params![id], |row| {
                Ok(Idea {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    is_todo: row.get::<_, i32>(4)? != 0,
                    todo_done: row.get::<_, i32>(5)? != 0,
                    deleted: row.get::<_, i32>(6)? != 0,
                    tags: vec![],
                })
            })
            .map_err(|e| format!("Get idea {}: {}", id, e))?;

        let mut tag_stmt = conn
            .prepare("SELECT t.id, t.name FROM idea_tags it JOIN tags t ON t.id = it.tag_id WHERE it.idea_id=?")
            .map_err(|e| format!("Tag prepare: {}", e))?;
        let tag_rows = tag_stmt
            .query_map(params![id], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    count: None,
                })
            })
            .map_err(|e| format!("Tag query: {}", e))?;
        for row in tag_rows {
            idea.tags.push(row.map_err(|e| format!("Tag row: {}", e))?);
        }
        Ok(idea)
    }

    pub fn update_idea(&self, id: i64, content: &str) -> Result<Idea, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE ideas SET content=?1, updated_at=?2 WHERE id=?3",
            params![content, now, id],
        )
        .map_err(|e| format!("Update: {}", e))?;
        drop(conn);
        self.get_idea(id)
    }

    pub fn soft_delete_idea(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        conn.execute("UPDATE ideas SET deleted=1, updated_at=?1 WHERE id=?2",
            params![chrono::Utc::now().to_rfc3339(), id])
            .map_err(|e| format!("Delete: {}", e))?;
        Ok(())
    }

    pub fn toggle_todo(&self, id: i64) -> Result<Idea, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE ideas SET is_todo=1, todo_done=0, updated_at=?1 WHERE id=?2 AND is_todo=0",
            params![now, id],
        )
        .map_err(|e| format!("Toggle todo: {}", e))?;
        drop(conn);
        self.get_idea(id)
    }

    pub fn toggle_todo_done(&self, id: i64) -> Result<Idea, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE ideas SET todo_done=1 - todo_done, updated_at=?1 WHERE id=?2",
            params![now, id],
        )
        .map_err(|e| format!("Toggle done: {}", e))?;
        drop(conn);
        self.get_idea(id)
    }

    pub fn add_tag(&self, idea_id: i64, name: &str) -> Result<Tag, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        let name = name.trim();
        if name.is_empty() {
            return Err("Tag name cannot be empty".into());
        }
        conn.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", params![name])
            .map_err(|e| format!("Insert tag: {}", e))?;
        let tag_id: i64 = conn
            .query_row("SELECT id FROM tags WHERE name=?1", params![name], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Get tag id: {}", e))?;
        conn.execute(
            "INSERT OR IGNORE INTO idea_tags (idea_id, tag_id) VALUES (?1, ?2)",
            params![idea_id, tag_id],
        )
        .map_err(|e| format!("Link tag: {}", e))?;
        Ok(Tag {
            id: tag_id,
            name: name.to_string(),
            count: None,
        })
    }

    pub fn remove_tag(&self, idea_id: i64, tag_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        conn.execute(
            "DELETE FROM idea_tags WHERE idea_id=?1 AND tag_id=?2",
            params![idea_id, tag_id],
        )
        .map_err(|e| format!("Remove tag: {}", e))?;
        Ok(())
    }

    pub fn get_tags(&self) -> Result<Vec<Tag>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.name, COUNT(it.idea_id) as cnt
                 FROM tags t
                 LEFT JOIN idea_tags it ON t.id = it.tag_id
                 LEFT JOIN ideas i ON i.id = it.idea_id AND i.deleted = 0
                 GROUP BY t.id
                 ORDER BY cnt DESC, t.name ASC",
            )
            .map_err(|e| format!("Prepare: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    count: Some(row.get(2)?),
                })
            })
            .map_err(|e| format!("Query: {}", e))?;
        let mut tags = Vec::new();
        for row in rows {
            tags.push(row.map_err(|e| format!("Row: {}", e))?);
        }
        Ok(tags)
    }

    pub fn save_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| format!("Save setting: {}", e))?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        let result: Result<String, _> = conn.query_row(
            "SELECT value FROM settings WHERE key=?1",
            params![key],
            |row| row.get(0),
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Get setting: {}", e)),
        }
    }

    pub fn get_all_for_sync(&self) -> Result<(Vec<Idea>, Vec<Tag>), String> {
        let ideas = self.get_ideas(None, &[], true)?;
        let tags = self.get_tags()?;
        Ok((ideas, tags))
    }

    pub fn apply_sync(&self, remote_ideas: &[Idea], remote_tags: &[Tag]) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;

        for idea in remote_ideas {
            let existing: Result<Idea, _> = {
                let mut stmt = conn
                    .prepare("SELECT id, content, created_at, updated_at, is_todo, todo_done, deleted FROM ideas WHERE id=?")
                    .map_err(|e| format!("Prepare: {}", e))?;
                stmt.query_row(params![idea.id], |row| {
                    Ok(Idea {
                        id: row.get(0)?,
                        content: row.get(1)?,
                        created_at: row.get(2)?,
                        updated_at: row.get(3)?,
                        is_todo: row.get::<_, i32>(4)? != 0,
                        todo_done: row.get::<_, i32>(5)? != 0,
                        deleted: row.get::<_, i32>(6)? != 0,
                        tags: vec![],
                    })
                })
            };

            match existing {
                Ok(local) if local.updated_at >= idea.updated_at => {
                    continue; // Local is newer or same, skip
                }
                _ => {
                    // Remote is newer or idea doesn't exist locally
                    conn.execute(
                        "INSERT OR REPLACE INTO ideas (id, content, created_at, updated_at, is_todo, todo_done, deleted)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            idea.id,
                            idea.content,
                            idea.created_at,
                            idea.updated_at,
                            idea.is_todo as i32,
                            idea.todo_done as i32,
                            idea.deleted as i32,
                        ],
                    )
                    .map_err(|e| format!("Sync insert: {}", e))?;
                }
            }
        }

        for tag in remote_tags {
            conn.execute("INSERT OR IGNORE INTO tags (id, name) VALUES (?1, ?2)",
                params![tag.id, tag.name])
                .map_err(|e| format!("Sync tag: {}", e))?;
        }

        Ok(())
    }

    pub fn get_all_idea_tags(&self) -> Result<Vec<(i64, i64)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT idea_id, tag_id FROM idea_tags ORDER BY idea_id")
            .map_err(|e| format!("Prepare: {}", e))?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
            .map_err(|e| format!("Query: {}", e))?;
        let mut result = Vec::new();
        for row in rows {
            let (idea_id, tag_id): (i64, i64) = row.map_err(|e| format!("Row: {}", e))?;
            result.push((idea_id, tag_id));
        }
        Ok(result)
    }

    pub fn insert_idea_tag(&self, idea_id: i64, tag_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        conn.execute(
            "INSERT OR IGNORE INTO idea_tags (idea_id, tag_id) VALUES (?1, ?2)",
            params![idea_id, tag_id],
        )
        .map_err(|e| format!("Insert idea_tag: {}", e))?;
        Ok(())
    }
}
