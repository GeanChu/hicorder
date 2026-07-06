//! Sink que encoda áudio ao vivo para Opus em Ogg, canalizando PCM f32 para um
//! processo ffmpeg de longa duração (stdin). Substitui o WavSink: o arquivo no
//! disco já cresce codificado durante a gravação, então:
//! - um travamento não perde a gravação (Ogg tolera truncamento);
//! - parar é quase instantâneo (o encode já aconteceu ao longo da reunião).
//!
//! Entrada: f32 little-endian, `sample_rate`/`channels` do dispositivo.
//! Saída: Opus mono 16 kHz ~32 kbps (mesmo alvo do encode antigo).

use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};

use anyhow::{anyhow, bail, Result};

pub struct OpusSink {
    child: Child,
    stdin: Option<BufWriter<ChildStdin>>,
}

impl OpusSink {
    pub fn create(ffmpeg: &str, path: &Path, sample_rate: u32, channels: u16) -> Result<Self> {
        let mut cmd = Command::new(ffmpeg);
        cmd.arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-f")
            .arg("f32le")
            .arg("-ar")
            .arg(sample_rate.to_string())
            .arg("-ac")
            .arg(channels.to_string())
            .arg("-i")
            .arg("-")
            // Alvo: mono, 16 kHz, Opus ~32 kbps em Ogg.
            .arg("-ac")
            .arg("1")
            .arg("-ar")
            .arg("16000")
            .arg("-c:a")
            .arg("libopus")
            .arg("-b:a")
            .arg("32k")
            .arg("-f")
            .arg("ogg")
            .arg("-y")
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| anyhow!("falha ao iniciar o ffmpeg para gravar ('{ffmpeg}'): {e}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("ffmpeg sem stdin"))?;
        Ok(Self {
            child,
            stdin: Some(BufWriter::new(stdin)),
        })
    }

    pub fn write_f32(&mut self, samples: &[f32]) -> Result<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("sink já finalizado"))?;
        // f32 LE em lote (evita muitas escritas de 4 bytes).
        let mut buf = Vec::with_capacity(samples.len() * 4);
        for &s in samples {
            buf.extend_from_slice(&s.to_le_bytes());
        }
        stdin
            .write_all(&buf)
            .map_err(|e| anyhow!("falha ao enviar áudio ao ffmpeg: {e}"))?;
        Ok(())
    }

    /// Fecha o stdin (ffmpeg finaliza o Ogg) e espera o processo terminar.
    pub fn finalize(mut self) -> Result<()> {
        if let Some(mut w) = self.stdin.take() {
            let _ = w.flush();
            // Dropar o BufWriter fecha o ChildStdin → ffmpeg recebe EOF.
        }
        let status = self.child.wait()?;
        if !status.success() {
            bail!("ffmpeg terminou com erro ao finalizar a gravação");
        }
        Ok(())
    }
}
