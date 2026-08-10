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
    /// Nome da reunião que gerou a gravação, ou "Gravação manual".
    pub title: String,
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

#[derive(Serialize, Clone)]
pub struct SummaryRow {
    pub recording_id: String,
    pub text: String,
    pub created_at: i64,
}

#[derive(Serialize, Clone)]
pub struct MeetingRow {
    pub uid: String,
    pub title: String,
    pub starts_at: i64,
    pub ends_at: i64,
    pub record_enabled: bool,
    /// Emails separados por '\n' no banco; entregues como lista à UI.
    pub participants: Vec<String>,
    pub location: Option<String>,
    pub link: Option<String>,
}

pub fn open(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    // Várias conexões concorrem (comandos, scheduler, refresh no boot).
    // WAL permite leitura+escrita simultâneas; busy_timeout faz o escritor
    // esperar em vez de falhar com "database is locked".
    conn.busy_timeout(std::time::Duration::from_secs(10))?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
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
    let _ = conn.execute(
        "ALTER TABLE recordings ADD COLUMN title TEXT NOT NULL DEFAULT 'Gravação manual'",
        [],
    );
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
    conn.execute(
        "CREATE TABLE IF NOT EXISTS summaries (
            recording_id TEXT PRIMARY KEY,
            text         TEXT NOT NULL,
            created_at   INTEGER NOT NULL
        )",
        [],
    )?;
    // Anotações manuais da reunião. Tabela própria (não coluna em `recordings`)
    // porque as notas são salvas DURANTE a gravação, antes de a linha da gravação
    // existir — a linha só é inserida ao parar. A chave é o mesmo recording_id.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS notes (
            recording_id TEXT PRIMARY KEY,
            text         TEXT NOT NULL,
            updated_at   INTEGER NOT NULL
        )",
        [],
    )?;
    // Base de prompts de resumo nomeados (CRUD na aba "Prompts de resumo").
    // Fica no callrec.db em app-data, então sobrevive a atualizações do app.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS summary_prompts (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            text       TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS meetings (
            uid            TEXT PRIMARY KEY,
            title          TEXT NOT NULL,
            starts_at      INTEGER NOT NULL,
            ends_at        INTEGER NOT NULL,
            record_enabled INTEGER NOT NULL DEFAULT 0,
            participants   TEXT,
            location       TEXT,
            link           TEXT
        )",
        [],
    )?;
    // Migração de bancos antigos (erro = coluna já existe → ignora).
    let _ = conn.execute("ALTER TABLE meetings ADD COLUMN participants TEXT", []);
    let _ = conn.execute("ALTER TABLE meetings ADD COLUMN location TEXT", []);
    let _ = conn.execute("ALTER TABLE meetings ADD COLUMN link TEXT", []);
    Ok(conn)
}

pub fn insert(conn: &Connection, r: &RecordingRow) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO recordings (id, title, path, system_path, created_at, duration_s, size_bytes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![r.id, r.title, r.path, r.system_path, r.created_at, r.duration_s, r.size_bytes],
    )?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<RecordingRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, path, system_path, created_at, duration_s, size_bytes
         FROM recordings ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RecordingRow {
            id: row.get(0)?,
            title: row.get(1)?,
            path: row.get(2)?,
            system_path: row.get(3)?,
            created_at: row.get(4)?,
            duration_s: row.get(5)?,
            size_bytes: row.get(6)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn rename_recording(conn: &Connection, id: &str, title: &str) -> Result<()> {
    conn.execute(
        "UPDATE recordings SET title = ?2 WHERE id = ?1",
        params![id, title],
    )?;
    Ok(())
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

pub fn upsert_summary(conn: &Connection, s: &SummaryRow) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO summaries (recording_id, text, created_at) VALUES (?1, ?2, ?3)",
        params![s.recording_id, s.text, s.created_at],
    )?;
    Ok(())
}

pub fn get_summary(conn: &Connection, recording_id: &str) -> Result<Option<SummaryRow>> {
    let row = conn
        .query_row(
            "SELECT recording_id, text, created_at FROM summaries WHERE recording_id = ?1",
            params![recording_id],
            |r| {
                Ok(SummaryRow {
                    recording_id: r.get(0)?,
                    text: r.get(1)?,
                    created_at: r.get(2)?,
                })
            },
        )
        .optional()?;
    Ok(row)
}

