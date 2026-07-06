import { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { save } from "@tauri-apps/plugin-dialog";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import "./App.css";

// Registra um erro do frontend no log persistente (callrec.log).
function logClient(category: string, message: unknown) {
  invoke("log_client", { category, message: String(message) }).catch(() => {});
}

type Tab = "agenda" | "gravacoes" | "config";

type Meeting = {
  uid: string;
  title: string;
  starts_at: number;
  ends_at: number;
  record_enabled: boolean;
  participants: string[];
  location: string | null;
  link: string | null;
};

type Recording = {
  id: string;
  title: string;
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
  has_attio_key: boolean;
  attio_user_email: string;
  theme: string;
  auto_sync_agenda: boolean;
};

type AttioMeeting = {
  meeting_id: string;
  title: string;
  start: string | null;
  end: string | null;
  participants: string[];
};

type Summary = {
  recording_id: string;
  text: string;
  created_at: number;
};

const TABS: { id: Tab; label: string }[] = [
  { id: "agenda", label: "Home" },
  { id: "gravacoes", label: "Gravações" },
];

const LANGUAGES: { code: string; label: string }[] = [
  { code: "pt", label: "Português (BR)" },
  { code: "en", label: "Inglês" },
  { code: "es", label: "Espanhol" },
  { code: "fr", label: "Francês" },
  { code: "de", label: "Alemão" },
  { code: "it", label: "Italiano" },
];

type Provider = {
  id: string;
  label: string;
  endpoint: string;
  model: string;
  models: string[];
  keyHelp: string;
};

// Speech-to-text: compatíveis com a API OpenAI de transcrição (multipart Whisper).
const STT_PROVIDERS: Provider[] = [
  {
    id: "groq",
    label: "Groq (Whisper)",
    endpoint: "https://api.groq.com/openai/v1/audio/transcriptions",
    model: "whisper-large-v3-turbo",
    models: ["whisper-large-v3-turbo", "whisper-large-v3", "distil-whisper-large-v3-en"],
    keyHelp: "Chave grátis em console.groq.com/keys (Groq → API Keys → Create API Key).",
  },
  {
    id: "openai",
    label: "OpenAI (Whisper)",
    endpoint: "https://api.openai.com/v1/audio/transcriptions",
    model: "whisper-1",
    models: ["whisper-1", "gpt-4o-transcribe", "gpt-4o-mini-transcribe"],
    keyHelp: "platform.openai.com/api-keys (OpenAI → API keys → Create new secret key).",
  },
  {
    id: "fireworks",
    label: "Fireworks AI (Whisper)",
    endpoint: "https://api.fireworks.ai/inference/v1/audio/transcriptions",
    model: "whisper-v3",
    models: ["whisper-v3", "whisper-v3-turbo"],
    keyHelp: "fireworks.ai → Account → API Keys.",
  },
  {
    id: "custom",
    label: "Personalizado",
    endpoint: "",
    model: "",
    models: [],
    keyHelp: "Informe um endpoint compatível com a API OpenAI de transcrição.",
  },
];

// Resumo: compatíveis com a API OpenAI de chat completions.
const SUMMARY_PROVIDERS: Provider[] = [
  {
    id: "openai",
    label: "OpenAI (GPT)",
    endpoint: "https://api.openai.com/v1/chat/completions",
    model: "gpt-4o-mini",
    models: ["gpt-4o-mini", "gpt-4o", "gpt-4.1", "gpt-4.1-mini", "o4-mini"],
    keyHelp: "platform.openai.com/api-keys (OpenAI → API keys → Create new secret key).",
  },
  {
    id: "anthropic",
    label: "Claude (Anthropic)",
    endpoint: "https://api.anthropic.com/v1/chat/completions",
    model: "claude-3-5-sonnet-latest",
    models: [
      "claude-3-5-sonnet-latest",
      "claude-3-7-sonnet-latest",
      "claude-3-5-haiku-latest",
      "claude-3-opus-latest",
    ],
    keyHelp: "console.anthropic.com/settings/keys (Anthropic → API Keys). Endpoint compatível com OpenAI.",
  },
  {
    id: "gemini",
    label: "Google Gemini",
    endpoint: "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
    model: "gemini-2.0-flash",
    models: ["gemini-2.0-flash", "gemini-2.0-flash-lite", "gemini-1.5-pro", "gemini-1.5-flash"],
    keyHelp: "aistudio.google.com/apikey (Google AI Studio → Get API key).",
  },
  {
    id: "minimax_sub",
    label: "MiniMax (Subscription sk-cp)",
    endpoint: "https://api.minimax.io/v1/chat/completions",
    model: "MiniMax-M3",
    models: ["MiniMax-M3", "MiniMax-Text-01"],
    keyHelp: "Use a Subscription Key sk-cp da sua conta MiniMax (menu da conta → Subscription Key).",
  },
  {
    id: "minimax_api",
    label: "MiniMax (API)",
    endpoint: "https://api.minimax.io/v1/chat/completions",
    model: "MiniMax-M3",
    models: ["MiniMax-M3", "MiniMax-Text-01"],
    keyHelp: "platform.minimax.io → Account → API Keys (chave de API, começa com ey...).",
  },
  {
    id: "custom",
    label: "Personalizado",
    endpoint: "",
    model: "",
    models: [],
    keyHelp: "Informe um endpoint compatível com a API OpenAI de chat completions.",
  },
];

// Descobre o provedor salvo a partir do endpoint (para pré-selecionar o select).
function providerFromEndpoint(list: Provider[], endpoint: string): string {
  const hit = list.find((p) => p.id !== "custom" && p.endpoint === endpoint);
  return hit ? hit.id : "custom";
}

// Opções de modelo do provedor; inclui o modelo salvo caso não esteja na lista.
function modelOptions(list: Provider[], providerId: string, current: string): string[] {
  const models = list.find((p) => p.id === providerId)?.models ?? [];
  return current && !models.includes(current) ? [current, ...models] : models;
}

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
    case "play":
      return (
        <svg {...c} fill="currentColor" stroke="none">
          <path d="M8 5v14l11-7z" />
        </svg>
      );
    case "stop":
      return (
        <svg {...c} fill="currentColor" stroke="none">
          <rect x="6" y="6" width="12" height="12" rx="2" />
        </svg>
      );
    case "rename":
      return (
        <svg {...c}>
          <path d="M12 20h9" />
          <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4 12.5-12.5z" />
        </svg>
      );
    case "delete":
      return (
        <svg {...c}>
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      );
    case "export":
      return (
        <svg {...c}>
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <polyline points="7 9 12 4 17 9" />
          <line x1="12" y1="4" x2="12" y2="16" />
        </svg>
      );
    default:
      return null;
  }
}

