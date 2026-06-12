use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Shared, mutex-guarded connection held in Tauri state.
pub struct Db(pub Mutex<Connection>);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Task {
    pub id: String,
    pub title: String,
    /// active | waiting | done
    pub state: String,
    /// me | machine | null  (only meaningful when state == "waiting")
    pub waiting_kind: Option<String>,
    pub project: Option<String>,
    /// workspace | window | url | command | null
    pub jump_type: Option<String>,
    pub jump_value: Option<String>,
    pub notes: Option<String>,
    pub sort: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
}

pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn row_to_task(row: &Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get("id")?,
        title: row.get("title")?,
        state: row.get("state")?,
        waiting_kind: row.get("waiting_kind")?,
        project: row.get("project")?,
        jump_type: row.get("jump_type")?,
        jump_value: row.get("jump_value")?,
        notes: row.get("notes")?,
        sort: row.get("sort")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        completed_at: row.get("completed_at")?,
    })
}

/// Open (creating if needed) the database at ~/.task-stack/state.db and run migrations.
pub fn open() -> rusqlite::Result<Connection> {
    let dir = dirs::home_dir()
        .map(|h| h.join(".task-stack"))
        .expect("could not resolve home directory");
    std::fs::create_dir_all(&dir).expect("could not create ~/.task-stack");
    let conn = Connection::open(dir.join("state.db"))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tasks (
            id            TEXT PRIMARY KEY,
            title         TEXT NOT NULL,
            state         TEXT NOT NULL DEFAULT 'active',
            waiting_kind  TEXT,
            project       TEXT,
            jump_type     TEXT,
            jump_value    TEXT,
            notes         TEXT,
            sort          INTEGER NOT NULL DEFAULT 0,
            created_at    INTEGER NOT NULL,
            updated_at    INTEGER NOT NULL,
            completed_at  INTEGER
         );
         CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
         );",
    )?;
    Ok(conn)
}

pub fn list_tasks(conn: &Connection) -> rusqlite::Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM tasks ORDER BY sort ASC, created_at DESC",
    )?;
    let rows = stmt.query_map([], row_to_task)?;
    rows.collect()
}

fn get_task(conn: &Connection, id: &str) -> rusqlite::Result<Task> {
    conn.query_row("SELECT * FROM tasks WHERE id = ?1", [id], row_to_task)
}

pub fn create_task(
    conn: &Connection,
    title: &str,
    project: Option<String>,
    jump_type: Option<String>,
    jump_value: Option<String>,
) -> rusqlite::Result<Task> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();
    // New tasks sort to the top of their group (most-negative sort first).
    let sort = -now;
    conn.execute(
        "INSERT INTO tasks (id, title, state, project, jump_type, jump_value, sort, created_at, updated_at)
         VALUES (?1, ?2, 'active', ?3, ?4, ?5, ?6, ?7, ?7)",
        rusqlite::params![id, title, project, jump_type, jump_value, sort, now],
    )?;
    get_task(conn, &id)
}

pub fn set_state(
    conn: &Connection,
    id: &str,
    state: &str,
    waiting_kind: Option<String>,
) -> rusqlite::Result<Task> {
    let now = now_millis();
    let completed_at = if state == "done" { Some(now) } else { None };
    let kind = if state == "waiting" { waiting_kind } else { None };
    conn.execute(
        "UPDATE tasks SET state = ?2, waiting_kind = ?3, completed_at = ?4, updated_at = ?5 WHERE id = ?1",
        rusqlite::params![id, state, kind, completed_at, now],
    )?;
    get_task(conn, id)
}

pub fn update_title(
    conn: &Connection,
    id: &str,
    title: &str,
    project: Option<String>,
) -> rusqlite::Result<Task> {
    conn.execute(
        "UPDATE tasks SET title = ?2, project = ?3, updated_at = ?4 WHERE id = ?1",
        rusqlite::params![id, title, project, now_millis()],
    )?;
    get_task(conn, id)
}

pub fn set_notes(conn: &Connection, id: &str, notes: Option<String>) -> rusqlite::Result<Task> {
    conn.execute(
        "UPDATE tasks SET notes = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, notes, now_millis()],
    )?;
    get_task(conn, id)
}

pub fn set_jump(
    conn: &Connection,
    id: &str,
    jump_type: Option<String>,
    jump_value: Option<String>,
) -> rusqlite::Result<Task> {
    conn.execute(
        "UPDATE tasks SET jump_type = ?2, jump_value = ?3, updated_at = ?4 WHERE id = ?1",
        rusqlite::params![id, jump_type, jump_value, now_millis()],
    )?;
    get_task(conn, id)
}

pub fn delete_task(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
    Ok(())
}

/// Rewrite the `sort` column for an ordered list of ids (0..n).
pub fn reorder(conn: &mut Connection, ids: &[String]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE tasks SET sort = ?2 WHERE id = ?1",
            rusqlite::params![id, i as i64],
        )?;
    }
    tx.commit()
}

pub fn get_setting(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}
