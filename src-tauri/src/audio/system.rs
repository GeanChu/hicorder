//! Captura do áudio do sistema (loopback) — a voz dos outros participantes.
//!
//! Windows: WASAPI loopback via crate `wasapi` (device de render + Direction::Capture
//! em modo Shared liga o flag AUDCLNT_STREAMFLAGS_LOOPBACK). Linux (PR2c) e macOS
//! (PR3) ainda não implementados — `spawn_system` retorna `Ok(None)` lá.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::Arc;
use std::thread::JoinHandle;

use anyhow::Result;

use super::RecordedTrack;

#[cfg(windows)]
pub fn spawn_system(
    out_path: PathBuf,
    stop: Arc<AtomicBool>,
    level: Arc<AtomicU32>,
) -> Result<Option<JoinHandle<Result<RecordedTrack>>>> {
    Ok(Some(windows_impl::spawn(out_path, stop, level)?))
}

#[cfg(not(windows))]
pub fn spawn_system(
    _out_path: PathBuf,
    _stop: Arc<AtomicBool>,
    _level: Arc<AtomicU32>,
) -> Result<Option<JoinHandle<Result<RecordedTrack>>>> {
    // Linux (PR2c) e macOS (PR3) ainda não implementados.
    Ok(None)
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

    use super::super::wav::WavSink;
    use super::super::RecordedTrack;

    const CHANNELS: u16 = 2;
    const SAMPLE_RATE: u32 = 48_000;

    pub fn spawn(
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

            let mut sink = WavSink::create(&out_path, SAMPLE_RATE, CHANNELS)?;
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