function App() {
  const [tab, setTab] = useState<Tab>("agenda");
  const [recordings, setRecordings] = useState<Recording[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [update, setUpdate] = useState<{ version: string } | null>(null);
  const [updating, setUpdating] = useState<string | null>(null);

  // Janela-toast de reunião começando (aberta pelo scheduler com ?alert=1).
  const isAlert = new URLSearchParams(window.location.search).get("alert") === "1";

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

  // Aplica o tema escolhido no <html> (system=sem atributo, segue o SO).
  useEffect(() => {
    const t = settings?.theme ?? "system";
    if (t === "system") delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = t;
  }, [settings?.theme]);

  useEffect(() => {
    const un = listen("recording-changed", () => refreshRecordings());
    return () => {
      un.then((f) => f());
    };
  }, [refreshRecordings]);

  // Verifica atualização no início e uma vez por dia. Falha silenciosa
  // (offline, sem manifesto) — só avisa quando há versão nova.
  useEffect(() => {
    if (isAlert) return;
    let cancelled = false;
    const run = async () => {
      try {
        const u = await check();
        if (u && !cancelled) setUpdate({ version: u.version });
      } catch {
        /* offline ou sem update */
      }
    };
    run();
    const id = window.setInterval(run, 24 * 60 * 60 * 1000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [isAlert]);

  async function applyUpdate() {
    setUpdating("Baixando atualização...");
    try {
      const u = await check();
      if (!u) {
        setUpdate(null);
        setUpdating(null);
        return;
      }
      await u.downloadAndInstall((e) => {
        if (e.event === "Progress") setUpdating("Baixando atualização...");
        else if (e.event === "Finished") setUpdating("Instalando...");
      });
      await relaunch();
    } catch (e) {
      setUpdating(null);
      logClient("updater", e);
      alert("Falha ao atualizar: " + String(e));
    }
  }

  if (isAlert) return <MeetingAlert />;

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="brand">
          <img className="brand-logo" src="/icon.png" alt="" />
          Hicorder
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
        <button
          className={tab === "config" ? "nav-item gear active" : "nav-item gear"}
          onClick={() => setTab("config")}
          title="Configurações"
          aria-label="Configurações"
        >
          {icon("config")}
        </button>
      </nav>

      <main className="content">
        {update && (
          <div className="update-banner">
            <span>Nova versão {update.version} disponível.</span>
            <button onClick={applyUpdate} disabled={!!updating}>
              {updating ?? "Atualizar agora"}
            </button>
          </div>
        )}
        {tab === "agenda" && (
          <HomeScreen
            hasIcs={!!settings?.ics_url}
            recordAll={settings?.record_all ?? false}
            autoSync={settings?.auto_sync_agenda ?? true}
            onFinished={refreshRecordings}
          />
        )}
        {tab === "gravacoes" && (
          <GravacoesScreen
            recordings={recordings}
            defaultLanguage={settings?.default_language ?? "pt"}
            hasApiKey={settings?.has_api_key ?? false}
            hasSummaryKey={settings?.has_summary_key ?? false}
            hasAttioKey={settings?.has_attio_key ?? false}
            attioUserEmail={settings?.attio_user_email ?? ""}
            onChanged={refreshRecordings}
          />
        )}
        {tab === "config" && <ConfigScreen settings={settings} onSaved={refreshSettings} />}
      </main>
    </div>
  );
}

/// Transcrição em formato de chat: "Você" à direita, "Participantes" à esquerda.
function ChatTranscript({ text }: { text: string }) {
  const re = /^\[(\d{2}:\d{2}(?::\d{2})?)\]\s*(Você|Participantes):\s*(.*)$/;
  const lines = text.split("\n");
  const parsed = lines.map((l) => {
    const m = l.match(re);
    return m ? { time: m[1], who: m[2], text: m[3] } : null;
  });

  // Transcrição fora do formato esperado: mantém o texto puro.
  if (!parsed.some(Boolean)) {
    return <textarea className="transcript" readOnly value={text} />;
  }

  return (
    <div className="chat">
      {parsed.map((p, i) =>
        p ? (
          <div key={i} className={p.who === "Você" ? "chat-msg me" : "chat-msg them"}>
            <div className="chat-bubble">
              <span className="chat-meta">
                {p.who} · {p.time}
              </span>
              {p.text}
            </div>
          </div>
        ) : (
          lines[i].trim() && (
            <div key={i} className="chat-msg them">
              <div className="chat-bubble">{lines[i]}</div>
            </div>
          )
        ),
      )}
    </div>
  );
}

/// Janela-toast: reunião começando, com botão de iniciar gravação.
function MeetingAlert() {
  const params = new URLSearchParams(window.location.search);
  const title = params.get("title") ?? "Reunião";
  const endMs = Number(params.get("end") ?? 0);
  const [busy, setBusy] = useState(false);

  async function close() {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  }

  async function record() {
    setBusy(true);
    try {
      await invoke("start_meeting_recording", { endMs, title });
      await close();
    } catch {
      setBusy(false);
    }
  }

  return (
    <div className="meeting-toast">
      <div className="meeting-toast-body">
        <img src="/icon.png" alt="" width={32} height={32} />
        <div>
          <strong>Reunião começando</strong>
          <p>{title}</p>
        </div>
      </div>
      <div className="meeting-toast-actions">
        <button onClick={record} disabled={busy}>
          {busy ? "Iniciando..." : "Iniciar gravação"}
        </button>
        <button className="secondary" onClick={close}>
          Dispensar
        </button>
      </div>
    </div>
  );
}

/// Home: barra de gravação fixa no topo + agenda.
function HomeScreen({
  hasIcs,
  recordAll,
  autoSync,
  onFinished,
}: {
  hasIcs: boolean;
  recordAll: boolean;
  autoSync: boolean;
  onFinished: () => void;
}) {
  return (
    <section className="panel">
      <RecordBar onFinished={onFinished} />
      <AgendaList hasIcs={hasIcs} recordAll={recordAll} autoSync={autoSync} />
    </section>
  );
}

function RecordBar({ onFinished }: { onFinished: () => void }) {
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
    <div className="record-bar">
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
      {error && <p className="error">{error}</p>}
    </div>
  );
}

function AgendaList({
  hasIcs,
  recordAll,
  autoSync,
}: {
  hasIcs: boolean;
  recordAll: boolean;
  autoSync: boolean;
}) {
  const [meetings, setMeetings] = useState<Meeting[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [now, setNow] = useState(Date.now());
  const [recording, setRecording] = useState(false);
  const autoRefreshed = useRef(false);

  useEffect(() => {
    invoke<Meeting[]>("list_meetings").then(setMeetings).catch(() => {});
    invoke<boolean>("is_recording").then(setRecording).catch(() => {});
    // Agenda atualizada no boot pelo backend chega por evento.
    const un = listen<Meeting[]>("meetings-refreshed", (e) => setMeetings(e.payload));
    const unRec = listen<boolean>("recording-changed", (e) => setRecording(e.payload));
    // Reavalia "acontecendo agora" periodicamente.
    const t = window.setInterval(() => setNow(Date.now()), 20000);
    return () => {
      un.then((f) => f());
      unRec.then((f) => f());
      window.clearInterval(t);
    };
  }, []);

  async function startNow(m: Meeting) {
    setError(null);
    try {
      await invoke("start_meeting_recording", { endMs: m.ends_at, title: m.title });
    } catch (e) {
      setError(String(e));
    }
  }

  // Ao abrir a Home com ICS configurado e auto-sync ligado, atualiza 1x.
  useEffect(() => {
    if (hasIcs && autoSync && !autoRefreshed.current) {
      autoRefreshed.current = true;
      invoke<Meeting[]>("refresh_meetings").then(setMeetings).catch(() => {});
    }
  }, [hasIcs, autoSync]);

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
    <>
      <div className="agenda-head">
        <h2>Agenda</h2>
        <button className="secondary" onClick={refresh} disabled={busy}>
          {busy ? "Atualizando..." : "Atualizar"}
        </button>
      </div>
      {recordAll && (
        <p className="banner">
          Gravar todas as reuniões está habilitado em Configurações — todas as reuniões serão
          gravadas automaticamente.
        </p>
      )}
      {!hasIcs && <p className="hint">Configure a URL do calendário (ICS) em Configurações.</p>}
      {error && <p className="error">{error}</p>}
      {meetings.length === 0 ? (
        <div className="empty">
          {icon("agenda")}
          <p>Nenhuma reunião próxima. Clique em Atualizar.</p>
        </div>
      ) : (
        <ul className="rec-list">
          {meetings.map((m) => {
            const happening = now >= m.starts_at && now < m.ends_at;
            return (
              <li key={m.uid} className={happening ? "meeting-now" : ""}>
                <div className="rec-row">
                  <div className="rec-meta">
                    <span className="meeting-title">
                      {m.title}
                      {happening && <span className="now-badge">Agora</span>}
                    </span>
                    <small>{formatMeetingTime(m.starts_at, m.ends_at)}</small>
                    {m.participants.length > 0 && (
                      <small className="meeting-extra">{m.participants.join(", ")}</small>
                    )}
                    {m.location && !isUrl(m.location) && (
                      <small className="meeting-extra">Local: {m.location}</small>
                    )}
                  </div>
                  <div className="meeting-actions">
                    {m.link && (
                      <button
                        className="call-btn"
                        onClick={() => openUrl(m.link!).catch((e) => logClient("call-link", e))}
                      >
                        Entrar na call
                      </button>
                    )}
                    <button
                      className="icon-btn"
                      onClick={() => startNow(m)}
                      disabled={recording}
                      title={recording ? "Já existe uma gravação em andamento" : ""}
                    >
                      {icon("play")}
                      Iniciar Gravação
                    </button>
                    <label className="chk" title={recordAll ? "Gravar todas está habilitado" : ""}>
                      <input
                        type="checkbox"
                        checked={recordAll || m.record_enabled}
                        disabled={recordAll}
                        onChange={(e) => toggle(m.uid, e.target.checked)}
                      />
                      Agendar Gravação
                    </label>
                  </div>
                </div>
              </li>
            );
          })}
        </ul>
      )}
    </>
  );
}

function isUrl(s: string): boolean {
  return s.startsWith("http://") || s.startsWith("https://");
}

function GravacoesScreen({
  recordings,
  defaultLanguage,
  hasApiKey,
  hasSummaryKey,
  hasAttioKey,
  attioUserEmail,
  onChanged,
}: {
  recordings: Recording[];
  defaultLanguage: string;
  hasApiKey: boolean;
  hasSummaryKey: boolean;
  hasAttioKey: boolean;
  attioUserEmail: string;
  onChanged: () => void;
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
  const [playing, setPlaying] = useState(false);
  const [playSrc, setPlaySrc] = useState<string | null>(null);
  const [preparing, setPreparing] = useState(false);
  const [exportFmt, setExportFmt] = useState("mp3");
  const [exporting, setExporting] = useState(false);
  const [actionMsg, setActionMsg] = useState<string | null>(null);

  const selected = recordings.find((r) => r.id === selectedId) ?? null;

  useEffect(() => {
    if (!selectedId && recordings.length > 0) setSelectedId(recordings[0].id);
  }, [recordings, selectedId]);

  useEffect(() => {
    setPlaying(false);
    setPlaySrc(null);
    setActionMsg(null);
  }, [selectedId]);

  // Prepara a faixa mixada sob demanda (mic+sistema) e toca. Na 1a vez pode
  // levar alguns segundos (mixagem); depois fica em cache.
  async function togglePlay() {
    if (playing) {
      setPlaying(false);
      setPlaySrc(null);
      return;
    }
    if (!selected) return;
    setPlaying(true);
    setPreparing(true);
    setError(null);
    try {
      const path = await invoke<string>("prepare_playback", { recordingId: selected.id });
      setPlaySrc(convertFileSrc(path));
    } catch (e) {
      setError(String(e));
      setPlaying(false);
    } finally {
      setPreparing(false);
    }
  }

  async function renameSel() {
    if (!selected) return;
    const title = window.prompt("Novo nome da gravação:", selected.title);
    if (title === null || !title.trim()) return;
    try {
      await invoke("rename_recording", { recordingId: selected.id, title: title.trim() });
      onChanged();
    } catch (e) {
      setError(String(e));
    }
  }

  async function removeSel() {
    if (!selected) return;
    if (!window.confirm("Apagar esta gravação e sua transcrição? Não dá pra desfazer.")) return;
    try {
      await invoke("delete_recording", { recordingId: selected.id });
      const rest = recordings.filter((r) => r.id !== selected.id);
      setSelectedId(rest[0]?.id ?? "");
      onChanged();
    } catch (e) {
      setError(String(e));
    }
  }

  async function exportAudio() {
    if (!selected) return;
    setError(null);
    setActionMsg(null);
    const safe = selected.title.replace(/[\\/:*?"<>|]/g, "_").slice(0, 80) || "gravacao";
    const dest = await save({
      defaultPath: `${safe}.${exportFmt}`,
      filters: [{ name: exportFmt.toUpperCase(), extensions: [exportFmt] }],
    });
    if (!dest) return;
    setExporting(true);
    try {
      await invoke("export_audio", { recordingId: selected.id, destPath: dest });
      setActionMsg("Áudio exportado.");
    } catch (e) {
      setError(String(e));
    } finally {
      setExporting(false);
    }
  }

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
      <h2>Gravações</h2>
      {recordings.length === 0 ? (
        <div className="empty">
          {icon("gravacoes")}
          <p>Nenhuma gravação ainda. Grave pelo botão na Home.</p>
        </div>
      ) : (
        <>
          <div className="form-row">
            <label>Gravação</label>
            <select value={selectedId} onChange={(e) => setSelectedId(e.target.value)}>
              {recordings.map((r) => (
                <option key={r.id} value={r.id}>
                  {r.title} — {formatDate(r.created_at)} · {formatTime(Math.round(r.duration_s))} ·{" "}
                  {formatSize(r.size_bytes)}
                </option>
              ))}
            </select>
          </div>

          {selected && (
            <div className="rec-toolbar">
              <button className="icon-btn" onClick={togglePlay} disabled={preparing}>
                {icon(playing ? "stop" : "play")}
                {preparing ? "Preparando..." : playing ? "Fechar player" : "Play"}
              </button>
              <button className="icon-btn" onClick={renameSel}>
                {icon("rename")}
                Renomear
              </button>
              <span className="export-group">
                <button className="icon-btn" onClick={exportAudio} disabled={exporting}>
                  {icon("export")}
                  {exporting ? "Exportando..." : "Exportar áudio"}
                </button>
                <select value={exportFmt} onChange={(e) => setExportFmt(e.target.value)}>
                  <option value="mp3">MP3</option>
                  <option value="wav">WAV</option>
                  <option value="ogg">OGG</option>
                </select>
              </span>
              <button className="icon-btn danger" onClick={removeSel}>
                {icon("delete")}
                Apagar
              </button>
            </div>
          )}
          {playing && playSrc && (
            <audio
              className="player"
              controls
              autoPlay
              src={playSrc}
              onError={() => {
                setError("Não foi possível reproduzir esta gravação.");
                logClient("player", `falha ao reproduzir ${selected?.path ?? ""}`);
              }}
            />
          )}
          {actionMsg && <p className="ok">{actionMsg}</p>}

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
          {text && <ChatTranscript text={text} />}

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

          {text && (
            <AttioUpload
              recording={recordings.find((r) => r.id === selectedId) ?? null}
              hasSummary={!!summary}
              hasAttioKey={hasAttioKey}
              userEmail={attioUserEmail}
            />
          )}
        </>
      )}
    </section>
  );
}

function AttioUpload({
  recording,
  hasSummary,
  hasAttioKey,
  userEmail,
}: {
  recording: Recording | null;
  hasSummary: boolean;
  hasAttioKey: boolean;
  userEmail: string;
}) {
  const [kind, setKind] = useState<"transcript" | "summary" | null>(null);
  const [title, setTitle] = useState("");
  const [candidates, setCandidates] = useState<AttioMeeting[] | null>(null);
  const [selected, setSelected] = useState<string | null>(null); // meeting_id or "new"
  const [checkedEmails, setCheckedEmails] = useState<Set<string>>(new Set());
  const [manualEmails, setManualEmails] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);

  function parseManual(): string[] {
    return manualEmails
      .split(/[\s,;]+/)
      .map((e) => e.trim())
      .filter((e) => e.includes("@"));
  }

  // Emails finais = sugeridos marcados + digitados manualmente (dedup).
  function finalEmails(): string[] {
    return Array.from(new Set([...checkedEmails, ...parseManual()]));
  }

  // Ao escolher uma reunião, sugere seus participantes já marcados — exceto o
  // próprio usuário (email do Attio nas Configurações), que vem desmarcado.
  function pick(meeting: AttioMeeting | "new") {
    if (meeting === "new") {
      setSelected("new");
      setCheckedEmails(new Set());
    } else {
      setSelected(meeting.meeting_id);
      const me = userEmail.trim().toLowerCase();
      setCheckedEmails(
        new Set(meeting.participants.filter((p) => p.trim().toLowerCase() !== me)),
      );
    }
  }

  function toggleEmail(email: string) {
    setCheckedEmails((prev) => {
      const next = new Set(prev);
      if (next.has(email)) next.delete(email);
      else next.add(email);
      return next;
    });
  }

  async function start(k: "transcript" | "summary") {
    setKind(k);
    setCandidates(null);
    setSelected(null);
    setCheckedEmails(new Set());
    setManualEmails("");
    setTitle("");
    setResult(null);
    setError(null);
    if (!recording) return;
    // Janela: reuniões que terminam depois do início da gravação (folga 30min p/
    // gravação iniciada tarde) até as que começam antes do fim (+30min).
    const buffer = 30 * 60 * 1000;
    const endsFrom = new Date(recording.created_at - buffer).toISOString();
    const startsBefore = new Date(
      recording.created_at + recording.duration_s * 1000 + buffer,
    ).toISOString();
    const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    setBusy(true);
    try {
      const found = await invoke<AttioMeeting[]>("attio_find_meetings", {
        endsFrom,
        startsBefore,
        timezone,
        emails: [],
      });
      setCandidates(found);
      if (found.length > 0) pick(found[0]);
      else pick("new");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function upload() {
    if (!recording || !kind || !selected) return;
    const list = finalEmails();
    if (list.length === 0) {
      setError("Marque ou informe ao menos 1 email para receber a nota.");
      return;
    }
    setError(null);
    setResult(null);
    setBusy(true);
    try {
      const start = new Date(recording.created_at);
      const end = new Date(recording.created_at + recording.duration_s * 1000);
      const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
      const r = await invoke<{ meeting_id: string; notes_created: number; missing_people: string[] }>(
        "attio_upload",
        {
          recordingId: recording.id,
          kind,
          meetingId: selected === "new" ? null : selected,
          title: title || `Reunião ${formatDate(recording.created_at)}`,
          startIso: start.toISOString(),
          endIso: end.toISOString(),
          timezone: tz,
          emails: list,
        },
      );
      let msg = `${r.notes_created} nota(s) criada(s) na meeting ${r.meeting_id}.`;
      if (r.missing_people.length > 0) msg += ` Sem pessoa no Attio: ${r.missing_people.join(", ")}.`;
      setResult(msg);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const selectedMeeting =
    candidates?.find((m) => m.meeting_id === selected) ?? null;

  return (
    <div className="summary-block">
      <h3>Subir ao Attio</h3>
      {!hasAttioKey && <p className="hint">Configure a chave do Attio em Configurações.</p>}

      <div className="actions">
        <button
          className={kind === "transcript" ? "" : "secondary"}
          onClick={() => start("transcript")}
          disabled={!hasAttioKey || busy}
        >
          Subir transcrição
        </button>
        <button
          className={kind === "summary" ? "" : "secondary"}
          onClick={() => start("summary")}
          disabled={!hasAttioKey || !hasSummary || busy}
        >
          Subir resumo
        </button>
      </div>

      {kind && busy && !candidates && <p className="hint">Buscando reuniões no horário...</p>}

      {candidates && (
        <div className="attio-candidates">
          <p className="hint">1. Escolha a reunião (buscada pelo horário da gravação):</p>
          {candidates.map((m) => (
            <label key={m.meeting_id} className="chk">
              <input
                type="radio"
                name="attio-meeting"
                checked={selected === m.meeting_id}
                onChange={() => pick(m)}
              />
              <span>
                <strong>{m.title}</strong>
                {m.start && <> — {new Date(m.start).toLocaleString("pt-BR")}</>}
              </span>
            </label>
          ))}
          {candidates.length === 0 && (
            <p className="hint">Nenhuma reunião nesse horário. Crie uma nova abaixo.</p>
          )}
          <label className="chk">
            <input
              type="radio"
              name="attio-meeting"
              checked={selected === "new"}
              onChange={() => pick("new")}
            />
            <span>➕ Criar nova reunião</span>
          </label>

          {selected === "new" && (
            <div className="form-row">
              <label>Nome da nova reunião</label>
              <input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder={`Reunião ${recording ? formatDate(recording.created_at) : ""}`}
              />
            </div>
          )}

          <p className="hint">2. Quem recebe a nota:</p>
          {selectedMeeting && selectedMeeting.participants.length > 0 && (
            <div className="attio-people">
              {selectedMeeting.participants.map((email) => (
                <label key={email} className="chk">
                  <input
                    type="checkbox"
                    checked={checkedEmails.has(email)}
                    onChange={() => toggleEmail(email)}
                  />
                  <span>{email}</span>
                </label>
              ))}
            </div>
          )}
          <div className="form-row">
            <label>Outros emails (opcional, separados por vírgula)</label>
            <input
              value={manualEmails}
              onChange={(e) => setManualEmails(e.target.value)}
              placeholder="ana@empresa.com, bob@cliente.com"
            />
          </div>

          <div className="actions">
            <button onClick={upload} disabled={busy || !selected}>
              {busy ? "Subindo..." : "Confirmar e subir nota"}
            </button>
          </div>
        </div>
      )}

      {error && <p className="error">{error}</p>}
      {result && <p className="ok">{result}</p>}
    </div>
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
  const [sttProvider, setSttProvider] = useState("groq");
  const [endpointUrl, setEndpointUrl] = useState("");
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [summaryProvider, setSummaryProvider] = useState("minimax_sub");
  const [summaryEndpointUrl, setSummaryEndpointUrl] = useState("");
  const [summaryModel, setSummaryModel] = useState("");
  const [summaryKey, setSummaryKey] = useState("");
  const [attioKey, setAttioKey] = useState("");
  const [attioUserEmail, setAttioUserEmail] = useState("");
  const [testResult, setTestResult] = useState<Record<string, string>>({});
  const [logText, setLogText] = useState<string | null>(null);
  const [icsUrl, setIcsUrl] = useState("");
  const [recordAll, setRecordAll] = useState(false);
  const [autoSyncAgenda, setAutoSyncAgenda] = useState(true);
  const [autostart, setAutostart] = useState(true);
  const [theme, setTheme] = useState("system");
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function testApi(which: "stt" | "summary" | "attio") {
    setTestResult((t) => ({ ...t, [which]: "Testando..." }));
    try {
      let res: string;
      if (which === "stt") {
        res = await invoke<string>("test_transcription_api", {
          endpointUrl,
          key: apiKey.trim() || null,
        });
      } else if (which === "summary") {
        res = await invoke<string>("test_summary_api", {
          endpointUrl: summaryEndpointUrl,
          model: summaryModel,
          key: summaryKey.trim() || null,
        });
      } else {
        res = await invoke<string>("test_attio_api", { key: attioKey.trim() || null });
      }
      setTestResult((t) => ({ ...t, [which]: res }));
    } catch (e) {
      setTestResult((t) => ({ ...t, [which]: String(e) }));
    }
  }

  useEffect(() => {
    if (settings) {
      setDefaultLanguage(settings.default_language);
      setEndpointUrl(settings.endpoint_url);
      setModel(settings.model);
      setSttProvider(providerFromEndpoint(STT_PROVIDERS, settings.endpoint_url));
      setSummaryEndpointUrl(settings.summary_endpoint_url);
      setSummaryModel(settings.summary_model);
      setSummaryProvider(providerFromEndpoint(SUMMARY_PROVIDERS, settings.summary_endpoint_url));
      setIcsUrl(settings.ics_url);
      setRecordAll(settings.record_all);
      setAutoSyncAgenda(settings.auto_sync_agenda);
      setAttioUserEmail(settings.attio_user_email);
      setTheme(settings.theme);
    }
  }, [settings]);

  // Estado da autoinicialização (nível do SO, não fica no AppSettings).
  useEffect(() => {
    invoke<boolean>("get_autostart").then(setAutostart).catch(() => {});
  }, []);

  async function toggleAutostart(enabled: boolean) {
    setAutostart(enabled);
    try {
      await invoke("set_autostart", { enabled });
    } catch (e) {
      setError(String(e));
      setAutostart(!enabled); // reverte se falhar
    }
  }

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
        attioUserEmail,
        theme,
        autoSyncAgenda,
      });
      if (apiKey.trim()) {
        await invoke("set_api_key", { key: apiKey });
        setApiKey("");
      }
      if (summaryKey.trim()) {
        await invoke("set_summary_key", { key: summaryKey });
        setSummaryKey("");
      }
      if (attioKey.trim()) {
        await invoke("set_attio_key", { key: attioKey });
        setAttioKey("");
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

      <div className="form-row">
        <label>Tema</label>
        <select
          value={theme}
          onChange={(e) => {
            const t = e.target.value;
            setTheme(t);
            // Prévia imediata (persiste ao Salvar).
            if (t === "system") delete document.documentElement.dataset.theme;
            else document.documentElement.dataset.theme = t;
          }}
        >
          <option value="system">Automático (sistema)</option>
          <option value="light">Claro</option>
          <option value="dark">Escuro</option>
        </select>
      </div>

      <h3 className="cfg-section">Transcrição (speech-to-text)</h3>
      <p className="hint">Converte o áudio da reunião em texto.</p>
      <div className="form-row">
        <label>Provedor</label>
        <select
          value={sttProvider}
          onChange={(e) => {
            const id = e.target.value;
            setSttProvider(id);
            const p = STT_PROVIDERS.find((x) => x.id === id)!;
            if (id !== "custom") {
              setEndpointUrl(p.endpoint);
              setModel(p.model);
            }
          }}
        >
          {STT_PROVIDERS.map((p) => (
            <option key={p.id} value={p.id}>
              {p.label}
            </option>
          ))}
        </select>
        <span className="hint">{STT_PROVIDERS.find((p) => p.id === sttProvider)?.keyHelp}</span>
      </div>
      {sttProvider === "custom" && (
        <div className="form-row">
          <label>Endpoint</label>
          <input
            value={endpointUrl}
            onChange={(e) => setEndpointUrl(e.target.value)}
            placeholder="https://.../v1/audio/transcriptions"
          />
        </div>
      )}
      <div className="form-row">
        <label>Modelo</label>
        {sttProvider === "custom" ? (
          <input
            value={model}
            onChange={(e) => setModel(e.target.value)}
            placeholder="whisper-large-v3-turbo"
          />
        ) : (
          <select value={model} onChange={(e) => setModel(e.target.value)}>
            {modelOptions(STT_PROVIDERS, sttProvider, model).map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        )}
      </div>
      <div className="form-row">
        <label>Chave da API</label>
        <div className="key-row">
          <input
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder={settings?.has_api_key ? "•••••• (configurada)" : "cole a chave da transcrição"}
          />
          <button type="button" className="secondary" onClick={() => testApi("stt")}>
            Testar
          </button>
        </div>
        {testResult.stt && <TestLine text={testResult.stt} />}
      </div>

      <h3 className="cfg-section">Resumo (LLM) — opcional</h3>
      <p className="hint">Gera um resumo da reunião a partir da transcrição.</p>
      <div className="form-row">
        <label>Provedor</label>
        <select
          value={summaryProvider}
          onChange={(e) => {
            const id = e.target.value;
            setSummaryProvider(id);
            const p = SUMMARY_PROVIDERS.find((x) => x.id === id)!;
            if (id !== "custom") {
              setSummaryEndpointUrl(p.endpoint);
              setSummaryModel(p.model);
            }
          }}
        >
          {SUMMARY_PROVIDERS.map((p) => (
            <option key={p.id} value={p.id}>
              {p.label}
            </option>
          ))}
        </select>
        <span className="hint">{SUMMARY_PROVIDERS.find((p) => p.id === summaryProvider)?.keyHelp}</span>
      </div>
      {summaryProvider === "custom" && (
        <div className="form-row">
          <label>Endpoint</label>
          <input
            value={summaryEndpointUrl}
            onChange={(e) => setSummaryEndpointUrl(e.target.value)}
            placeholder="https://.../v1/chat/completions"
          />
        </div>
      )}
      <div className="form-row">
        <label>Modelo</label>
        {summaryProvider === "custom" ? (
          <input
            value={summaryModel}
            onChange={(e) => setSummaryModel(e.target.value)}
            placeholder="MiniMax-M3"
          />
        ) : (
          <select value={summaryModel} onChange={(e) => setSummaryModel(e.target.value)}>
            {modelOptions(SUMMARY_PROVIDERS, summaryProvider, summaryModel).map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        )}
      </div>
      <div className="form-row">
        <label>Chave da API</label>
        <div className="key-row">
          <input
            type="password"
            value={summaryKey}
            onChange={(e) => setSummaryKey(e.target.value)}
            placeholder={settings?.has_summary_key ? "•••••• (configurada)" : "cole a chave do resumo"}
          />
          <button type="button" className="secondary" onClick={() => testApi("summary")}>
            Testar
          </button>
        </div>
        {testResult.summary && <TestLine text={testResult.summary} />}
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
      <label className="chk">
        <input
          type="checkbox"
          checked={autoSyncAgenda}
          onChange={(e) => setAutoSyncAgenda(e.target.checked)}
        />
        Sincronizar a agenda automaticamente ao abrir o app
      </label>
      <label className="chk">
        <input
          type="checkbox"
          checked={autostart}
          onChange={(e) => toggleAutostart(e.target.checked)}
        />
        Iniciar o Hicorder junto com o sistema (recomendado, para gravar reuniões
        automaticamente)
      </label>

      <h3 className="cfg-section">Attio (CRM)</h3>
      <p className="hint">Sobe transcrição/resumo como nota na meeting do Attio. Chave em Attio → Settings → Developers → API tokens.</p>
      <div className="form-row">
        <label>Seu email no Attio</label>
        <input
          type="email"
          value={attioUserEmail}
          onChange={(e) => setAttioUserEmail(e.target.value)}
          placeholder="voce@hi.capital"
        />
        <span className="hint">Filtra as reuniões sugeridas às que você participa.</span>
      </div>
      <div className="form-row">
        <label>Chave da API (Attio)</label>
        <div className="key-row">
          <input
            type="password"
            value={attioKey}
            onChange={(e) => setAttioKey(e.target.value)}
            placeholder={settings?.has_attio_key ? "•••••• (configurada)" : "cole a chave do Attio"}
          />
          <button type="button" className="secondary" onClick={() => testApi("attio")}>
            Testar
          </button>
        </div>
        {testResult.attio && <TestLine text={testResult.attio} />}
      </div>

      <h3 className="cfg-section">Logs (troubleshooting)</h3>
      <p className="hint">Registro persistente dos erros de API para diagnóstico.</p>
      <div className="actions">
        <button
          type="button"
          className="secondary"
          onClick={async () => setLogText((await invoke<string>("get_logs")) || "(sem registros)")}
        >
          Ver logs
        </button>
        <button
          type="button"
          className="secondary"
          onClick={async () => {
            await invoke("clear_logs");
            setLogText("(sem registros)");
          }}
        >
          Limpar logs
        </button>
      </div>
      {logText !== null && <pre className="log-view">{logText}</pre>}

      <div className="actions">
        <button onClick={save}>Salvar</button>
      </div>

      {msg && <p className="ok">{msg}</p>}
      {error && <p className="error">{error}</p>}
    </section>
  );
}

// Mostra o resultado de um teste de API; verde se "OK", senão vermelho.
function TestLine({ text }: { text: string }) {
  const ok = /\bOK\b/.test(text);
  return <span className={ok ? "test-ok" : "test-err"}>{text}</span>;
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
