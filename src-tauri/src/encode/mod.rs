//! Encode e mix de áudio via ffmpeg (sidecar empacotado).
//!
//! Implementação no PR4. Saída final: Opus mono ~32 kbps, 16 kHz, container `.ogg`.
//! Mix mic+sistema com filtro `amix`.
