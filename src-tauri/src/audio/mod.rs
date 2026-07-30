//! Captura de áudio: microfone (PR2a) e, futuramente, áudio do sistema.
//!
//! macOS implementa loopback só no PR3 (ScreenCaptureKit). Windows (WASAPI
//! loopback) e Linux (monitor source) entram no PR2b/PR2c. Referência: meetily
//! (MIT) — porém o loopback Win/Linux dele não é implementado, então construímos
//! o nosso.

mod mic;
mod opus;
pub mod recorder;
mod system;

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

#[cfg(test)]
mod tests {
    use super::recorder::Recorder;
    use std::time::Duration;

    /// Smoke test local (ignored): grava alguns segundos com o pipeline REAL
    /// (mic via cpal + loopback do sistema + OpusSink/ffmpeg do PATH) e imprime
    /// o tamanho de cada faixa. Diagnóstico de captura vazia sem instalador.
    ///
    ///   cargo test --release -- --ignored rec_smoke --nocapture
    ///
    /// `RECTEST_SECS` controla a duração (default 5s). Toque um áudio durante o
    /// teste para validar o loopback (faixa "sistema" > 2 KB).
    /// Probe do cpal (ignored): mostra o device/config default e conta as
    /// amostras entregues em 3s — isola a captura do resto do pipeline.
    #[test]
    #[ignore]
    fn mic_probe() {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let host = cpal::default_host();
        let device = host.default_input_device().expect("sem mic default");
        println!("device: {:?}", device.name());
        let supported = device.default_input_config().expect("sem config default");
        println!("config: {supported:?}");
        let count = Arc::new(AtomicUsize::new(0));
        let c2 = count.clone();
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    &supported.into(),
                    move |d: &[f32], _: &_| {
                        c2.fetch_add(d.len(), Ordering::SeqCst);
                    },
                    |e| eprintln!("stream err: {e}"),
                    None,
                )
                .unwrap(),
            other => panic!("formato {other:?} — probe só cobre f32"),
        };
        stream.play().unwrap();
        std::thread::sleep(Duration::from_secs(3));
        println!("amostras em 3s: {}", count.load(Ordering::SeqCst));
    }

    /// Probe do loopback WASAPI (ignored, Windows): replica o loop do
    /// windows_impl e conta eventos disparados + frames lidos em 5s.
    /// Rode com e sem áudio tocando para comparar.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn system_probe() {
        use wasapi::{initialize_mta, DeviceEnumerator, Direction, SampleType, StreamMode};

        let t0 = std::time::Instant::now();
        let step = |name: &str| println!("[{:>6.2}s] {name}", t0.elapsed().as_secs_f64());

        initialize_mta().ok().expect("MTA");
        step("MTA");
        let enumerator = DeviceEnumerator::new().expect("enumerator");
        step("enumerator");
        let device = enumerator.get_default_device(&Direction::Render).expect("render device");
        step("default render");
        println!("render device: {:?}", device.get_friendlyname());
        let mut audio_client = device.get_iaudioclient().expect("iaudioclient");
        step("iaudioclient");
        // Formato de mix nativo do device (em vez de forçar 48k/f32/2ch).
        let mix = audio_client.get_mixformat().expect("mixformat");
        println!(
            "mix format: {} Hz, {} ch, {} bits ({:?})",
            mix.get_samplespersec(),
            mix.get_nchannels(),
            mix.get_bitspersample(),
            mix.get_subformat()
        );
        let format = mix;
        let _ = SampleType::Float; // mantém o import usado no probe original
        let mode = StreamMode::EventsShared { autoconvert: true, buffer_duration_hns: 0 };
        audio_client
            .initialize_client(&format, &Direction::Capture, &mode)
            .expect("initialize (loopback)");
        step("initialize");
        let h_event = audio_client.set_get_eventhandle().expect("event handle");
        let capture = audio_client.get_audiocaptureclient().expect("capture client");
        audio_client.start_stream().expect("start");
        step("start_stream");

        let mut events = 0u32;
        let mut timeouts = 0u32;
        let mut bytes = 0usize;
        let mut poll_bytes = 0usize;
        let mut queue = std::collections::VecDeque::new();
        let loop_t = std::time::Instant::now();
        while loop_t.elapsed() < Duration::from_secs(5) {
            if h_event.wait_for_event(200).is_err() {
                timeouts += 1;
                // Poll no timeout: em alguns drivers o evento de loopback não
                // dispara; lê mesmo assim para ver se há frames disponíveis.
                if capture.read_from_device_to_deque(&mut queue).is_ok() {
                    poll_bytes += queue.len();
                    queue.clear();
                }
                continue;
            }
            events += 1;
            capture.read_from_device_to_deque(&mut queue).expect("read");
            bytes += queue.len();
            queue.clear();
        }
        println!(
            "eventos: {events}, timeouts: {timeouts}, bytes(evento): {bytes}, bytes(poll): {poll_bytes}"
        );
    }

    #[test]
    #[ignore]
    fn rec_smoke() {
        let secs: u64 = std::env::var("RECTEST_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        let dir = std::env::temp_dir().join("hicorder-rec-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let ffmpeg = std::env::var("CALLREC_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
        let rec = Recorder::new();
        rec.start(ffmpeg, dir.clone(), "test".into(), None, "teste".into())
            .expect("start falhou");
        // Nível de pico ao longo da gravação: se ficar 0.000 o tempo todo,
        // nenhuma amostra está chegando às threads de captura.
        for i in 0..secs * 2 {
            std::thread::sleep(Duration::from_millis(500));
            println!("t={:>4}ms nivel={:.4}", (i + 1) * 500, rec.level());
        }
        let res = rec.stop().expect("stop falhou");

        let mic = std::fs::metadata(&res.mic_path).map(|m| m.len()).unwrap_or(0);
        println!("duracao: {:.1}s", res.duration_s);
        println!("mic:     {mic} bytes ({})", res.mic_path);
        match &res.system_path {
            Some(p) => {
                let s = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                println!("sistema: {s} bytes ({p})");
            }
            None => println!("sistema: SEM FAIXA — erro: {:?}", res.system_error),
        }
        assert!(mic > 2048, "faixa do mic vazia ({mic} bytes)");
    }
}
