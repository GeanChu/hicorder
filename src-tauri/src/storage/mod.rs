//! Persistência local (SQLite via rusqlite, bundled). Metadados das gravações.
//!
//! A chave da API nunca entra aqui — vai no keychain (PR5/PR6).

use std::path::Path;

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct RecordingRow {
    pub id: String,
    /// Faixa do microfone ("Você").
    pub path: String,
    /// Faixa do áudio do sistema ("Participantes"), se houver.
    pub system_path: Option<String>,
    pub created_at: i64,
    pub duration_s: f64,
    pub size_bytes: i64,
}

#[derive(Serialize, Clone)]
pub struct TranscriptRow {
    pub recording_id: String,
    pub language: String,
    pub text: String,
    pub created_at: i64,
}

pub fn open(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS recordings (
            id          TEXT PRIMARY KEY,
            path        TEXT NOT NULL,
            system_path TEXT,
            created_at  INTEGER NOT NULL,
            duration_s  REAL NOT NULL,
            size_bytes  INTEGER NOT NULL
        )",
        [],
    )?;
    // Migração de bancos antigos (sem a coluna). Erro = coluna já existe → ignora.
    let _ = conn.execute("ALTER TABLE recordings ADD COLUMN system_path TEXT", []);
    conn.execute(
        "CREATE TABLE IF NOT EXISTS transcripts (
            recording_id TEXT PRIMARY KEY,
            language     TEXT NOT NULL,
            text         TEXT NOT NULL,
            created_at   INTEGER NOT NULL
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;
    Ok(conn)
}

pub fn insert(conn: &Connection, r: &RecordingRow) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO recordings (id, path, system_path, created_at, duration_s, size_bytes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![r.id, r.path, r.system_path, r.created_at, r.duration_s, r.size_bytes],
    )?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<RecordingRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, path, system_path, created_at, duration_s, size_bytes
         FROM recordings ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RecordingRow {
            id: row.get(0)?,
            path: row.get(1)?,
            system_path: row.get(2)?,
            created_at: row.get(3)?,
            duration_s: row.get(4)?,
            size_bytes: row.get(5)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Caminhos das faixas de uma gravação: (microfone, áudio do sistema opcional).
pub fn recording_paths(conn: &Connection, id: &str) -> Result<Option<(String, Option<String>)>> {
    let row = conn
        .query_row(
            "SELECT path, system_path FROM recordings WHERE id = ?1",
            params![id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .optional()?;
    Ok(row)
}

pub fn upsert_transcript(conn: &Connection, t: &TranscriptRow) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO transcripts (recording_id, language, text, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![t.recording_id, t.language, t.text, t.created_at],
    )?;
    Ok(())
}

pub fn get_transcript(conn: &Connection, recording_id: &str) -> Result<Option<TranscriptRow>> {
    let row = conn
        .query_row(
            "SELECT recording_id, language, text, created_at FROM transcripts WHERE recording_id = ?1",
            params![recording_id],
            |r| {
                Ok(TranscriptRow {
                    recording_id: r.get(0)?,
                    language: r.get(1)?,
                    text: r.get(2)?,
                    created_at: r.get(3)?,
                })
            },
        )
        .optional()?;
    Ok(row)
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let v = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| {
            r.get::<_, String>(0)
        })
        .optional()?;
    Ok(v)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}
