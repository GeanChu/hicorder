import { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type Tab = "gravar" | "agenda" | "gravacoes" | "transcricao" | "config";

type Meeting = {
  uid: string;
  title: string;
  starts_at: number;
  ends_at: number;
  record_enabled: boolean;
};

type Recording = {
  id: string;
  path: string;
  system_path: string | null;
  created_at: number;
  duration_s: number;
  size_bytes: number;
};

type Transcript = {
  recording_id: string;
  language: string;
  text: string;
  created_at: number;
};

type Settings = {
  default_language: string;
  endpoint_url: string;
  model: string;
  has_api_key: boolean;
  summary_endpoint_url: string;
  summary_model: string;
  has_summary_key: boolean;
  ics_url: string;
  record_all: boolean;
};

type Summary = {
  recording_id: string;
  text: string;
  created_at: number;
};

const TABS: { id: Tab; label: string }[] = [
  { id: "gravar", label: "Gravar" },
  { id: "agenda", label: "Agenda" },
  { id: "gravacoes", label: "Gravações" },
  { id: "transcricao", label: "Transcrição" },
  { id: "config", label: "Configurações" },
];

const LANGUAGES: { code: string; label: string }[] = [
  { code: "pt", label: "Português (BR)" },
  { code: "en", label: "Inglês" },
  { code: "es", label: "Espanhol" },
  { code: "fr", label: "Francês" },
  { code: "de", label: "Alemão" },
  { code: "it", label: "Italiano" },
];

function icon(name: string) {
  const c = { width: 18, height: 18, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 2 };
  switch (name) {
    case "mic":
    case "gravar":
      return (
        <svg {...c}>
          <rect x="9" y="2" width="6" height="12" rx="3" />
          <path d="M5 10a7 7 0 0 0 14 0" />
          <line x1="12" y1="19" x2="12" y2="22" />
        </svg>
      );
    case "gravacoes":
      return (
        <svg {...c}>
          <line x1="8" y1="6" x2="21" y2="6" />
          <line x1="8" y1="12" x2="21" y2="12" />
          <line x1="8" y1="18" x2="21" y2="18" />
          <circle cx="3.5" cy="6" r="1" />
          <circle cx="3.5" cy="12" r="1" />
          <circle cx="3.5" cy="18" r="1" />
        </svg>
      );
    case "transcricao":
      return (
        <svg {...c}>
          <path d="M14 3v4a1 1 0 0 0 1 1h4" />
          <path d="M17 21H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7l5 5v11a2 2 0 0 1-2 2z" />
          <line x1="9" y1="13" x2="15" y2="13" />
          <line x1="9" y1="17" x2="13" y2="17" />
        </svg>
      );
    case "agenda":
      return (
        <svg {...c}>
          <rect x="3" y="4" width="18" height="18" rx="2" />
          <line x1="16" y1="2" x2="16" y2="6" />
          <line x1="8" y1="2" x2="8" y2="6" />
          <line x1="3" y1="10" x2="21" y2="10" />
        </svg>
      );
    case "config":
      return (
        <svg {...c}>
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
      );
    default:
      return null;
  }
}

function App() {
  const [tab, setTab] = useState<Tab>("gravar");
  const [recordings, setRecordings] = useState<Recording[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);

  const refreshRecordings = useCallback(async () => {
    try {
      setRecordings(await invoke<Recording[]>("list_recordings"));
    } catch {
      /* ignore */
    }
  }, []);

  const refreshSettings = useCallback(async () => {
    try {
      setSettings(await invoke<Settings>("get_settings"));
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    refreshRecordings();
    refreshSettings();
  }, [refreshRecordings, refreshSettings]);

  useEffect(() => {
    const un = listen("recording-changed", () => refreshRecordings());
    return () => {
      un.then((f) => f());
    };
  }, [refreshRecordings]);

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="brand">
          <span className="brand-dot">{icon("mic")}</span>
          Call Recorder
        </div>
        {TABS.map((t) => (
          <button
            key={t.id}
            className={tab === t.id ? "nav-item active" : "nav-item"}
            onClick={() => setTab(t.id)}
          >
            {icon(t.id)}
            {t.label}
          </button>
        ))}
      </nav>

      <main className="content">
        {tab === "gravar" && <RecordScreen onFinished={refreshRecordings} />}
        {tab === "agenda" && <AgendaScreen hasIcs={!!settings?.ics_url} />}
        {tab === "gravacoes" && (
          <RecordingsScreen recordings={recordings} onChanged={refreshRecordings} />
        )}
        {tab === "transcricao" && (
          <TranscriptionScreen
            recordings={recordings}
            defaultLanguage={settings?.default_language ?? "pt"}
            hasApiKey={settings?.has_api_key ?? false}
            hasSummaryKey={settings?.has_summary_key ?? false}
          />
        )}
        {tab === "config" && <ConfigScreen settings={settings} onSaved={refreshSettings} />}
      </main>
    </div>
  );
}

function RecordScreen({ onFinished }: { onFinished: () => void }) {
  const [recording, setRecording] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [level, setLevel] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const timers = useRef<number[]>([]);

  useEffect(() => {
    invoke<boolean>("is_recording")
      .then((r) => {
        setRecording(r);
        if (r) beginPolling();
      })
      .catch(() => {});
    return () => timers.current.forEach(clearInterval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const un = listen<boolean>("recording-changed", (e) => {
      setRecording(e.payload);
      if (e.payload) {
        if (timers.current.length === 0) beginPolling();
      } else {
        clearTimers();
        setLevel(0);
        setElapsed(0);
      }
    });
    return () => {
      un.then((f) => f());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function clearTimers() {
    timers.current.forEach(clearInterval);
    timers.current = [];
  }

  function beginPolling() {
    clearTimers();
    const t = window.setInterval(async () => {
      try {
        const s = await invoke<{ recording: boolean; elapsed_s: number; level: number }>(
          "recording_status",
        );
        setRecording(s.recording);
        setElapsed(Math.floor(s.elapsed_s));
        setLevel(s.level);
        if (!s.recording) {
          clearTimers();
          setLevel(0);
        }
      } catch {
        /* ignore */
      }
    }, 200);
    timers.current = [t];
  }

  async function start() {
    setError(null);
    try {
      await invoke("start_recording");
      setRecording(true);
      beginPolling();
    } catch (e) {
      setError(String(e));
    }
  }

  async function stop() {
    clearTimers();
    setLevel(0);
    setBusy(true);
    try {
      await invoke<Recording>("stop_recording");
      onFinished();
    } catch (e) {
      setError(String(e));
    } finally {
      setRecording(false);
      setBusy(false);
    }
  }

  return (
    <section className="panel record">
      <h2>Gravar</h2>
      <button
        className={recording ? "rec-btn stop" : "rec-btn"}
        onClick={recording ? stop : start}
        disabled={busy}
      >
        <span className="rec-dot" />
        {busy ? "Processando..." : recording ? "Parar" : "Gravar"}
      </button>

      {recording && (
        <div className="meters">
          <div className="timer">{formatTime(elapsed)}</div>
          <div className="level-bar">
            <div className="level-fill" style={{ width: `${Math.min(level * 100, 100)}%` }} />
          </div>
        </div>
      )}

      <p className="hint">
        Grava microfone + áudio do sistema (Windows) em faixas separadas (Opus, leve). A transcrição
        rotula "Você" (mic) e "Participantes" (sistema). No Linux/macOS o áudio do sistema chega depois.
      </p>
      {error && <p className="error">{error}</p>}
    </section>
  );
}

function AgendaScreen({ hasIcs }: { hasIcs: boolean }) {
  const [meetings, setMeetings] = useState<Meeting[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<Meeting[]>("list_meetings").then(setMeetings).catch(() => {});
  }, []);

  async function refresh() {
    setError(null);
    setBusy(true);
    try {
      setMeetings(await invoke<Meeting[]>("refresh_meetings"));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function toggle(uid: string, enabled: boolean) {
    setMeetings((ms) => ms.map((m) => (m.uid === uid ? { ...m, record_enabled: enabled } : m)));
    try {
      await invoke("set_meeting_record", { uid, enabled });
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <section className="panel">
      <h2>Agenda</h2>
      <div className="actions">
        <button onClick={refresh} disabled={busy}>
          {busy ? "Atualizando..." : "Atualizar"}
        </button>
      </div>
      {!hasIcs && <p className="hint">Configure a URL do calendário (ICS) em Configurações.</p>}
      {error && <p className="error">{error}</p>}
      {meetings.length === 0 ? (
        <div className="empty">
          {icon("agenda")}
          <p>Nenhuma reunião próxima. Clique em Atualizar.</p>
        </div>
      ) : (
        <ul className="rec-list">
          {meetings.map((m) => (
            <li key={m.uid}>
              <div className="rec-row">
                <div className="rec-meta">
                  {m.title}
                  <small>{formatMeetingTime(m.starts_at, m.ends_at)}</small>
                </div>
                <label className="chk">
                  <input
                    type="checkbox"
                    checked={m.record_enabled}
                    onChange={(e) => toggle(m.uid, e.target.checked)}
                  />
                  Gravar
                </label>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function RecordingsScreen({
  recordings,
  onChanged,
}: {
  recordings: Recording[];
  onChanged: () => void;
}) {
  const [playing, setPlaying] = useState<string | null>(null);

  async function remove(id: string) {
    if (!window.confirm("Apagar esta gravação e sua transcrição? Não dá pra desfazer.")) return;
    try {
      await invoke("delete_recording", { recordingId: id });
      if (playing === id) setPlaying(null);
      onChanged();
    } catch (e) {
      alert(String(e));
    }
  }

  function mixSrc(micPath: string): string {
    return convertFileSrc(micPath.replace(/mic\.webm$/, "recording.webm"));
  }

  return (
    <section className="panel">
      <h2>Gravações</h2>
      {recordings.length === 0 ? (
        <div className="empty">
          {icon("gravacoes")}
          <p>Nenhuma gravação ainda. Grave na aba Gravar.</p>
        </div>
      ) : (
        <ul className="rec-list">
          {recordings.map((r) => (
            <li key={r.id}>
              <div className="rec-row">
                <div className="rec-meta">
                  {formatDate(r.created_at)}
                  <small>
                    {formatTime(Math.round(r.duration_s))} · {formatSize(r.size_bytes)}
                  </small>
                </div>
                <div className="rec-actions">
                  <button
                    className="play-btn"
                    onClick={() => setPlaying(playing === r.id ? null : r.id)}
                  >
                    {playing === r.id ? "Fechar" : "▶ Play"}
                  </button>
                  <button className="del-btn" onClick={() => remove(r.id)}>
                    Apagar
                  </button>
                </div>
              </div>
              {playing === r.id && (
                <audio className="player" controls autoPlay src={mixSrc(r.path)} />
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function TranscriptionScreen({
  recordings,
  defaultLanguage,
  hasApiKey,
  hasSummaryKey,
}: {
  recordings: Recording[];
  defaultLanguage: string;
  hasApiKey: boolean;
  hasSummaryKey: boolean;
}) {
  const [selectedId, setSelectedId] = useState("");
  const [language, setLanguage] = useState(defaultLanguage);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [summary, setSummary] = useState("");
  const [sumBusy, setSumBusy] = useState(false);
  const [sumError, setSumError] = useState<string | null>(null);
  const [sumCopied, setSumCopied] = useState(false);

  useEffect(() => {
    if (!selectedId && recordings.length > 0) setSelectedId(recordings[0].id);
  }, [recordings, selectedId]);

  useEffect(() => {
    setText("");
    setSummary("");
    setError(null);
    setSumError(null);
    if (!selectedId) return;
    invoke<Transcript | null>("get_transcript", { recordingId: selectedId })
      .then((t) => {
        if (t) {
          setText(t.text);
          setLanguage(t.language);
        }
      })
      .catch(() => {});
    invoke<Summary | null>("get_summary", { recordingId: selectedId })
      .then((s) => {
        if (s) setSummary(s.text);
      })
      .catch(() => {});
  }, [selectedId]);

  async function run() {
    setError(null);
    setBusy(true);
    try {
      const t = await invoke<Transcript>("transcribe", { recordingId: selectedId, language });
      setText(t.text);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function copy() {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  }

  async function makeSummary() {
    setSumError(null);
    setSumBusy(true);
    try {
      const s = await invoke<Summary>("generate_summary", { recordingId: selectedId });
      setSummary(s.text);
    } catch (e) {
      setSumError(String(e));
    } finally {
      setSumBusy(false);
    }
  }

  async function copySummary() {
    await navigator.clipboard.writeText(summary);
    setSumCopied(true);
    window.setTimeout(() => setSumCopied(false), 1500);
  }

  return (
    <section className="panel">
      <h2>Transcrição</h2>
      {recordings.length === 0 ? (
        <div className="empty">
          {icon("transcricao")}
          <p>Grave algo primeiro na aba Gravar.</p>
        </div>
      ) : (
        <>
          <div className="form-row">
            <label>Gravação</label>
            <select value={selectedId} onChange={(e) => setSelectedId(e.target.value)}>
              {recordings.map((r) => (
                <option key={r.id} value={r.id}>
                  {formatDate(r.created_at)} — {formatTime(Math.round(r.duration_s))}
                </option>
              ))}
            </select>
          </div>

          <div className="form-row">
            <label>Idioma</label>
            <select value={language} onChange={(e) => setLanguage(e.target.value)}>
              {LANGUAGES.map((l) => (
                <option key={l.code} value={l.code}>
                  {l.label}
                </option>
              ))}
            </select>
          </div>

          {!hasApiKey && (
            <p className="hint">Configure a chave da API em Configurações antes de transcrever.</p>
          )}

          <div className="actions">
            <button onClick={run} disabled={busy || !selectedId}>
              {busy ? "Transcrevendo..." : "Transcrever"}
            </button>
            {text && (
              <button className="secondary" onClick={copy}>
                {copied ? "Copiado!" : "Copiar"}
              </button>
            )}
          </div>

          {error && <p className="error">{error}</p>}
          {text && <textarea className="transcript" readOnly value={text} />}

          {text && (
            <div className="summary-block">
              <h3>Resumo (opcional)</h3>
              {!hasSummaryKey && (
                <p className="hint">Configure a chave do Resumo (MiniMax) em Configurações.</p>
              )}
              <div className="actions">
                <button onClick={makeSummary} disabled={sumBusy || !hasSummaryKey}>
                  {sumBusy ? "Resumindo..." : summary ? "Refazer resumo" : "Gerar resumo"}
                </button>
                {summary && (
                  <button className="secondary" onClick={copySummary}>
                    {sumCopied ? "Copiado!" : "Copiar resumo"}
                  </button>
                )}
              </div>
              {sumError && <p className="error">{sumError}</p>}
              {summary && <textarea className="transcript" readOnly value={summary} />}
            </div>
          )}
        </>
      )}
    </section>
  );
}

function ConfigScreen({
  settings,
  onSaved,
}: {
  settings: Settings | null;
  onSaved: () => void;
}) {
  const [defaultLanguage, setDefaultLanguage] = useState("pt");
  const [endpointUrl, setEndpointUrl] = useState("");
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [summaryEndpointUrl, setSummaryEndpointUrl] = useState("");
  const [summaryModel, setSummaryModel] = useState("");
  const [summaryKey, setSummaryKey] = useState("");
  const [icsUrl, setIcsUrl] = useState("");
  const [recordAll, setRecordAll] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (settings) {
      setDefaultLanguage(settings.default_language);
      setEndpointUrl(settings.endpoint_url);
      setModel(settings.model);
      setSummaryEndpointUrl(settings.summary_endpoint_url);
      setSummaryModel(settings.summary_model);
      setIcsUrl(settings.ics_url);
      setRecordAll(settings.record_all);
    }
  }, [settings]);

  async function save() {
    setError(null);
    setMsg(null);
    try {
      await invoke("save_settings", {
        defaultLanguage,
        endpointUrl,
        model,
        summaryEndpointUrl,
        summaryModel,
        icsUrl,
        recordAll,
      });
      if (apiKey.trim()) {
        await invoke("set_api_key", { key: apiKey });
        setApiKey("");
      }
      if (summaryKey.trim()) {
        await invoke("set_summary_key", { key: summaryKey });
        setSummaryKey("");
      }
      setMsg("Configurações salvas.");
      onSaved();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <section className="panel">
      <h2>Configurações</h2>

      <div className="form-row">
        <label>Idioma padrão</label>
        <select value={defaultLanguage} onChange={(e) => setDefaultLanguage(e.target.value)}>
          {LANGUAGES.map((l) => (
            <option key={l.code} value={l.code}>
              {l.label}
            </option>
          ))}
        </select>
      </div>

      <h3 className="cfg-section">Transcrição (Groq / Whisper)</h3>
      <p className="hint">Converte o áudio da reunião em texto.</p>
      <div className="form-row">
        <label>Endpoint</label>
        <input
          value={endpointUrl}
          onChange={(e) => setEndpointUrl(e.target.value)}
          placeholder="https://api.groq.com/openai/v1/audio/transcriptions"
        />
      </div>
      <div className="form-row">
        <label>Modelo</label>
        <input
          value={model}
          onChange={(e) => setModel(e.target.value)}
          placeholder="whisper-large-v3-turbo"
        />
      </div>
      <div className="form-row">
        <label>Chave da API</label>
        <input
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder={settings?.has_api_key ? "•••••• (configurada)" : "cole a chave da transcrição"}
        />
      </div>

      <h3 className="cfg-section">Resumo (MiniMax-M3) — opcional</h3>
      <p className="hint">Gera um resumo da reunião a partir da transcrição. Usa a chave sk-cp da MiniMax.</p>
      <div className="form-row">
        <label>Endpoint</label>
        <input
          value={summaryEndpointUrl}
          onChange={(e) => setSummaryEndpointUrl(e.target.value)}
          placeholder="https://api.minimax.io/v1/chat/completions"
        />
      </div>
      <div className="form-row">
        <label>Modelo</label>
        <input
          value={summaryModel}
          onChange={(e) => setSummaryModel(e.target.value)}
          placeholder="MiniMax-M3"
        />
      </div>
      <div className="form-row">
        <label>Chave da API (sk-cp)</label>
        <input
          type="password"
          value={summaryKey}
          onChange={(e) => setSummaryKey(e.target.value)}
          placeholder={settings?.has_summary_key ? "•••••• (configurada)" : "cole a chave do resumo"}
        />
      </div>

      <p className="hint">As chaves ficam no keychain do sistema, nunca em texto puro.</p>

      <h3 className="cfg-section">Calendário (agenda)</h3>
      <p className="hint">URL secreta (ICS) do seu calendário, para listar as próximas reuniões.</p>
      <div className="form-row">
        <label>URL do calendário (ICS)</label>
        <input
          value={icsUrl}
          onChange={(e) => setIcsUrl(e.target.value)}
          placeholder="https://calendar.google.com/.../basic.ics"
        />
      </div>
      <label className="chk">
        <input
          type="checkbox"
          checked={recordAll}
          onChange={(e) => setRecordAll(e.target.checked)}
        />
        Gravar todas as reuniões por padrão
      </label>

      <div className="actions">
        <button onClick={save}>Salvar</button>
      </div>

      {msg && <p className="ok">{msg}</p>}
      {error && <p className="error">{error}</p>}
    </section>
  );
}

function formatTime(s: number): string {
  const m = Math.floor(s / 60);
  const sec = s % 60;
  return `${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
}

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}

function formatDate(ms: number): string {
  return new Date(ms).toLocaleString("pt-BR");
}

function formatMeetingTime(start: number, end: number): string {
  const s = new Date(start);
  const e = new Date(end);
  const day = s.toLocaleDateString("pt-BR", { weekday: "short", day: "2-digit", month: "2-digit" });
  const t = (d: Date) => d.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
  return `${day}, ${t(s)}–${t(e)}`;
}

export default App;
