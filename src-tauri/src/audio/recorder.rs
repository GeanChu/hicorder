//! Gerencia uma sessão de gravação: inicia/para as capturas e expõe o nível.
//!
//! PR2a: microfone. PR2b: + áudio do sistema no Windows (loopback). Cada fonte
//! grava sua própria faixa WAV; o mix vem no PR4 (ffmpeg).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use anyhow::{anyhow, Result};
use serde::Serialize;

use super::{mic, system, RecordedTrack};

type TrackHandle = JoinHandle<Result<RecordedTrack>>;

struct ActiveSession {
    id: String,
    title: String,
    stop: Arc<AtomicBool>,
    mic_handle: TrackHandle,
    system_handle: Option<TrackHandle>,
    mic_level: Arc<AtomicU32>,
    system_level: Arc<AtomicU32>,
    started: Instant,
    /// Fim previsto da reunião (unix ms) quando a gravação é de uma reunião.
    meeting_end_ms: Option<i64>,
    /// Se o alerta de fim de reunião já foi disparado.
    end_alerted: AtomicBool,
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
    pub title: String,
    pub mic_path: String,
    pub system_path: Option<String>,
    pub duration_s: f64,
}

impl Recorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_recording(&self) -> bool {
        self.inner.lock().unwrap().is_some()
    }

    /// Maior pico atual entre microfone e áudio do sistema (0.0..=1.0).
    pub fn level(&self) -> f32 {
        match &*self.inner.lock().unwrap() {
            Some(s) => {
                let mic = f32::from_bits(s.mic_level.load(Ordering::Relaxed));
                let sys = f32::from_bits(s.system_level.load(Ordering::Relaxed));
                mic.max(sys)
            }
            None => 0.0,
        }
    }

    /// Estado atual: (gravando, segundos decorridos, nível 0.0..=1.0).
    /// O tempo vem do backend, então sobrevive a trocas de aba na UI.
    pub fn status(&self) -> (bool, f64, f32) {
        match &*self.inner.lock().unwrap() {
            Some(s) => {
                let mic = f32::from_bits(s.mic_level.load(Ordering::Relaxed));
                let sys = f32::from_bits(s.system_level.load(Ordering::Relaxed));
                (true, s.started.elapsed().as_secs_f64(), mic.max(sys))
            }
            None => (false, 0.0, 0.0),
        }
    }

    pub fn start(
        &self,
        ffmpeg: String,
        recordings_dir: PathBuf,
        id: String,
        meeting_end_ms: Option<i64>,
        title: String,
    ) -> Result<RecordingInfo> {
        let mut guard = self.inner.lock().unwrap();
        if guard.is_some() {
            return Err(anyhow!("já existe uma gravação em andamento"));
        }
        let dir = recordings_dir.join(&id);
        std::fs::create_dir_all(&dir)?;

        let stop = Arc::new(AtomicBool::new(false));
        let mic_level = Arc::new(AtomicU32::new(0));
        let system_level = Arc::new(AtomicU32::new(0));

        // Encode ao vivo direto para Opus/Ogg (crash-safe + stop instantâneo).
        let mic_handle = mic::spawn_microphone(
            ffmpeg.clone(),
            dir.join("mic.ogg"),
            stop.clone(),
            mic_level.clone(),
        )?;
        let system_handle = system::spawn_system(
            ffmpeg,
            dir.join("system.ogg"),
            stop.clone(),
            system_level.clone(),
        )?;

        *guard = Some(ActiveSession {
            id: id.clone(),
            title,
            stop,
            mic_handle,
            system_handle,
            mic_level,
            system_level,
            started: Instant::now(),
            meeting_end_ms,
            end_alerted: AtomicBool::new(false),
        });
        Ok(RecordingInfo { id })
    }

    /// True (uma única vez) quando passou do fim previsto da reunião em andamento.
    pub fn should_alert_end(&self, now_ms: i64) -> bool {
        let guard = self.inner.lock().unwrap();
        if let Some(s) = &*guard {
            if let Some(end) = s.meeting_end_ms {
                if now_ms >= end && !s.end_alerted.swap(true, Ordering::Relaxed) {
                    return true;
                }
            }
        }
        false
    }

    /// True quando passou 1h do fim previsto da reunião em andamento (auto-stop).
    pub fn should_auto_stop(&self, now_ms: i64) -> bool {
        let guard = self.inner.lock().unwrap();
        if let Some(s) = &*guard {
            if let Some(end) = s.meeting_end_ms {
                return now_ms >= end + 3_600_000;
            }
        }
        false
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

        let mic_track = session
            .mic_handle
            .join()
            .map_err(|_| anyhow!("a thread do microfone terminou em pânico"))??;

        // Falha na captura do sistema degrada para só-microfone (não perde a gravação).
        let system_path = match session.system_handle {
            Some(handle) => match handle.join() {
                Ok(Ok(track)) => Some(track.path.to_string_lossy().into_owned()),
                Ok(Err(e)) => {
                    eprintln!("[system] captura falhou: {e}");
                    None
                }
                Err(_) => {
                    eprintln!("[system] thread terminou em pânico");
                    None
                }
            },
            None => None,
        };

        Ok(RecordingResult {
            id: session.id,
            title: session.title,
            mic_path: mic_track.path.to_string_lossy().into_owned(),
            system_path,
            duration_s,
        })
    }
}
