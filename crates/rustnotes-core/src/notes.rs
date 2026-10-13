//! Note storage — SQLite via rusqlite (bundled), in the app's internal data dir.

use crate::util;
use rusqlite::{params, Connection};
use std::path::Path;

#[derive(Clone)]
pub struct Note {
    pub id: i64,
    pub body: String,
    pub updated: i64,
}

pub struct NotesDb {
    conn: Connection,
}

impl NotesDb {
    pub fn open(data_dir: &Path) -> rusqlite::Result<NotesDb> {
        let _ = std::fs::create_dir_all(data_dir);
        let conn = Connection::open(data_dir.join("notes.db"))?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                body TEXT NOT NULL,
                updated INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(NotesDb { conn })
    }

    pub fn list(&self) -> Vec<Note> {
        let mut stmt = match self
            .conn
            .prepare("SELECT id, body, updated FROM notes ORDER BY updated DESC")
        {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map([], |row| {
            Ok(Note {
                id: row.get(0)?,
                body: row.get(1)?,
                updated: row.get(2)?,
            })
        });
        match rows {
            Ok(it) => it.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn save(&self, note: &mut Note) {
        let now = util::now_epoch();
        if note.id == 0 {
            let ok = self.conn.execute(
                "INSERT INTO notes (body, updated) VALUES (?1, ?2)",
                params![note.body, now],
            );
            if ok.is_ok() {
                note.id = self.conn.last_insert_rowid();
            }
        } else {
            let _ = self.conn.execute(
                "UPDATE notes SET body=?1, updated=?2 WHERE id=?3",
                params![note.body, now, note.id],
            );
        }
        note.updated = now;
    }

    pub fn delete(&self, id: i64) {
        let _ = self
            .conn
            .execute("DELETE FROM notes WHERE id=?1", params![id]);
    }
}

/// First non-empty line, trimmed; notes are titled by their first line.
pub fn title_of(body: &str) -> &str {
    let t = body.lines().next().unwrap_or("").trim();
    if t.is_empty() {
        "(untitled)"
    } else {
        t
    }
}

/// First non-empty line after the title, if any.
pub fn snippet_of(body: &str) -> Option<&str> {
    body.lines().skip(1).map(str::trim).find(|l| !l.is_empty())
}