pub fn upsert_notes(conn: &Connection, recording_id: &str, text: &str, updated_at: i64) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO notes (recording_id, text, updated_at) VALUES (?1, ?2, ?3)",
        params![recording_id, text, updated_at],
    )?;
    Ok(())
}

pub fn get_notes(conn: &Connection, recording_id: &str) -> Result<Option<String>> {
    let v = conn
        .query_row(
            "SELECT text FROM notes WHERE recording_id = ?1",
            params![recording_id],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    Ok(v)
}

#[derive(Serialize, Clone)]
pub struct SummaryPromptRow {
    pub id: String,
    pub name: String,
    pub text: String,
    pub created_at: i64,
}

pub fn list_summary_prompts(conn: &Connection) -> Result<Vec<SummaryPromptRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, text, created_at FROM summary_prompts ORDER BY name COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(SummaryPromptRow {
            id: r.get(0)?,
            name: r.get(1)?,
            text: r.get(2)?,
            created_at: r.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn upsert_summary_prompt(
    conn: &Connection,
    id: &str,
    name: &str,
    text: &str,
    created_at: i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO summary_prompts (id, name, text, created_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, text = excluded.text",
        params![id, name, text, created_at],
    )?;
    Ok(())
}

pub fn delete_summary_prompt(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM summary_prompts WHERE id = ?1", params![id])?;
    Ok(())
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

/// Remove a gravação e sua transcrição do banco. (Arquivos são apagados no command.)
pub fn delete_recording(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM notes WHERE recording_id = ?1", params![id])?;
    conn.execute("DELETE FROM summaries WHERE recording_id = ?1", params![id])?;
    conn.execute("DELETE FROM transcripts WHERE recording_id = ?1", params![id])?;
    conn.execute("DELETE FROM recordings WHERE id = ?1", params![id])?;
    Ok(())
}

/// Insere/atualiza reunião preservando `record_enabled` de reuniões já existentes.
/// `default_enabled` vale só para reuniões novas (respeita "gravar todas").
pub fn upsert_meeting(
    conn: &Connection,
    uid: &str,
    title: &str,
    starts_at: i64,
    ends_at: i64,
    default_enabled: bool,
    participants: &[String],
    location: Option<&str>,
    link: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO meetings (uid, title, starts_at, ends_at, record_enabled, participants, location, link)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(uid) DO UPDATE SET
            title = excluded.title, starts_at = excluded.starts_at, ends_at = excluded.ends_at,
            participants = excluded.participants, location = excluded.location, link = excluded.link",
        params![
            uid,
            title,
            starts_at,
            ends_at,
            default_enabled as i64,
            participants.join("\n"),
            location,
            link
        ],
    )?;
    Ok(())
}

pub fn list_meetings(conn: &Connection, from_ms: i64) -> Result<Vec<MeetingRow>> {
    let mut stmt = conn.prepare(
        "SELECT uid, title, starts_at, ends_at, record_enabled, participants, location, link
         FROM meetings WHERE ends_at >= ?1 ORDER BY starts_at ASC",
    )?;
    let rows = stmt.query_map([from_ms], |r| {
        let participants: Option<String> = r.get(5)?;
        Ok(MeetingRow {
            uid: r.get(0)?,
            title: r.get(1)?,
            starts_at: r.get(2)?,
            ends_at: r.get(3)?,
            record_enabled: r.get::<_, i64>(4)? != 0,
            participants: participants
                .unwrap_or_default()
                .lines()
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect(),
            location: r.get(6)?,
            link: r.get(7)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn set_meeting_record(conn: &Connection, uid: &str, enabled: bool) -> Result<()> {
    conn.execute(
        "UPDATE meetings SET record_enabled = ?2 WHERE uid = ?1",
        params![uid, enabled as i64],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HORA: i64 = 3_600_000;

    fn db() -> Connection {
        open(Path::new(":memory:")).unwrap()
    }

    fn add(conn: &Connection, uid: &str, starts_at: i64, gravar: bool) {
        upsert_meeting(conn, uid, "R", starts_at, starts_at + HORA, gravar, &[], None, None).unwrap();
    }

    fn uids(conn: &Connection, from: i64) -> Vec<String> {
        list_meetings(conn, from).unwrap().into_iter().map(|m| m.uid).collect()
    }

    #[test]
    fn remove_a_reuniao_que_sumiu_do_feed() {
        let c = db();
        let agora = 1_000 * HORA;
        add(&c, "fica", agora + 24 * HORA, false);
        add(&c, "apagada-no-google", agora + 48 * HORA, false);

        let n = prune_missing_meetings(&c, &["fica".to_string()], agora).unwrap();
        assert_eq!(n, 1);
        assert_eq!(uids(&c, agora), vec!["fica"]);
    }

    #[test]
    fn feed_vazio_nao_apaga_nada() {
        // Armadilha 1: fetch que falhou ou veio truncado não pode limpar a
        // agenda — o usuário perderia as reuniões marcadas para gravar.
        let c = db();
        let agora = 1_000 * HORA;
        add(&c, "a", agora + 24 * HORA, true);
        add(&c, "b", agora + 48 * HORA, false);

        assert_eq!(prune_missing_meetings(&c, &[], agora).unwrap(), 0);
        assert_eq!(uids(&c, agora).len(), 2);
    }

    #[test]
    fn nao_mexe_em_reuniao_ja_encerrada() {
        // Armadilha 2: o passado não está no feed (o ICS só expande o futuro).
        // Reconciliar sobre ele apagaria histórico legítimo.
        let c = db();
        let agora = 1_000 * HORA;
        add(&c, "antiga", agora - 48 * HORA, false);
        add(&c, "futura", agora + 24 * HORA, false);

        prune_missing_meetings(&c, &["futura".to_string()], agora).unwrap();
        // A antiga continua no banco (quem a remove é o prune por tempo).
        assert_eq!(uids(&c, 0), vec!["antiga", "futura"]);
    }

    #[test]
    fn preserva_o_agendamento_de_gravacao_de_quem_fica() {
        let c = db();
        let agora = 1_000 * HORA;
        add(&c, "marcada", agora + 24 * HORA, false);
        set_meeting_record(&c, "marcada", true).unwrap();
        add(&c, "sumiu", agora + 48 * HORA, false);

        prune_missing_meetings(&c, &["marcada".to_string()], agora).unwrap();
        let ms = list_meetings(&c, agora).unwrap();
        assert_eq!(ms.len(), 1);
        assert!(ms[0].record_enabled, "perdeu o 'Agendar Gravação'");
    }
}

pub fn prune_meetings(conn: &Connection, before_ms: i64) -> Result<()> {
    conn.execute("DELETE FROM meetings WHERE ends_at < ?1", params![before_ms])?;
    Ok(())
}

/// Remove as reuniões **ainda não encerradas** que não vieram na sincronização.
///
/// Sem isto a agenda só crescia: o upsert adicionava, e a única remoção era por
/// tempo (`prune_meetings`). Reunião apagada no Google sumia do feed mas ficava
/// no banco até passar da hora — e, se estivesse marcada para gravar, o
/// scheduler iniciava a gravação de uma reunião que não existe mais.
///
/// **Quem chama precisa garantir que a sincronização deu certo**: reconciliar
/// com resultado de um fetch que falhou apagaria a agenda inteira. Por isso
/// `uids` vazio é tratado como "não sei", e nada é removido.
///
/// Devolve quantas linhas foram apagadas.
pub fn prune_missing_meetings(conn: &Connection, uids: &[String], from_ms: i64) -> Result<usize> {
    if uids.is_empty() {
        return Ok(0);
    }
    let marcadores = std::iter::repeat("?").take(uids.len()).collect::<Vec<_>>().join(",");
    let sql = format!("DELETE FROM meetings WHERE ends_at >= ? AND uid NOT IN ({marcadores})");
    let mut valores: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(uids.len() + 1);
    valores.push(&from_ms);
    for u in uids {
        valores.push(u);
    }
    Ok(conn.execute(&sql, valores.as_slice())?)
}
