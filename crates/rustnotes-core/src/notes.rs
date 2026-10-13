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

/// Schema shared by the on-disk store and the in-memory fallback.
const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    body TEXT NOT NULL,
    updated INTEGER NOT NULL
)";

impl NotesDb {
    pub fn open(data_dir: &Path) -> rusqlite::Result<NotesDb> {
        let _ = std::fs::create_dir_all(data_dir);
        let conn = Connection::open(data_dir.join("notes.db"))?;
        conn.execute(SCHEMA, [])?;
        Ok(NotesDb { conn })
    }

    /// An always-empty store for when the on-disk database cannot be opened
    /// (see [`crate::App::new`]). Only an out-of-memory process fails here.
    pub(crate) fn open_in_memory() -> NotesDb {
        let conn = Connection::open_in_memory().expect("open in-memory notes store");
        conn.execute(SCHEMA, [])
            .expect("create in-memory notes table");
        NotesDb { conn }
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

    /// Persist `note`. Errors are logged, not swallowed: a save that failed
    /// silently looks exactly like one that worked, which is how notes get
    /// lost. `id` and `updated` only change when the row really landed.
    pub fn save(&self, note: &mut Note) {
        let now = util::now_epoch();
        let result = if note.id == 0 {
            self.conn
                .execute(
                    "INSERT INTO notes (body, updated) VALUES (?1, ?2)",
                    params![note.body, now],
                )
                .map(|_| {
                    note.id = self.conn.last_insert_rowid();
                })
        } else {
            self.conn
                .execute(
                    "UPDATE notes SET body=?1, updated=?2 WHERE id=?3",
                    params![note.body, now, note.id],
                )
                .map(|_| ())
        };
        match result {
            Ok(()) => note.updated = now,
            Err(e) => log::error!("notes: save id={} failed: {e}", note.id),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh temp dir, so the reopen test exercises a real on-disk file.
    fn temp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("rn-notes-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn note(body: &str) -> Note {
        Note {
            id: 0,
            body: body.to_owned(),
            updated: 0,
        }
    }

    #[test]
    fn save_survives_reopen() {
        let dir = temp("roundtrip");
        let mut n = note("first line\nsecond line");
        {
            let db = NotesDb::open(&dir).unwrap();
            db.save(&mut n);
            assert_ne!(n.id, 0, "insert assigns the row id");
            assert!(n.updated > 0, "insert stamps updated");
        }
        let db = NotesDb::open(&dir).unwrap();
        let notes = db.list();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id, n.id);
        assert_eq!(notes[0].body, "first line\nsecond line");
        assert_eq!(notes[0].updated, n.updated);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn title_is_the_first_line() {
        assert_eq!(title_of("Hello\nworld"), "Hello");
        assert_eq!(title_of("  padded  \nbody"), "padded");
        assert_eq!(title_of(""), "(untitled)");
        assert_eq!(title_of("\nstarts on line two"), "(untitled)");
    }

    #[test]
    fn snippet_is_the_first_non_empty_line_after_the_title() {
        assert_eq!(snippet_of("Title\nbody line"), Some("body line"));
        assert_eq!(snippet_of("Title\n\n  spaced  \nmore"), Some("spaced"));
        assert_eq!(snippet_of("Title"), None);
        assert_eq!(snippet_of("Title\n\n"), None);
    }

    #[test]
    fn save_bumps_updated_only_when_the_write_lands() {
        let db = NotesDb::open_in_memory();

        // A failed insert must not invent an id or stamp updated.
        db.conn.execute("DROP TABLE notes", []).unwrap();
        let mut n = note("doomed insert");
        db.save(&mut n);
        assert_eq!(
            (n.id, n.updated),
            (0, 0),
            "failed insert leaves the note as-is"
        );
        db.conn.execute(SCHEMA, []).unwrap();

        // A successful update re-stamps updated past a seeded old value.
        db.conn
            .execute(
                "INSERT INTO notes (body, updated) VALUES (?1, ?2)",
                params!["old body", 1000],
            )
            .unwrap();
        let mut m = db
            .list()
            .into_iter()
            .find(|x| x.body == "old body")
            .unwrap();
        assert_eq!(m.updated, 1000);
        m.body = "new body".to_owned();
        db.save(&mut m);
        assert!(m.updated > 1000, "successful update re-stamps updated");

        // A failed update must leave updated where the last success put it.
        let stamped = m.updated;
        db.conn.execute("DROP TABLE notes", []).unwrap();
        m.body = "doomed update".to_owned();
        db.save(&mut m);
        assert_eq!(m.updated, stamped, "failed update must not bump updated");
    }

    #[test]
    fn delete_removes_only_its_note() {
        let db = NotesDb::open_in_memory();
        let mut a = note("a");
        let mut b = note("b");
        db.save(&mut a);
        db.save(&mut b);

        db.delete(a.id);
        let left = db.list();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].body, "b");

        db.delete(a.id); // already gone: a no-op, not an error
        assert_eq!(db.list().len(), 1);
    }

    #[test]
    fn list_orders_most_recent_first() {
        let db = NotesDb::open_in_memory();
        for (body, updated) in [("old", 100), ("newest", 300), ("middle", 200)] {
            db.conn
                .execute(
                    "INSERT INTO notes (body, updated) VALUES (?1, ?2)",
                    params![body, updated],
                )
                .unwrap();
        }
        let notes = db.list();
        let bodies: Vec<&str> = notes.iter().map(|n| n.body.as_str()).collect();
        assert_eq!(bodies, ["newest", "middle", "old"]);
    }
}
