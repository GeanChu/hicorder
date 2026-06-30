//! Captura de áudio: microfone + áudio do sistema (loopback).
//!
//! Implementação nos PR2 (Windows/Linux) e PR3 (macOS / ScreenCaptureKit).
//! Portar de meetily (MIT): `frontend/src-tauri/src/audio/`
//!   - capture/ (WASAPI / CoreAudio / PulseAudio)
//!   - devices/ (enumeração)
//!   - level_monitor.rs, incremental_saver.rs, recording_manager.rs
//!   - ffmpeg_mixer.rs (adaptar saída para Opus)
