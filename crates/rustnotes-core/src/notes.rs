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
        Self::init(conn)
    }

    /// Ephemeral fallback for when the real database cannot be opened: the
    /// app still runs (editor works, list is empty) but nothing persists.
    /// See HIGH-3 in the review board.
    pub fn open_in_memory() -> rusqlite::Result<NotesDb> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<NotesDb> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                body TEXT NOT NULL,
                updated INTEGER NOT NULL
            )",
            [],
        )?;
        // Schema version stamp (v1). The first real migration hangs off
        // this: read the version, step forward one migration at a time,
        // write the new version back.
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version == 0 {
            conn.execute_batch("PRAGMA user_version = 1;")?;
        } else if version > 1 {
            log::warn!(
                "notes.db schema version {version} is newer than this build understands (1)"
            );
        }
        Ok(NotesDb { conn })
    }

    /// Rows for the list UI: id, updated and only a body *prefix* — the
    /// editor fetches the full body with [`NotesDb::get`]. Ties on the
    /// second-resolution `updated` break toward the newer id (MED-3).
    /// Errors are propagated so the caller can tell "no notes" from
    /// "couldn't read" (HIGH-1); rows that cannot be decoded (e.g. a
    /// non-UTF-8 body) are logged and skipped, not silently dropped.
    pub fn list(&self) -> rusqlite::Result<Vec<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, substr(body,1,120), updated FROM notes ORDER BY updated DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Note {
                id: row.get(0)?,
                body: row.get(1)?,
                updated: row.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            match row {
                Ok(note) => out.push(note),
                Err(e) => log::error!("skipping unreadable note row: {e}"),
            }
        }
        Ok(out)
    }

    /// The full note by id (the list only carries a body prefix). Ok(None)
    /// means no such row.
    pub fn get(&self, id: i64) -> rusqlite::Result<Option<Note>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, body, updated FROM notes WHERE id=?1")?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Note {
                id: row.get(0)?,
                body: row.get(1)?,
                updated: row.get(2)?,
            })
        })?;
        match rows.next() {
            Some(Ok(note)) => Ok(Some(note)),
            Some(Err(e)) => {
                log::error!("load note {id}: {e}");
                Err(e)
            }
            None => Ok(None),
        }
    }

    /// Insert or update. `note.id`/`note.updated` are set only on success,
    /// so a failed write can no longer masquerade as a saved note (HIGH-1).
    pub fn save(&self, note: &mut Note) -> rusqlite::Result<()> {
        let now = util::now_epoch();
        let inserting = note.id == 0;
        let result = if inserting {
            self.conn.execute(
                "INSERT INTO notes (body, updated) VALUES (?1, ?2)",
                params![note.body, now],
            )
        } else {
            self.conn.execute(
                "UPDATE notes SET body=?1, updated=?2 WHERE id=?3",
                params![note.body, now, note.id],
            )
        };
        match result {
            Ok(_) => {
                if inserting {
                    note.id = self.conn.last_insert_rowid();
                }
                note.updated = now;
                Ok(())
            }
            Err(e) => {
                log::error!("save note {}: {e}", note.id);
                Err(e)
            }
        }
    }

    pub fn delete(&self, id: i64) -> rusqlite::Result<()> {
        let result = self
            .conn
            .execute("DELETE FROM notes WHERE id=?1", params![id]);
        if let Err(e) = &result {
            log::error!("delete note {id}: {e}");
        }
        result.map(|_| ())
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

    #[test]
    fn schema_is_version_stamped() {
        let db = NotesDb::open_in_memory().unwrap();
        let v: i64 = db
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 1, "NIT-1: fresh databases carry user_version = 1");
    }

    #[test]
    fn same_second_rows_tie_break_toward_the_newer_id() {
        let db = NotesDb::open_in_memory().unwrap();
        db.conn
            .execute(
                "INSERT INTO notes (body, updated) VALUES ('older', 500), ('newer', 500)",
                [],
            )
            .unwrap();
        let listed = db.list().unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(
            listed[0].body, "newer",
            "MED-3: id DESC breaks `updated` ties"
        );
    }

    #[test]
    fn list_carries_a_body_prefix_get_returns_the_whole_note() {
        let db = NotesDb::open_in_memory().unwrap();
        let mut n = Note {
            id: 0,
            body: format!("{}\nsecond line", "x".repeat(300)),
            updated: 0,
        };
        db.save(&mut n).unwrap();
        assert_ne!(n.id, 0);

        let listed = db.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].body.len() <= 120, "MED-5: list body is a preview");

        let full = db.get(n.id).unwrap().expect("row exists");
        assert_eq!(full.body.len(), 300 + "\nsecond line".len());
        assert!(db.get(999).unwrap().is_none(), "missing id is Ok(None)");
    }

    #[test]
    fn list_reports_read_errors_instead_of_an_empty_list() {
        let db = NotesDb::open_in_memory().unwrap();
        db.conn.execute("DROP TABLE notes", []).unwrap();
        assert!(
            db.list().is_err(),
            "HIGH-1: a read failure must not look like \"no notes\""
        );
    }

    // ---- grafted from the parallel fix branch (PR #11) ----

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
            db.save(&mut n).unwrap();
            assert_ne!(n.id, 0, "insert assigns the row id");
            assert!(n.updated > 0, "insert stamps updated");
        }
        let db = NotesDb::open(&dir).unwrap();
        let notes = db.list().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id, n.id);
        assert_eq!(notes[0].updated, n.updated);
        // The list carries only a prefix; the stored body is read via `get`.
        assert_eq!(
            db.get(n.id).unwrap().unwrap().body,
            "first line\nsecond line"
        );
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
        let db = NotesDb::open_in_memory().unwrap();

        // A failed insert must not invent an id or stamp updated.
        db.conn.execute("DROP TABLE notes", []).unwrap();
        let mut n = note("doomed insert");
        assert!(db.save(&mut n).is_err(), "failed insert reports the error");
        assert_eq!(
            (n.id, n.updated),
            (0, 0),
            "failed insert leaves the note as-is"
        );
        db.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS notes (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    body TEXT NOT NULL,
                    updated INTEGER NOT NULL
                )",
                [],
            )
            .unwrap();

        // A successful update re-stamps updated past a seeded old value.
        db.conn
            .execute(
                "INSERT INTO notes (body, updated) VALUES (?1, ?2)",
                params!["old body", 1000],
            )
            .unwrap();
        let mut m = db
            .list()
            .unwrap()
            .into_iter()
            .find(|x| x.body == "old body")
            .unwrap();
        assert_eq!(m.updated, 1000);
        m.body = "new body".to_owned();
        db.save(&mut m).unwrap();
        assert!(m.updated > 1000, "successful update re-stamps updated");

        // A failed update must leave updated where the last success put it.
        let stamped = m.updated;
        db.conn.execute("DROP TABLE notes", []).unwrap();
        m.body = "doomed update".to_owned();
        assert!(db.save(&mut m).is_err(), "failed update reports the error");
        assert_eq!(m.updated, stamped, "failed update must not bump updated");
    }

    #[test]
    fn delete_removes_only_its_note() {
        let db = NotesDb::open_in_memory().unwrap();
        let mut a = note("a");
        let mut b = note("b");
        db.save(&mut a).unwrap();
        db.save(&mut b).unwrap();

        db.delete(a.id).unwrap();
        let left = db.list().unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].body, "b");

        db.delete(a.id).unwrap(); // already gone: a no-op, not an error
        assert_eq!(db.list().unwrap().len(), 1);
    }

    #[test]
    fn list_orders_most_recent_first() {
        let db = NotesDb::open_in_memory().unwrap();
        for (body, updated) in [("old", 100), ("newest", 300), ("middle", 200)] {
            db.conn
                .execute(
                    "INSERT INTO notes (body, updated) VALUES (?1, ?2)",
                    params![body, updated],
                )
                .unwrap();
        }
        let notes = db.list().unwrap();
        let bodies: Vec<&str> = notes.iter().map(|n| n.body.as_str()).collect();
        assert_eq!(bodies, ["newest", "middle", "old"]);
    }
}
