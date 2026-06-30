//! Captura de áudio: microfone (PR2a) e, futuramente, áudio do sistema.
//!
//! macOS implementa loopback só no PR3 (ScreenCaptureKit). Windows (WASAPI
//! loopback) e Linux (monitor source) entram no PR2b/PR2c. Referência: meetily
//! (MIT) — porém o loopback Win/Linux dele não é implementado, então construímos
//! o nosso.

mod mic;
pub mod recorder;
mod system;
mod wav;

use std::path::PathBuf;

pub use mic::list_input_devices;

/// Uma faixa de áudio gravada em disco (WAV bruto no PR2).
/// `sample_rate`/`channels` são consumidos no PR4 (encode).
#[derive(Clone)]
#[allow(dead_code)]
pub struct RecordedTrack {
    pub path: PathBuf,
    pub sample_rate: u32,
    pub channels: u16,
}
