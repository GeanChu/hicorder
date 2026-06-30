//! Mix + encode via ffmpeg. Mistura mic + áudio do sistema numa faixa Opus leve.
//!
//! Dev: usa o `ffmpeg` do PATH (ou o caminho em `CALLREC_FFMPEG`). Em produção,
//! será o sidecar empacotado (PR7) — nada para o usuário instalar.

use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Result};

/// Mistura `mic` (+ `system` se houver) numa faixa Opus mono ~32 kbps, 16 kHz.
/// O container vem da extensão de `out` (usamos `.webm`, aceito pela MiniMax).
pub fn mix_to_opus(mic: &str, system: Option<&str>, out: &Path) -> Result<()> {
    let ffmpeg = ffmpeg_path();
    let mut cmd = Command::new(&ffmpeg);
    cmd.arg("-y").arg("-i").arg(mic);

    if let Some(sys) = system {
        cmd.arg("-i").arg(sys);
        cmd.arg("-filter_complex")
            .arg("amix=inputs=2:duration=longest:normalize=0");
    }

    cmd.arg("-ac").arg("1").arg("-ar").arg("16000");
    cmd.arg("-c:a").arg("libopus").arg("-b:a").arg("32k");
    cmd.arg(out);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    let output = cmd
        .output()
        .map_err(|e| anyhow!("falha ao executar ffmpeg ('{ffmpeg}'): {e}. Instale o ffmpeg ou defina CALLREC_FFMPEG."))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let last = stderr.lines().last().unwrap_or("erro desconhecido");
        bail!("ffmpeg falhou: {last}");
    }
    Ok(())
}

fn ffmpeg_path() -> String {
    std::env::var("CALLREC_FFMPEG").unwrap_or_else(|_| "ffmpeg".to_string())
}
