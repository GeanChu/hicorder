//! Migração única dos dados do app antigo ("Call Recorder",
//! com.hicapital.callrecorder) para o Hicorder (com.hicapital.hicorder).
//!
//! Copia (não move) a pasta de dados e corrige os caminhos absolutos das
//! gravações no DB copiado. A pasta antiga fica intacta como backup.

use std::fs;
use std::path::Path;

use tauri::{AppHandle, Manager};

const OLD_DIR_NAME: &str = "com.hicapital.callrecorder";
const DB_FILE: &str = "callrec.db";

pub fn run(app: &AppHandle) {
    let Ok(new_dir) = app.path().app_data_dir() else {
        return;
    };
    // Já migrado (ou instalação nova que já criou dados): não mexe.
    if new_dir.join(DB_FILE).exists() {
        return;
    }
    let Some(parent) = new_dir.parent() else {
        return;
    };
    let old_dir = parent.join(OLD_DIR_NAME);
    if !old_dir.join(DB_FILE).exists() {
        return;
    }

    if fs::create_dir_all(&new_dir).is_err() || copy_dir(&old_dir, &new_dir).is_err() {
        return;
    }

    // Os paths das gravações no DB apontavam para a pasta antiga.
    if let Ok(conn) = rusqlite::Connection::open(new_dir.join(DB_FILE)) {
        let old_s = old_dir.to_string_lossy().to_string();
        let new_s = new_dir.to_string_lossy().to_string();
        let _ = conn.execute(
            "UPDATE recordings SET path = REPLACE(path, ?1, ?2),
                                   system_path = REPLACE(system_path, ?1, ?2)",
            rusqlite::params![old_s, new_s],
        );
    }
    crate::logs::log(app, "INFO", "migracao", "dados do Call Recorder migrados para o Hicorder");
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}
