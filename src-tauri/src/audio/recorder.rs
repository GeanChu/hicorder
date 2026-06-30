//! Gerencia uma sessão de gravação: inicia/para as capturas e expõe o nível.
//!
//! PR2a: só microfone. A captura do áudio do sistema (loopback) entra no PR2b/PR2c.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use anyhow::{anyhow, Result};
use serde::Serialize;

use super::{mic, RecordedTrack};

struct ActiveSession {
    id: String,
    stop: Arc<AtomicBool>,
    mic_handle: JoinHandle<Result<RecordedTrack>>,
    mic_level: Arc<AtomicU32>,
    started: Instant,
}

#[derive(Default)]
pub struct Recorder {
    inner: Mutex<Option<ActiveSession>>,
}

#[derive(Serialize, Clone)]
pub struct RecordingInfo {
    pub id: String,
}

#[derive(Serialize, Clone)]
pub struct RecordingResult {
    pub id: String,
    pub mic_path: String,
    pub duration_s: f64,
}

impl Recorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_recording(&self) -> bool {
        self.inner.lock().unwrap().is_some()
    }

    /// Pico mais recente do microfone (0.0..=1.0). 0.0 se não estiver gravando.
    pub fn mic_level(&self) -> f32 {
        match &*self.inner.lock().unwrap() {
            Some(s) => f32::from_bits(s.mic_level.load(Ordering::Relaxed)),
            None => 0.0,
        }
    }

    pub fn start(&self, recordings_dir: PathBuf, id: String) -> Result<RecordingInfo> {
        let mut guard = self.inner.lock().unwrap();
        if guard.is_some() {
            return Err(anyhow!("já existe uma gravação em andamento"));
        }
        let dir = recordings_dir.join(&id);
        std::fs::create_dir_all(&dir)?;

        let stop = Arc::new(AtomicBool::new(false));
        let mic_level = Arc::new(AtomicU32::new(0));
        let mic_handle = mic::spawn_microphone(dir.join("mic.wav"), stop.clone(), mic_level.clone())?;

        *guard = Some(ActiveSession {
            id: id.clone(),
            stop,
            mic_handle,
            mic_level,
            started: Instant::now(),
        });
        Ok(RecordingInfo { id })
    }

    pub fn stop(&self) -> Result<RecordingResult> {
        let session = self
            .inner
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| anyhow!("nenhuma gravação em andamento"))?;

        session.stop.store(true, Ordering::Relaxed);
        let duration_s = session.started.elapsed().as_secs_f64();
        let track = session
            .mic_handle
            .join()
            .map_err(|_| anyhow!("a thread do microfone terminou em pânico"))??;

        Ok(RecordingResult {
            id: session.id,
            mic_path: track.path.to_string_lossy().into_owned(),
            duration_s,
        })
    }
}
