import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type Tab = "gravar" | "gravacoes" | "transcricao" | "config";

type Recording = {
  id: string;
  mic_path: string;
  duration_s: number;
};

const TABS: { id: Tab; label: string }[] = [
  { id: "gravar", label: "Gravar" },
  { id: "gravacoes", label: "Gravações" },
  { id: "transcricao", label: "Transcrição" },
  { id: "config", label: "Configurações" },
];

function App() {
  const [tab, setTab] = useState<Tab>("gravar");
  const [recordings, setRecordings] = useState<Recording[]>([]);

  return (
    <div className="app">
      <nav className="sidebar">
        <h1 className="brand">Call Recorder</h1>
        {TABS.map((t) => (
          <button
            key={t.id}
            className={tab === t.id ? "nav-item active" : "nav-item"}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </nav>

      <main className="content">
        {tab === "gravar" && (
          <RecordScreen onFinished={(r) => setRecordings((prev) => [r, ...prev])} />
        )}
        {tab === "gravacoes" && <RecordingsScreen recordings={recordings} />}
        {tab === "transcricao" && (
          <Placeholder title="Transcrição" hint="Seleção de idioma (padrão pt-BR), texto e copiar (PR5)." />
        )}
        {tab === "config" && (
          <Placeholder title="Configurações" hint="Idioma padrão, chave da API e 'gravar todos' (PR6)." />
        )}
      </main>
    </div>
  );
}

function RecordScreen({ onFinished }: { onFinished: (r: Recording) => void }) {
  const [recording, setRecording] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [level, setLevel] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const timers = useRef<number[]>([]);

  useEffect(() => {
    invoke<boolean>("is_recording").then(setRecording).catch(() => {});
    return () => timers.current.forEach(clearInterval);
  }, []);

  function clearTimers() {
    timers.current.forEach(clearInterval);
    timers.current = [];
  }

  async function start() {
    setError(null);
    try {
      await invoke("start_recording");
      setRecording(true);
      setElapsed(0);
      const t1 = window.setInterval(() => setElapsed((e) => e + 1), 1000);
      const t2 = window.setInterval(async () => {
        try {
          setLevel(await invoke<number>("recording_level"));
        } catch {
          /* ignore */
        }
      }, 100);
      timers.current = [t1, t2];
    } catch (e) {
      setError(String(e));
    }
  }

  async function stop() {
    clearTimers();
    setLevel(0);
    try {
      const r = await invoke<Recording>("stop_recording");
      onFinished(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setRecording(false);
    }
  }

  return (
    <section className="panel record">
      <h2>Gravar</h2>
      <button className={recording ? "rec-btn stop" : "rec-btn"} onClick={recording ? stop : start}>
        {recording ? "Parar" : "Gravar"}
      </button>

      {recording && (
        <div className="meters">
          <div className="timer">{formatTime(elapsed)}</div>
          <div className="level-bar">
            <div className="level-fill" style={{ width: `${Math.min(level * 100, 100)}%` }} />
          </div>
        </div>
      )}

      <p className="hint">PR2a grava só o microfone. Áudio do sistema (outros participantes) entra no PR2b.</p>
      {error && <p className="error">{error}</p>}
    </section>
  );
}

function RecordingsScreen({ recordings }: { recordings: Recording[] }) {
  return (
    <section className="panel">
      <h2>Gravações</h2>
      {recordings.length === 0 ? (
        <p className="hint">Nenhuma gravação ainda. Grave na aba Gravar.</p>
      ) : (
        <ul className="rec-list">
          {recordings.map((r) => (
            <li key={r.id}>
              <strong>{r.id}</strong> — {r.duration_s.toFixed(1)}s
              <div className="path">{r.mic_path}</div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function Placeholder({ title, hint }: { title: string; hint: string }) {
  return (
    <section className="panel">
      <h2>{title}</h2>
      <p className="hint">{hint}</p>
    </section>
  );
}

function formatTime(s: number): string {
  const m = Math.floor(s / 60);
  const sec = s % 60;
  return `${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
}

export default App;
