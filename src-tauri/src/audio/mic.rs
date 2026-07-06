//! Captura de microfone (cross-platform) via cpal, gravando em WAV.
//!
//! O stream do cpal é `!Send`, então é construído e mantido vivo dentro de uma
//! thread dedicada. As amostras chegam pelo callback de áudio, são enviadas por
//! um canal e escritas no WAV pela mesma thread.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;

use super::opus::OpusSink;
use super::RecordedTrack;

fn report_err(e: cpal::StreamError) {
    eprintln!("[mic] stream error: {e}");
}

/// Lista os dispositivos de entrada (microfones) disponíveis.
pub fn list_input_devices() -> Result<Vec<String>> {
    let host = cpal::default_host();
    let mut names = Vec::new();
    for device in host.input_devices()? {
        if let Ok(name) = device.name() {
            names.push(name);
        }
    }
    Ok(names)
}

/// Inicia a captura do microfone padrão numa thread dedicada.
/// `stop` sinaliza o fim; `level` recebe o pico mais recente (bits de f32, 0.0..=1.0).
pub fn spawn_microphone(
    ffmpeg: String,
    out_path: PathBuf,
    stop: Arc<AtomicBool>,
    level: Arc<AtomicU32>,
) -> Result<JoinHandle<Result<RecordedTrack>>> {
    let handle = thread::spawn(move || -> Result<RecordedTrack> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("nenhum microfone padrão encontrado"))?;
        let supported = device.default_input_config()?;
        let sample_format = supported.sample_format();
        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels();
        let config: cpal::StreamConfig = supported.into();

        let (tx, rx) = mpsc::channel::<Vec<f32>>();

        let stream = match sample_format {
            SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _: &_| {
                    let _ = tx.send(data.to_vec());
                },
                report_err,
                None,
            )?,
            SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _: &_| {
                    let v: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                    let _ = tx.send(v);
                },
                report_err,
                None,
            )?,
            SampleFormat::U16 => device.build_input_stream(
                &config,
                move |data: &[u16], _: &_| {
                    let v: Vec<f32> = data.iter().map(|&s| (s as f32 - 32768.0) / 32768.0).collect();
                    let _ = tx.send(v);
                },
                report_err,
                None,
            )?,
            other => bail!("formato de amostra do microfone não suportado: {other:?}"),
        };

        stream.play()?;

        let mut sink = OpusSink::create(&ffmpeg, &out_path, sample_rate, channels)?;
        while !stop.load(Ordering::Relaxed) {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(chunk) => {
                    let peak = chunk.iter().fold(0f32, |m, &s| m.max(s.abs()));
                    level.store(peak.to_bits(), Ordering::Relaxed);
                    sink.write_f32(&chunk)?;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        drop(stream);
        while let Ok(chunk) = rx.try_recv() {
            sink.write_f32(&chunk)?;
        }
        sink.finalize()?;

        Ok(RecordedTrack {
            path: out_path,
            sample_rate,
            channels,
        })
    });

    Ok(handle)
}
