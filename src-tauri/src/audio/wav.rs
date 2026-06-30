//! Gravação de WAV (PCM 16-bit) via hound. Formato bruto do PR2;
//! convertido para Opus `.ogg` no PR4.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use anyhow::Result;
use hound::{SampleFormat, WavSpec, WavWriter};

pub struct WavSink {
    writer: WavWriter<BufWriter<File>>,
}

impl WavSink {
    pub fn create(path: &Path, sample_rate: u32, channels: u16) -> Result<Self> {
        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        Ok(Self {
            writer: WavWriter::create(path, spec)?,
        })
    }

    pub fn write_f32(&mut self, samples: &[f32]) -> Result<()> {
        for &s in samples {
            let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            self.writer.write_sample(v)?;
        }
        Ok(())
    }

    pub fn finalize(self) -> Result<()> {
        self.writer.finalize()?;
        Ok(())
    }
}
