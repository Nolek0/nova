//! Stockage local SQLite : conversations, messages (recherche plein texte), réglages,
//! mémoire volontaire, journal d'activité, cache météo. Tout reste sur la machine.

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attachment {
    pub id: String,
    pub kind: String, // image | pdf | text | other
    pub filename: String,
    pub path: String,
    #[serde(default)]
    pub extracted_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Meta {
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub left_device: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub files: Vec<crate::tools::files::FileHit>,
    #[serde(default)]
    pub action: Option<crate::tools::actions::PendingAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Source {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String, // user | assistant
    pub content: String,
    pub created_at: f64,
    pub status: String, // complete | streaming | interrupted | failed
    pub attachments: Vec<Attachment>,
    pub meta: Meta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: f64,
    pub updated_at: f64,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub message_id: String,
    pub conversation_id: String,
    pub conversation_title: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryItem {
    pub id: i64,
    pub content: String,
    pub created_at: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivityEntry {
    pub id: i64,
    pub date: f64,
    pub tool: String,
    pub summary: String,
    pub risk: String,
    pub outcome: String,
}

pub fn now() -> f64 {
    chrono::Utc::now().timestamp_millis() as f64 / 1000.0
}

pub struct Db {
    pub conn: Connection,
}

impl Db {
    pub fn open(path: Option<&Path>) -> Result<Db> {
        let conn = match path {
            Some(p) => {
                if let Some(dir) = p.parent() {
                    std::fs::create_dir_all(dir)?;
                }
                Connection::open(p)?
            }
            None => Connection::open_in_memory()?,
        };
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, created_at REAL NOT NULL,
                updated_at REAL NOT NULL, pinned INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
                role TEXT NOT NULL, content TEXT NOT NULL, created_at REAL NOT NULL, status TEXT NOT NULL,
                attachments TEXT NOT NULL DEFAULT '[]', meta TEXT NOT NULL DEFAULT '{}');
            CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id, created_at);
            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                content, message_id UNINDEXED, conversation_id UNINDEXED,
                tokenize = 'unicode61 remove_diacritics 2');
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS memories (id INTEGER PRIMARY KEY AUTOINCREMENT, content TEXT NOT NULL, created_at REAL NOT NULL);
            CREATE TABLE IF NOT EXISTS activity_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT, date REAL NOT NULL, tool TEXT NOT NULL,
                summary TEXT NOT NULL, risk TEXT NOT NULL, outcome TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS cache (key TEXT PRIMARY KEY, value TEXT NOT NULL, fetched_at REAL NOT NULL);
            "#,
        )?;
        Ok(Db { conn })
    }

    // ---- Conversations

    pub fn conversations(&self) -> Result<Vec<Conversation>> {
        let mut st = self.conn.prepare("SELECT id, title, created_at, updated_at, pinned FROM conversations ORDER BY pinned DESC, updated_at DESC")?;
        let rows = st.query_map([], |r| {
            Ok(Conversation { id: r.get(0)?, title: r.get(1)?, created_at: r.get(2)?, updated_at: r.get(3)?, pinned: r.get::<_, i64>(4)? != 0 })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn save_conversation(&self, c: &Conversation) -> Result<()> {
        self.conn.execute(
            "INSERT INTO conversations (id, title, created_at, updated_at, pinned) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET title = excluded.title, updated_at = excluded.updated_at, pinned = excluded.pinned",
            params![c.id, c.title, c.created_at, c.updated_at, c.pinned as i64],
        )?;
        Ok(())
    }

    pub fn conversation(&self, id: &str) -> Result<Option<Conversation>> {
        Ok(self
            .conn
            .query_row("SELECT id, title, created_at, updated_at, pinned FROM conversations WHERE id = ?1", [id], |r| {
                Ok(Conversation { id: r.get(0)?, title: r.get(1)?, created_at: r.get(2)?, updated_at: r.get(3)?, pinned: r.get::<_, i64>(4)? != 0 })
            })
            .optional()?)
    }

    pub fn delete_conversation(&self, id: &str) -> Result<Vec<Attachment>> {
        let atts: Vec<Attachment> = self.messages(id)?.into_iter().flat_map(|m| m.attachments).collect();
        self.conn.execute("DELETE FROM messages_fts WHERE conversation_id = ?1", [id])?;
        self.conn.execute("DELETE FROM messages WHERE conversation_id = ?1", [id])?;
        self.conn.execute("DELETE FROM conversations WHERE id = ?1", [id])?;
        Ok(atts)
    }

    // ---- Messages

    pub fn messages(&self, conversation_id: &str) -> Result<Vec<Message>> {
        let mut st = self.conn.prepare(
            "SELECT id, conversation_id, role, content, created_at, status, attachments, meta FROM messages
             WHERE conversation_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = st.query_map([conversation_id], |r| {
            let atts: String = r.get(6)?;
            let meta: String = r.get(7)?;
            Ok(Message {
                id: r.get(0)?,
                conversation_id: r.get(1)?,
                role: r.get(2)?,
                content: r.get(3)?,
                created_at: r.get(4)?,
                status: r.get(5)?,
                attachments: serde_json::from_str(&atts).unwrap_or_default(),
                meta: serde_json::from_str(&meta).unwrap_or_default(),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn save_message(&self, m: &Message) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO messages (id, conversation_id, role, content, created_at, status, attachments, meta)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET content = excluded.content, status = excluded.status,
                attachments = excluded.attachments, meta = excluded.meta",
            params![
                m.id,
                m.conversation_id,
                m.role,
                m.content,
                m.created_at,
                m.status,
                serde_json::to_string(&m.attachments)?,
                serde_json::to_string(&m.meta)?
            ],
        )?;
        tx.execute("DELETE FROM messages_fts WHERE message_id = ?1", [&m.id])?;
        if m.status != "streaming" && !m.content.is_empty() {
            tx.execute("INSERT INTO messages_fts (content, message_id, conversation_id) VALUES (?1, ?2, ?3)", params![m.content, m.id, m.conversation_id])?;
        }
        tx.execute("UPDATE conversations SET updated_at = ?1 WHERE id = ?2", params![now(), m.conversation_id])?;
        tx.commit()?;
        Ok(())
    }

    /// Supprime ce message et tous les suivants (modifier / régénérer).
    pub fn delete_messages_from(&self, conversation_id: &str, message_id: &str) -> Result<()> {
        let all = self.messages(conversation_id)?;
        if let Some(idx) = all.iter().position(|m| m.id == message_id) {
            for m in &all[idx..] {
                self.conn.execute("DELETE FROM messages_fts WHERE message_id = ?1", [&m.id])?;
                self.conn.execute("DELETE FROM messages WHERE id = ?1", [&m.id])?;
            }
        }
        Ok(())
    }

    pub fn mark_stale_streaming(&self) -> Result<()> {
        self.conn.execute("UPDATE messages SET status = 'interrupted' WHERE status = 'streaming'", [])?;
        Ok(())
    }

    pub fn search(&self, text: &str, limit: i64) -> Result<Vec<SearchHit>> {
        let terms: Vec<String> = text.split_whitespace().map(|t| format!("\"{}\"*", t.replace('"', ""))).collect();
        if terms.is_empty() {
            return Ok(vec![]);
        }
        let mut st = self.conn.prepare(
            "SELECT f.message_id, f.conversation_id, c.title, snippet(messages_fts, 0, '«', '»', '…', 12)
             FROM messages_fts f JOIN conversations c ON c.id = f.conversation_id
             WHERE messages_fts MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let rows = st.query_map(params![terms.join(" "), limit], |r| {
            Ok(SearchHit { message_id: r.get(0)?, conversation_id: r.get(1)?, conversation_title: r.get(2)?, snippet: r.get(3)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn all_attachments(&self) -> Result<Vec<(Attachment, String, f64)>> {
        let mut st = self.conn.prepare("SELECT conversation_id, created_at, attachments FROM messages WHERE attachments != '[]' ORDER BY created_at DESC")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, String>(2)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (cid, date, json) = row?;
            for a in serde_json::from_str::<Vec<Attachment>>(&json).unwrap_or_default() {
                out.push((a, cid.clone(), date));
            }
        }
        Ok(out)
    }

    // ---- Réglages

    pub fn setting(&self, key: &str) -> Option<String> {
        self.conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional().ok().flatten()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn all_settings(&self) -> Result<std::collections::HashMap<String, String>> {
        let mut st = self.conn.prepare("SELECT key, value FROM settings")?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ---- Mémoire

    pub fn memories(&self) -> Result<Vec<MemoryItem>> {
        let mut st = self.conn.prepare("SELECT id, content, created_at FROM memories ORDER BY created_at DESC")?;
        let rows = st.query_map([], |r| Ok(MemoryItem { id: r.get(0)?, content: r.get(1)?, created_at: r.get(2)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn add_memory(&self, content: &str) -> Result<()> {
        self.conn.execute("INSERT INTO memories (content, created_at) VALUES (?1, ?2)", params![content, now()])?;
        Ok(())
    }

    pub fn delete_memory(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM memories WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---- Journal

    pub fn log(&self, tool: &str, summary: &str, risk: &str, outcome: &str) {
        let _ = self.conn.execute(
            "INSERT INTO activity_log (date, tool, summary, risk, outcome) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![now(), tool, summary, risk, outcome],
        );
    }

    pub fn activity(&self, limit: i64) -> Result<Vec<ActivityEntry>> {
        let mut st = self.conn.prepare("SELECT id, date, tool, summary, risk, outcome FROM activity_log ORDER BY date DESC LIMIT ?1")?;
        let rows = st.query_map([limit], |r| {
            Ok(ActivityEntry { id: r.get(0)?, date: r.get(1)?, tool: r.get(2)?, summary: r.get(3)?, risk: r.get(4)?, outcome: r.get(5)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn clear_activity(&self) -> Result<()> {
        self.conn.execute("DELETE FROM activity_log", [])?;
        Ok(())
    }

    // ---- Cache (météo : dernière donnée connue, horodatée)

    pub fn cache_get(&self, key: &str) -> Option<(String, f64)> {
        self.conn.query_row("SELECT value, fetched_at FROM cache WHERE key = ?1", [key], |r| Ok((r.get(0)?, r.get(1)?))).optional().ok().flatten()
    }

    pub fn cache_put(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO cache (key, value, fetched_at) VALUES (?1, ?2, ?3) ON CONFLICT(key) DO UPDATE SET value = excluded.value, fetched_at = excluded.fetched_at",
            params![key, value, now()],
        )?;
        Ok(())
    }

    // ---- Export / suppression

    pub fn export_all(&self) -> Result<serde_json::Value> {
        let mut convs = Vec::new();
        for c in self.conversations()? {
            let msgs: Vec<serde_json::Value> = self
                .messages(&c.id)?
                .into_iter()
                .map(|m| serde_json::json!({"role": m.role, "content": m.content, "date": m.created_at, "attachments": m.attachments.iter().map(|a| a.filename.clone()).collect::<Vec<_>>()}))
                .collect();
            convs.push(serde_json::json!({"title": c.title, "created": c.created_at, "pinned": c.pinned, "messages": msgs}));
        }
        let mems: Vec<String> = self.memories()?.into_iter().map(|m| m.content).collect();
        Ok(serde_json::json!({"exported": now(), "conversations": convs, "memories": mems}))
    }

    pub fn delete_all_conversations(&self) -> Result<()> {
        self.conn.execute_batch("DELETE FROM messages_fts; DELETE FROM messages; DELETE FROM conversations;")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(cid: &str, role: &str, content: &str, t: f64) -> Message {
        Message {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: cid.into(),
            role: role.into(),
            content: content.into(),
            created_at: t,
            status: "complete".into(),
            attachments: vec![],
            meta: Meta::default(),
        }
    }

    #[test]
    fn roundtrip_search_and_truncate() {
        let db = Db::open(None).unwrap();
        let c = Conversation { id: "c1".into(), title: "Chimie".into(), created_at: 1.0, updated_at: 1.0, pinned: false };
        db.save_conversation(&c).unwrap();
        let m1 = msg("c1", "user", "Quelle est la masse molaire de l'éthanol ?", 1.0);
        let m2 = msg("c1", "assistant", "46 g/mol", 2.0);
        db.save_message(&m1).unwrap();
        db.save_message(&m2).unwrap();
        assert_eq!(db.messages("c1").unwrap().len(), 2);
        assert_eq!(db.search("ethanol", 10).unwrap().len(), 1, "accents ignorés");
        assert_eq!(db.search("mola", 10).unwrap().len(), 1, "préfixes");
        db.delete_messages_from("c1", &m2.id).unwrap();
        assert_eq!(db.messages("c1").unwrap().len(), 1);
        assert!(db.search("46", 10).unwrap().is_empty());
        db.delete_conversation("c1").unwrap();
        assert!(db.conversations().unwrap().is_empty());
        assert!(db.search("ethanol", 10).unwrap().is_empty());
    }

    #[test]
    fn settings_memory_cache() {
        let db = Db::open(None).unwrap();
        db.set_setting("theme", "dark").unwrap();
        db.set_setting("theme", "light").unwrap();
        assert_eq!(db.setting("theme").as_deref(), Some("light"));
        db.add_memory("Je préfère le système métrique").unwrap();
        assert_eq!(db.memories().unwrap().len(), 1);
        db.cache_put("weather:Paris", "{}").unwrap();
        assert!(db.cache_get("weather:Paris").is_some());
        db.log("math", "2+2", "free", "ok");
        assert_eq!(db.activity(10).unwrap()[0].tool, "math");
    }
}
