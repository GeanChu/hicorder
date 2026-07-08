//! Captura do áudio do sistema (loopback) — a voz dos outros participantes.
//!
//! - Windows: WASAPI loopback via crate `wasapi`.
//! - macOS: ScreenCaptureKit (crate `screencapturekit`, macOS 13+) — exige
//!   permissão de Gravação de Tela.
//! - Linux: ainda não implementado (`Ok(None)`).
//!
//! Falhas na captura do sistema degradam para só-microfone (não perdem a gravação).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::Arc;
use std::thread::JoinHandle;

use anyhow::Result;

use super::RecordedTrack;

#[cfg(windows)]
pub fn spawn_system(
    ffmpeg: String,
    out_path: PathBuf,
    stop: Arc<AtomicBool>,
    level: Arc<AtomicU32>,
) -> Result<Option<JoinHandle<Result<RecordedTrack>>>> {
    Ok(Some(windows_impl::spawn(ffmpeg, out_path, stop, level)?))
}

// macOS: captura do sistema via ScreenCaptureKit só quando a feature
// `macos-system-audio` está ligada. Ela puxa a dep `screencapturekit`, cujo
// build de Swift (apple-metal) exige um SDK muito novo e quebra a Cs do GitHub
// de forma recorrente. Por padrão fica desligada → macOS grava só o microfone.
#[cfg(all(target_os = "macos", feature = "macos-system-audio"))]
pub fn spawn_system(
    ffmpeg: String,
    out_path: PathBuf,
    stop: Arc<AtomicBool>,
    level: Arc<AtomicU32>,
) -> Result<Option<JoinHandle<Result<RecordedTrack>>>> {
    Ok(Some(macos_impl::spawn(ffmpeg, out_path, stop, level)?))
}

#[cfg(all(target_os = "macos", not(feature = "macos-system-audio")))]
pub fn spawn_system(
    _ffmpeg: String,
    _out_path: PathBuf,
    _stop: Arc<AtomicBool>,
    _level: Arc<AtomicU32>,
) -> Result<Option<JoinHandle<Result<RecordedTrack>>>> {
    // Áudio do sistema desabilitado neste build (só microfone).
    Ok(None)
}

#[cfg(all(not(windows), not(target_os = "macos")))]
pub fn spawn_system(
    _ffmpeg: String,
    _out_path: PathBuf,
    _stop: Arc<AtomicBool>,
    _level: Arc<AtomicU32>,
) -> Result<Option<JoinHandle<Result<RecordedTrack>>>> {
    // Linux ainda não implementado.
    Ok(None)
}

#[cfg(all(target_os = "macos", feature = "macos-system-audio"))]
mod macos_impl {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use anyhow::{anyhow, Result};
    use screencapturekit::prelude::*;

    use super::super::opus::OpusSink;
    use super::super::RecordedTrack;

    const SAMPLE_RATE: u32 = 48_000;

    /// Converte bytes little-endian em f32.
    fn to_f32(bytes: &[u8]) -> Vec<f32> {
        bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }

    pub fn spawn(
        ffmpeg: String,
        out_path: PathBuf,
        stop: Arc<AtomicBool>,
        level: Arc<AtomicU32>,
    ) -> Result<JoinHandle<Result<RecordedTrack>>> {
        let handle = thread::spawn(move || -> Result<RecordedTrack> {
            let content = SCShareableContent::get()
                .map_err(|e| anyhow!("ScreenCaptureKit (permissão de Gravação de Tela?): {e:?}"))?;
            let display = content
                .displays()
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("nenhum display encontrado"))?;
            let filter = SCContentFilter::create()
                .with_display(&display)
                .with_excluding_windows(&[])
                .build();
            let config = SCStreamConfiguration::new()
                .with_captures_audio(true)
                .with_sample_rate(SAMPLE_RATE as i32)
                .with_channel_count(2);

            let (tx, rx) = mpsc::channel::<Vec<f32>>();
            let level_cb = level.clone();
            let mut stream = SCStream::new(&filter, &config);
            stream.add_output_handler(
                move |sample: CMSampleBuffer, of_type: SCStreamOutputType| {
                    if !matches!(of_type, SCStreamOutputType::Audio) {
                        return;
                    }
                    let Some(list) = sample.audio_buffer_list() else {
                        return;
                    };
                    // Downmix para mono: planar (1 buffer por canal) ou já mono.
                    let nb = list.num_buffers();
                    let mono: Vec<f32> = if nb == 0 {
                        Vec::new()
                    } else if nb == 1 {
                        list.buffer(0).map(|b| to_f32(b.data())).unwrap_or_default()
                    } else {
                        let chans: Vec<Vec<f32>> = (0..nb)
                            .filter_map(|i| list.buffer(i).map(|b| to_f32(b.data())))
                            .collect();
                        let n = chans.iter().map(|c| c.len()).min().unwrap_or(0);
                        (0..n)
                            .map(|i| chans.iter().map(|c| c[i]).sum::<f32>() / chans.len() as f32)
                            .collect()
                    };
                    if !mono.is_empty() {
                        let peak = mono.iter().fold(0f32, |m, &s| m.max(s.abs()));
                        level_cb.store(peak.to_bits(), Ordering::Relaxed);
                        let _ = tx.send(mono);
                    }
                },
                SCStreamOutputType::Audio,
            );
            stream
                .start_capture()
                .map_err(|e| anyhow!("falha ao iniciar a captura do sistema: {e:?}"))?;

            let mut sink = OpusSink::create(&ffmpeg, &out_path, SAMPLE_RATE, 1)?;
            while !stop.load(Ordering::Relaxed) {
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(chunk) => sink.write_f32(&chunk)?,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            let _ = stream.stop_capture();
            while let Ok(chunk) = rx.try_recv() {
                sink.write_f32(&chunk)?;
            }
            sink.finalize()?;

            Ok(RecordedTrack {
                path: out_path,
                sample_rate: SAMPLE_RATE,
                channels: 1,
            })
        });
        Ok(handle)
    }
}

#[cfg(windows)]
mod windows_impl {
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};

    use anyhow::{anyhow, Result};
    use wasapi::{
        initialize_mta, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat,
    };

    use super::super::opus::OpusSink;
    use super::super::RecordedTrack;

    const CHANNELS: u16 = 2;
    const SAMPLE_RATE: u32 = 48_000;

    pub fn spawn(
        ffmpeg: String,
        out_path: PathBuf,
        stop: Arc<AtomicBool>,
        level: Arc<AtomicU32>,
    ) -> Result<JoinHandle<Result<RecordedTrack>>> {
        let handle = thread::spawn(move || -> Result<RecordedTrack> {
            initialize_mta()
                .ok()
                .map_err(|e| anyhow!("falha ao inicializar COM (MTA): {e}"))?;

            let enumerator = DeviceEnumerator::new().map_err(wasapi_err)?;
            let device = enumerator
                .get_default_device(&Direction::Render)
                .map_err(wasapi_err)?;
            let mut audio_client = device.get_iaudioclient().map_err(wasapi_err)?;

            let format =
                WaveFormat::new(32, 32, &SampleType::Float, SAMPLE_RATE as usize, CHANNELS as usize, None);
            let (_def_time, min_time) = audio_client.get_device_period().map_err(wasapi_err)?;
            let mode = StreamMode::EventsShared {
                autoconvert: true,
                buffer_duration_hns: min_time,
            };
            audio_client
                .initialize_client(&format, &Direction::Capture, &mode)
                .map_err(wasapi_err)?;

            let h_event = audio_client.set_get_eventhandle().map_err(wasapi_err)?;
            let capture_client = audio_client.get_audiocaptureclient().map_err(wasapi_err)?;
            let frame_bytes = format.get_blockalign() as usize; // canais * 4 bytes (f32)
            audio_client.start_stream().map_err(wasapi_err)?;

            let mut sink = OpusSink::create(&ffmpeg, &out_path, SAMPLE_RATE, CHANNELS)?;
            let mut queue: VecDeque<u8> = VecDeque::new();

            while !stop.load(Ordering::Relaxed) {
                // Espera o evento (timeout curto para reavaliar o stop). Em silêncio
                // total o evento pode não disparar — por isso o timeout.
                if h_event.wait_for_event(200).is_err() {
                    continue;
                }
                capture_client
                    .read_from_device_to_deque(&mut queue)
                    .map_err(wasapi_err)?;

                let mut peak = 0f32;
                while queue.len() >= frame_bytes {
                    let mut frame = [0f32; CHANNELS as usize];
                    for s in frame.iter_mut() {
                        let b = [
                            queue.pop_front().unwrap(),
                            queue.pop_front().unwrap(),
                            queue.pop_front().unwrap(),
                            queue.pop_front().unwrap(),
                        ];
                        *s = f32::from_le_bytes(b);
                    }
                    peak = peak.max(frame[0].abs()).max(frame[1].abs());
                    sink.write_f32(&frame)?;
                }
                level.store(peak.to_bits(), Ordering::Relaxed);
            }

            let _ = audio_client.stop_stream();
            while queue.len() >= frame_bytes {
                let mut frame = [0f32; CHANNELS as usize];
                for s in frame.iter_mut() {
                    let b = [
                        queue.pop_front().unwrap(),
                        queue.pop_front().unwrap(),
                        queue.pop_front().unwrap(),
                        queue.pop_front().unwrap(),
                    ];
                    *s = f32::from_le_bytes(b);
                }
                sink.write_f32(&frame)?;
            }
            sink.finalize()?;

            Ok(RecordedTrack {
                path: out_path,
                sample_rate: SAMPLE_RATE,
                channels: CHANNELS,
            })
        });
        Ok(handle)
    }

    fn wasapi_err(e: wasapi::WasapiError) -> anyhow::Error {
        anyhow!("wasapi: {e}")
    }
}
