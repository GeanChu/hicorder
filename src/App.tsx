import { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { save, open } from "@tauri-apps/plugin-dialog";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";
import "./App.css";

// Registra um erro do frontend no log persistente (callrec.log).
function logClient(category: string, message: unknown) {
  invoke("log_client", { category, message: String(message) }).catch(() => {});
}

type Tab = "agenda" | "gravacoes" | "prompts" | "config";

type SummaryPrompt = {
  id: string;
  name: string;
  text: string;
  created_at: number;
};

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
  summary_prompt: string;
  ics_url: string;
  record_all: boolean;
  has_attio_key: boolean;
  attio_user_email: string;
  theme: string;
  auto_sync_agenda: boolean;
  auto_stop_minutes: number;
};

type AttioMeeting = {
  meeting_id: string;
  title: string;
  start: string | null;
  end: string | null;
  participants: string[];
};

type AttioCompany = {
  record_id: string;
  name: string;
};

type Summary = {
  recording_id: string;
  text: string;
  created_at: number;
};

const TABS: { id: Tab; label: string }[] = [
  { id: "agenda", label: "Home" },
  { id: "gravacoes", label: "Gravações" },
  { id: "prompts", label: "Prompts de resumo" },
];

// Opções do auto-stop por tempo (em minutos).
const AUTO_STOP_OPTIONS: { min: number; label: string }[] = [
  { min: 15, label: "15min" },
  { min: 30, label: "30min" },
  { min: 60, label: "1h" },
  { min: 120, label: "2h" },
  { min: 180, label: "3h" },
  { min: 240, label: "4h" },
  { min: 300, label: "5h" },
  { min: 360, label: "6h" },
  { min: 720, label: "12h" },
  { min: 1440, label: "24h" },
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
    model: "whisper-large-v3",
    models: ["whisper-large-v3", "whisper-large-v3-turbo"],
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
    id: "nvidia",
    label: "NVIDIA NIM",
    endpoint: "https://integrate.api.nvidia.com/v1/chat/completions",
    model: "minimaxai/minimax-m3",
    models: [
      "minimaxai/minimax-m3",
      "deepseek-ai/deepseek-v4-pro",
      "meta/llama-3.3-70b-instruct",
      "deepseek-ai/deepseek-r1",
    ],
    keyHelp: "build.nvidia.com → API key (começa com nvapi-). Endpoint compatível com OpenAI.",
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
    case "prompts":
      return (
        <svg {...c}>
          <path d="M4 6h16" />
          <path d="M4 12h10" />
          <path d="M4 18h7" />
          <path d="M18 15l3 3-3 3" />
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
  const [prompts, setPrompts] = useState<SummaryPrompt[]>([]);
  const [update, setUpdate] = useState<{ version: string } | null>(null);
  const [updating, setUpdating] = useState<string | null>(null);

  const refreshPrompts = useCallback(async () => {
    try {
      setPrompts(await invoke<SummaryPrompt[]>("list_summary_prompts"));
    } catch {
      /* ignore */
    }
  }, []);

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
    refreshPrompts();
  }, [refreshRecordings, refreshSettings, refreshPrompts]);

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
            baseSummaryPrompt={settings?.summary_prompt ?? ""}
            prompts={prompts}
            onChanged={refreshRecordings}
          />
        )}
        {tab === "prompts" && <PromptsScreen prompts={prompts} onChanged={refreshPrompts} />}
        {tab === "config" && <ConfigScreen settings={settings} onSaved={refreshSettings} />}
      </main>
    </div>
  );
}

/// Transcrição em formato de chat: "Você" à direita, "Participantes" à esquerda.
function ChatTranscript({ text, query = "" }: { text: string; query?: string }) {
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

  // Com busca ativa, mostra só as falas que casam (destacadas).
  const q = query.trim().toLowerCase();
  const hits = parsed.map((p, i) => {
    const line = p ? p.text : lines[i];
    return !q || (line ?? "").toLowerCase().includes(q);
  });

  return (
    <div className="chat">
      {parsed.map((p, i) => {
        if (!hits[i]) return null;
        return p ? (
          <div key={i} className={p.who === "Você" ? "chat-msg me" : "chat-msg them"}>
            <div className="chat-bubble">
              <span className="chat-meta">
                {p.who} · {p.time}
              </span>
              <Highlight text={p.text} query={query} />
            </div>
          </div>
        ) : (
          lines[i].trim() && (
            <div key={i} className="chat-msg them">
              <div className="chat-bubble">
                <Highlight text={lines[i]} query={query} />
              </div>
            </div>
          )
        );
      })}
      {q && !hits.some(Boolean) && <p className="hint">Nenhum trecho encontrado.</p>}
    </div>
  );
}

/// Destaca as ocorrências de `query` dentro de `text` (sem regex — o termo
/// pode conter caracteres especiais).
function Highlight({ text, query }: { text: string; query: string }) {
  const q = query.trim();
  if (!q) return <>{text}</>;
  const parts: React.ReactNode[] = [];
  const lower = text.toLowerCase();
  const needle = q.toLowerCase();
  let from = 0;
  for (;;) {
    const at = lower.indexOf(needle, from);
    if (at === -1) break;
    if (at > from) parts.push(text.slice(from, at));
    parts.push(
      <mark key={`${at}-${parts.length}`} className="hl">
        {text.slice(at, at + needle.length)}
      </mark>,
    );
    from = at + needle.length;
  }
  parts.push(text.slice(from));
  return <>{parts}</>;
}

/// Janela-toast. Dois tipos: `meeting` (reunião começando → iniciar gravação)
/// e `recording` (lembrete horário de gravação em andamento → parar).
function MeetingAlert() {
  const params = new URLSearchParams(window.location.search);
  const kind = params.get("kind") ?? "meeting";
  const title = params.get("title") ?? "Reunião";
  const endMs = Number(params.get("end") ?? 0);
  const [busy, setBusy] = useState(false);

  async function close() {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      // destroy() fecha incondicionalmente; close() pode ficar pendurado se
      // algo no webview interceptar o fechamento — e o toast não tem outra
      // forma de ser fechado (sem decoração/taskbar).
      await getCurrentWindow().destroy();
    } catch (e) {
      logClient("agenda", `toast: falha ao fechar: ${String(e)}`);
    }
  }

  // Auto-dispensa após 60s: nenhum dos alertas deve ficar preso na tela.
  useEffect(() => {
    const t = window.setTimeout(close, 60_000);
    return () => window.clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function record() {
    setBusy(true);
    try {
      await invoke("start_meeting_recording", { endMs, title });
      await close();
    } catch {
      setBusy(false);
    }
  }

  async function stop() {
    setBusy(true);
    try {
      await invoke("stop_recording");
      await close();
    } catch (e) {
      logClient("gravacao", `toast: falha ao parar: ${String(e)}`);
      setBusy(false);
    }
  }

  // `end` carrega as horas gravadas quando kind = recording.
  if (kind === "recording") {
    const hours = endMs;
    return (
      <div className="meeting-toast">
        <div className="meeting-toast-body">
          <img src="/icon.png" alt="" width={32} height={32} />
          <div>
            <strong>Gravação em andamento</strong>
            <p>
              Gravando há {hours}h. Esqueceu de parar?
            </p>
          </div>
        </div>
        <div className="meeting-toast-actions">
          <button onClick={stop} disabled={busy}>
            {busy ? "Parando..." : "Parar gravação"}
          </button>
          <button className="secondary" onClick={close}>
            Continuar gravando
          </button>
        </div>
      </div>
    );
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
    <section className="panel home-layout">
      <RecordBar onFinished={onFinished} />
      <div className="home-body">
        <div className="home-main">
          <AgendaList hasIcs={hasIcs} recordAll={recordAll} autoSync={autoSync} />
        </div>
        <LiveNotes />
      </div>
    </section>
  );
}

/// Painel lateral de anotações manuais, visível só durante a gravação. As notas
/// são salvas automaticamente (debounce) e vinculadas à gravação em andamento.
function LiveNotes() {
  const [recording, setRecording] = useState(false);
  const [recId, setRecId] = useState<string | null>(null);
  const [notes, setNotes] = useState("");
  const [status, setStatus] = useState<"idle" | "saving" | "saved">("idle");
  const saveTimer = useRef<number | null>(null);

  async function loadFor(id: string | null) {
    setRecId(id);
    setStatus("idle");
    if (id) {
      const n = await invoke<string | null>("get_notes", { recordingId: id }).catch(() => "");
      setNotes(n ?? "");
    } else {
      setNotes("");
    }
  }

  useEffect(() => {
    invoke<boolean>("is_recording")
      .then(async (r) => {
        setRecording(r);
        if (r) {
          const id = await invoke<string | null>("current_recording_id").catch(() => null);
          await loadFor(id);
        }
      })
      .catch(() => {});
    const un = listen<boolean>("recording-changed", async (e) => {
      setRecording(e.payload);
      if (e.payload) {
        const id = await invoke<string | null>("current_recording_id").catch(() => null);
        await loadFor(id);
      }
      // Ao parar não limpamos um save pendente: ele ainda dispara com o id certo.
    });
    return () => {
      un.then((f) => f());
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function onChange(v: string) {
    setNotes(v);
    const id = recId;
    if (!id) return;
    setStatus("saving");
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(async () => {
      try {
        await invoke("save_notes", { recordingId: id, notes: v });
        setStatus("saved");
      } catch (e) {
        logClient("anotacoes", e);
        setStatus("idle");
      }
    }, 600);
  }

  if (!recording) return null;

  return (
    <aside className="live-notes">
      <div className="live-notes-head">
        <h3>Anotações da reunião</h3>
        <span className="notes-status">
          {status === "saving" ? "Salvando..." : status === "saved" ? "Salvo" : ""}
        </span>
      </div>
      <textarea
        className="live-notes-area"
        value={notes}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Escreva suas anotações durante a reunião. São salvas automaticamente e usadas para enriquecer o resumo depois."
      />
    </aside>
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
  baseSummaryPrompt,
  prompts,
  onChanged,
}: {
  recordings: Recording[];
  defaultLanguage: string;
  hasApiKey: boolean;
  hasSummaryKey: boolean;
  hasAttioKey: boolean;
  attioUserEmail: string;
  baseSummaryPrompt: string;
  prompts: SummaryPrompt[];
  onChanged: () => void;
}) {
  const [selectedId, setSelectedId] = useState("");
  const [language, setLanguage] = useState(defaultLanguage);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [summary, setSummary] = useState("");
  const [summarySaved, setSummarySaved] = useState(""); // último resumo persistido
  const [savingSummary, setSavingSummary] = useState(false);
  const [sumBusy, setSumBusy] = useState(false);
  const [sumError, setSumError] = useState<string | null>(null);
  const [sumCopied, setSumCopied] = useState(false);
  const [textQuery, setTextQuery] = useState("");
  const [sumQuery, setSumQuery] = useState("");
  const [sumHits, setSumHits] = useState<number | null>(null);
  const summaryRef = useRef<HTMLTextAreaElement | null>(null);
  const sumFrom = useRef(0);
  const [editingPrompt, setEditingPrompt] = useState(false);
  const [promptOverride, setPromptOverride] = useState("");
  const [selectedPromptId, setSelectedPromptId] = useState(""); // "" = prompt base
  const [notes, setNotes] = useState("");
  const [notesSaved, setNotesSaved] = useState(""); // últimas anotações persistidas
  const [notesSaving, setNotesSaving] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [playSrc, setPlaySrc] = useState<string | null>(null);
  const [preparing, setPreparing] = useState(false);
  const [exportFmt, setExportFmt] = useState("mp3");
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);
  const [actionMsg, setActionMsg] = useState<string | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [renameValue, setRenameValue] = useState("");

  const selected = recordings.find((r) => r.id === selectedId) ?? null;

  useEffect(() => {
    if (!selectedId && recordings.length > 0) setSelectedId(recordings[0].id);
  }, [recordings, selectedId]);

  useEffect(() => {
    setPlaying(false);
    setPlaySrc(null);
    setActionMsg(null);
    setRenaming(false);
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

  function startRename() {
    if (!selected) return;
    setRenameValue(selected.title);
    setRenaming(true);
  }

  async function confirmRename() {
    if (!selected) return;
    const t = renameValue.trim();
    if (!t) {
      setRenaming(false);
      return;
    }
    setError(null);
    try {
      await invoke("rename_recording", { recordingId: selected.id, title: t });
      setRenaming(false);
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

  async function uploadAudio() {
    setError(null);
    const path = await open({
      multiple: false,
      filters: [
        {
          name: "Áudio",
          extensions: ["mp3", "wav", "m4a", "ogg", "opus", "webm", "flac", "aac", "mp4", "mpga"],
        },
      ],
    });
    if (!path || typeof path !== "string") return;
    setImporting(true);
    try {
      const row = await invoke<Recording>("import_audio", { srcPath: path });
      onChanged();
      setSelectedId(row.id);
    } catch (e) {
      setError(String(e));
    } finally {
      setImporting(false);
    }
  }

  useEffect(() => {
    setText("");
    setSummary("");
    setSummarySaved("");
    setNotes("");
    setNotesSaved("");
    setTextQuery("");
    setSumQuery("");
    setError(null);
    setSumError(null);
    setEditingPrompt(false);
    setPromptOverride("");
    setSelectedPromptId("");
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
        if (s) {
          setSummary(s.text);
          setSummarySaved(s.text);
        }
      })
      .catch(() => {});
    invoke<string | null>("get_notes", { recordingId: selectedId })
      .then((n) => {
        setNotes(n ?? "");
        setNotesSaved(n ?? "");
      })
      .catch(() => {});
  }, [selectedId]);

  async function saveNotes() {
    if (!selected) return;
    setNotesSaving(true);
    setError(null);
    try {
      await invoke("save_notes", { recordingId: selected.id, notes });
      setNotesSaved(notes);
    } catch (e) {
      setError(String(e));
    } finally {
      setNotesSaving(false);
    }
  }

  // Busca no resumo: seleciona a próxima ocorrência dentro do textarea (o
  // navegador rola até a seleção). Repetir avança; ao chegar no fim, volta ao
  // início. Não dá para destacar dentro de um textarea, então selecionamos.
  function findInSummary() {
    const q = sumQuery.trim().toLowerCase();
    const el = summaryRef.current;
    if (!q || !el) return;
    const hay = summary.toLowerCase();
    setSumHits(hay.split(q).length - 1);
    let at = hay.indexOf(q, sumFrom.current);
    if (at === -1) at = hay.indexOf(q); // volta ao início
    if (at === -1) return;
    sumFrom.current = at + q.length;
    el.focus();
    el.setSelectionRange(at, at + q.length);
  }

  // Termo novo (ou resumo trocado) reinicia a varredura.
  useEffect(() => {
    sumFrom.current = 0;
    setSumHits(null);
  }, [sumQuery, selectedId]);

  async function saveSummaryEdits() {
    if (!selectedId) return;
    setSavingSummary(true);
    setSumError(null);
    try {
      await invoke("set_summary", { recordingId: selectedId, text: summary });
      setSummarySaved(summary);
    } catch (e) {
      setSumError(String(e));
    } finally {
      setSavingSummary(false);
    }
  }

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
      // A caixa de texto é a fonte: prompt escolhido/editado sobrescreve só este
      // resumo; vazio usa o prompt base das Configurações.
      const prompt = promptOverride.trim() ? promptOverride : null;
      const s = await invoke<Summary>("generate_summary", { recordingId: selectedId, prompt });
      setSummary(s.text);
      setSummarySaved(s.text);
    } catch (e) {
      setSumError(String(e));
    } finally {
      setSumBusy(false);
    }
  }

  // Escolhe um prompt da base: carrega o texto na caixa (editável só p/ este
  // resumo, sem alterar o registro da base). "" = usar o prompt base.
  function pickPrompt(id: string) {
    setSelectedPromptId(id);
    setEditingPrompt(true);
    if (id === "") {
      setPromptOverride(baseSummaryPrompt);
    } else {
      const p = prompts.find((x) => x.id === id);
      if (p) setPromptOverride(p.text);
    }
  }

  function togglePromptEditor() {
    setEditingPrompt((on) => {
      const next = !on;
      if (next && !promptOverride) setPromptOverride(baseSummaryPrompt);
      return next;
    });
  }

  async function copySummary() {
    await navigator.clipboard.writeText(summary);
    setSumCopied(true);
    window.setTimeout(() => setSumCopied(false), 1500);
  }

  return (
    <section className="panel">
      <div className="agenda-head">
        <h2>Gravações</h2>
        <button className="icon-btn" onClick={uploadAudio} disabled={importing}>
          {icon("export")}
          {importing ? "Enviando..." : "Enviar áudio"}
        </button>
      </div>
      {recordings.length === 0 ? (
        <div className="empty">
          {icon("gravacoes")}
          <p>Nenhuma gravação ainda. Grave pelo botão na Home ou envie um áudio.</p>
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
              {renaming ? (
                <span className="rename-inline">
                  <input
                    autoFocus
                    value={renameValue}
                    onChange={(e) => setRenameValue(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") confirmRename();
                      if (e.key === "Escape") setRenaming(false);
                    }}
                  />
                  <button className="icon-btn" onClick={confirmRename}>
                    Salvar
                  </button>
                  <button className="icon-btn" onClick={() => setRenaming(false)}>
                    Cancelar
                  </button>
                </span>
              ) : (
                <button className="icon-btn" onClick={startRename}>
                  {icon("rename")}
                  Renomear
                </button>
              )}
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

          {selected && (
            <div className="summary-block">
              <h3>Anotações manuais</h3>
              <p className="hint">
                Escritas durante a reunião. Entram no resumo para dar mais clareza. Você pode editar
                aqui.
              </p>
              <textarea
                className="transcript"
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
                placeholder="Sem anotações para esta gravação."
              />
              <div className="actions">
                <button onClick={saveNotes} disabled={notes === notesSaved || notesSaving}>
                  {notesSaving ? "Salvando..." : "Salvar alterações"}
                </button>
              </div>
            </div>
          )}

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
          {text && (
            <>
              <div className="search-row">
                <input
                  type="search"
                  value={textQuery}
                  onChange={(e) => setTextQuery(e.target.value)}
                  placeholder="Buscar na transcrição..."
                />
                {textQuery && (
                  <button className="secondary" onClick={() => setTextQuery("")}>
                    Limpar
                  </button>
                )}
              </div>
              <ChatTranscript text={text} query={textQuery} />
            </>
          )}

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
                <select
                  value={selectedPromptId}
                  onChange={(e) => pickPrompt(e.target.value)}
                  title="Prompt usado neste resumo"
                >
                  <option value="">Prompt base (Configurações)</option>
                  {prompts.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
                {summary && (
                  <button className="secondary" onClick={copySummary}>
                    {sumCopied ? "Copiado!" : "Copiar resumo"}
                  </button>
                )}
                <button className="secondary" onClick={togglePromptEditor}>
                  {editingPrompt ? "Ocultar prompt" : "Editar prompt deste resumo"}
                </button>
              </div>
              {editingPrompt && (
                <div className="prompt-editor">
                  <p className="hint">
                    Editar aqui afeta só este resumo — não altera o prompt salvo na base. Clique em
                    "Refazer resumo" para aplicar. Gerencie a base na aba "Prompts de resumo".
                  </p>
                  <textarea
                    className="transcript"
                    value={promptOverride}
                    onChange={(e) => setPromptOverride(e.target.value)}
                    placeholder="Instruções para o modelo gerar o resumo..."
                  />
                  <div className="actions">
                    <button
                      className="secondary"
                      onClick={() => {
                        const src =
                          selectedPromptId === ""
                            ? baseSummaryPrompt
                            : prompts.find((p) => p.id === selectedPromptId)?.text ?? baseSummaryPrompt;
                        setPromptOverride(src);
                      }}
                    >
                      Restaurar prompt selecionado
                    </button>
                  </div>
                </div>
              )}
              {sumError && <p className="error">{sumError}</p>}
              {summary && (
                <>
                  <div className="search-row">
                    <input
                      type="search"
                      value={sumQuery}
                      onChange={(e) => setSumQuery(e.target.value)}
                      onKeyDown={(e) => e.key === "Enter" && findInSummary()}
                      placeholder="Buscar no resumo..."
                    />
                    <button className="secondary" onClick={findInSummary} disabled={!sumQuery.trim()}>
                      Buscar
                    </button>
                    {sumHits !== null && (
                      <span className="hint">
                        {sumHits === 0 ? "Nada encontrado" : `${sumHits} ocorrência(s)`}
                      </span>
                    )}
                  </div>
                  <textarea
                    ref={summaryRef}
                    className="transcript"
                    value={summary}
                    onChange={(e) => setSummary(e.target.value)}
                  />
                  <div className="actions">
                    <button
                      onClick={saveSummaryEdits}
                      disabled={summary === summarySaved || savingSummary}
                    >
                      {savingSummary ? "Salvando..." : "Salvar alterações"}
                    </button>
                  </div>
                </>
              )}
            </div>
          )}

          {(text || notes.trim()) && (
            <AttioUpload
              recording={recordings.find((r) => r.id === selectedId) ?? null}
              hasTranscript={!!text}
              hasSummary={!!summary}
              hasNotes={!!notes.trim()}
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
  hasTranscript,
  hasSummary,
  hasNotes,
  hasAttioKey,
  userEmail,
}: {
  recording: Recording | null;
  hasTranscript: boolean;
  hasSummary: boolean;
  hasNotes: boolean;
  hasAttioKey: boolean;
  userEmail: string;
}) {
  const [kind, setKind] = useState<"transcript" | "summary" | "notes" | null>(null);
  const [title, setTitle] = useState("");
  const [candidates, setCandidates] = useState<AttioMeeting[] | null>(null);
  const [selected, setSelected] = useState<string | null>(null); // meeting_id or "new"
  const [checkedEmails, setCheckedEmails] = useState<Set<string>>(new Set());
  const [manualEmails, setManualEmails] = useState("");
  const [manualCompanies, setManualCompanies] = useState("");
  const [companies, setCompanies] = useState<AttioCompany[]>([]);
  const [checkedCompanies, setCheckedCompanies] = useState<Set<string>>(new Set());
  const [loadingCompanies, setLoadingCompanies] = useState(false);
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

  // Empresas digitadas por nome (o backend resolve o record_id no Attio).
  // Só vírgula/; separam — nome de empresa costuma ter espaço.
  function parseCompanyNames(): string[] {
    return manualCompanies
      .split(/[,;\n]+/)
      .map((c) => c.trim())
      .filter(Boolean);
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

  function toggleCompany(id: string) {
    setCheckedCompanies((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  // Ao escolher uma reunião real, busca as empresas dos participantes (todas
  // sugeridas já marcadas). "Nova reunião" ou sem participantes: sem empresas.
  useEffect(() => {
    const m = candidates?.find((x) => x.meeting_id === selected) ?? null;
    if (!m || m.participants.length === 0) {
      setCompanies([]);
      setCheckedCompanies(new Set());
      return;
    }
    let cancelled = false;
    setLoadingCompanies(true);
    invoke<AttioCompany[]>("attio_meeting_companies", { emails: m.participants })
      .then((cs) => {
        if (cancelled) return;
        setCompanies(cs);
        setCheckedCompanies(new Set(cs.map((c) => c.record_id)));
      })
      .catch(() => {
        if (!cancelled) {
          setCompanies([]);
          setCheckedCompanies(new Set());
        }
      })
      .finally(() => {
        if (!cancelled) setLoadingCompanies(false);
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected, candidates]);

  async function start(k: "transcript" | "summary" | "notes") {
    setKind(k);
    setCandidates(null);
    setSelected(null);
    setCheckedEmails(new Set());
    setCompanies([]);
    setCheckedCompanies(new Set());
    setManualEmails("");
    setManualCompanies("");
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
    const companyIds = Array.from(checkedCompanies);
    const companyNames = parseCompanyNames();
    if (list.length === 0 && companyIds.length === 0 && companyNames.length === 0) {
      setError("Marque ou informe ao menos 1 pessoa ou empresa para receber a nota.");
      return;
    }
    setError(null);
    setResult(null);
    setBusy(true);
    try {
      const start = new Date(recording.created_at);
      const end = new Date(recording.created_at + recording.duration_s * 1000);
      const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
      const r = await invoke<{
        meeting_id: string;
        notes_created: number;
        missing_people: string[];
        missing_companies: string[];
      }>("attio_upload", {
        recordingId: recording.id,
        kind,
        meetingId: selected === "new" ? null : selected,
        title: title || `Reunião ${formatDate(recording.created_at)}`,
        startIso: start.toISOString(),
        endIso: end.toISOString(),
        timezone: tz,
        emails: list,
        companyIds,
        companyNames,
      });
      let msg = `${r.notes_created} nota(s) criada(s) na meeting ${r.meeting_id}.`;
      if (r.missing_people.length > 0) msg += ` Sem pessoa no Attio: ${r.missing_people.join(", ")}.`;
      if (r.missing_companies.length > 0)
        msg += ` Sem empresa no Attio: ${r.missing_companies.join(", ")}.`;
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
          disabled={!hasAttioKey || !hasTranscript || busy}
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
        <button
          className={kind === "notes" ? "" : "secondary"}
          onClick={() => start("notes")}
          disabled={!hasAttioKey || !hasNotes || busy}
        >
          Subir anotações manuais
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
          {loadingCompanies && <p className="hint">Buscando empresas dos participantes...</p>}
          {companies.length > 0 && (
            <div className="attio-people">
              <span className="hint">Empresas:</span>
              {companies.map((c) => (
                <label key={c.record_id} className="chk">
                  <input
                    type="checkbox"
                    checked={checkedCompanies.has(c.record_id)}
                    onChange={() => toggleCompany(c.record_id)}
                  />
                  <span>{c.name}</span>
                </label>
              ))}
            </div>
          )}
          <div className="form-row">
            <label>Outras empresas (opcional, pelo nome, separadas por vírgula)</label>
            <input
              value={manualCompanies}
              onChange={(e) => setManualCompanies(e.target.value)}
              placeholder="Honey Island Capital, Upstcap"
            />
          </div>
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

/// Aba de gerência da base de prompts de resumo (criar/renomear/editar/apagar).
/// Os prompts ficam no banco (app-data) e sobrevivem a atualizações.
function PromptsScreen({
  prompts,
  onChanged,
}: {
  prompts: SummaryPrompt[];
  onChanged: () => void;
}) {
  const NEW = "__new__";
  const [selectedId, setSelectedId] = useState(NEW);
  const [name, setName] = useState("");
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Carrega o prompt escolhido no formulário; NEW limpa para criar um novo.
  function select(id: string) {
    setSelectedId(id);
    setMsg(null);
    setError(null);
    if (id === NEW) {
      setName("");
      setText("");
    } else {
      const p = prompts.find((x) => x.id === id);
      setName(p?.name ?? "");
      setText(p?.text ?? "");
    }
  }

  async function save() {
    setError(null);
    setMsg(null);
    if (!name.trim()) {
      setError("Dê um nome ao prompt.");
      return;
    }
    setBusy(true);
    try {
      const id = await invoke<string>("save_summary_prompt", {
        id: selectedId === NEW ? null : selectedId,
        name,
        text,
      });
      setSelectedId(id);
      setMsg("Prompt salvo.");
      onChanged();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    if (selectedId === NEW) return;
    if (!window.confirm(`Apagar o prompt "${name}"?`)) return;
    setBusy(true);
    setError(null);
    try {
      await invoke("delete_summary_prompt", { id: selectedId });
      select(NEW);
      onChanged();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const dirty =
    selectedId === NEW
      ? !!name.trim() || !!text.trim()
      : (() => {
          const p = prompts.find((x) => x.id === selectedId);
          return !p || p.name !== name || p.text !== text;
        })();

  return (
    <section className="panel">
      <h2>Prompts de resumo</h2>
      <p className="hint">
        Crie prompts nomeados para gerar resumos com estilos diferentes. Escolha um deles ao gerar
        o resumo de uma reunião (aba Gravações). A base fica salva e sobrevive a atualizações.
      </p>

      <div className="form-row">
        <label>Prompt</label>
        <select value={selectedId} onChange={(e) => select(e.target.value)}>
          <option value={NEW}>➕ Novo prompt</option>
          {prompts.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </div>

      <div className="form-row">
        <label>Nome</label>
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Ex.: Ata formal, Bullet points, Follow-up comercial"
        />
      </div>

      <div className="form-row" style={{ maxWidth: 760 }}>
        <label>Instruções (prompt)</label>
        <textarea
          className="transcript"
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Instruções para o modelo gerar o resumo..."
        />
      </div>

      <div className="actions">
        <button onClick={save} disabled={busy || !dirty}>
          {busy ? "Salvando..." : selectedId === NEW ? "Criar prompt" : "Salvar alterações"}
        </button>
        {selectedId !== NEW && (
          <button className="secondary" onClick={remove} disabled={busy}>
            Apagar
          </button>
        )}
      </div>

      {msg && <p className="ok">{msg}</p>}
      {error && <p className="error">{error}</p>}
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
  const [sttProvider, setSttProvider] = useState("groq");
  const [endpointUrl, setEndpointUrl] = useState("");
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [summaryProvider, setSummaryProvider] = useState("minimax_sub");
  const [summaryEndpointUrl, setSummaryEndpointUrl] = useState("");
  const [summaryModel, setSummaryModel] = useState("");
  const [summaryKey, setSummaryKey] = useState("");
  const [summaryPrompt, setSummaryPrompt] = useState("");
  const [attioKey, setAttioKey] = useState("");
  const [attioUserEmail, setAttioUserEmail] = useState("");
  const [testResult, setTestResult] = useState<Record<string, string>>({});
  const [logText, setLogText] = useState<string | null>(null);
  const [icsUrl, setIcsUrl] = useState("");
  const [recordAll, setRecordAll] = useState(false);
  const [autoSyncAgenda, setAutoSyncAgenda] = useState(true);
  const [autoStopMinutes, setAutoStopMinutes] = useState(120);
  // Há chave guardada para o provedor/modelo selecionados na tela?
  const [sttKeySaved, setSttKeySaved] = useState(false);
  const [summaryKeySaved, setSummaryKeySaved] = useState(false);
  const [autostart, setAutostart] = useState(true);
  const [theme, setTheme] = useState("system");
  const [appVersion, setAppVersion] = useState("");
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);
  const [updBusy, setUpdBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => {});
  }, []);

  async function checkUpdate() {
    setUpdBusy(true);
    setUpdateStatus("Verificando...");
    setUpdateVersion(null);
    try {
      const u = await check();
      if (u) {
        setUpdateVersion(u.version);
        setUpdateStatus(`Nova versão ${u.version} disponível.`);
      } else {
        setUpdateStatus("Você já está na versão mais recente.");
      }
    } catch (e) {
      setUpdateStatus("Não foi possível verificar agora.");
      logClient("updater", e);
    } finally {
      setUpdBusy(false);
    }
  }

  async function installUpdate() {
    setUpdBusy(true);
    setUpdateStatus("Baixando atualização...");
    try {
      const u = await check();
      if (!u) {
        setUpdateStatus("Você já está na versão mais recente.");
        setUpdateVersion(null);
        return;
      }
      await u.downloadAndInstall((e) => {
        if (e.event === "Finished") setUpdateStatus("Instalando...");
      });
      await relaunch();
    } catch (e) {
      setUpdateStatus("Falha ao atualizar. Veja os logs.");
      logClient("updater", e);
    } finally {
      setUpdBusy(false);
    }
  }

  // Existe chave guardada para o provedor/modelo selecionados AGORA na tela?
  // Refaz a consulta a cada troca — na NVIDIA a chave é por modelo, então mudar
  // o modelo muda a resposta. O valor da chave nunca vem para a UI, só o "tem".
  useEffect(() => {
    let cancelled = false;
    invoke<boolean>("has_provider_key", { kind: "stt", endpointUrl, model })
      .then((v) => !cancelled && setSttKeySaved(v))
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [endpointUrl, model]);

  useEffect(() => {
    let cancelled = false;
    invoke<boolean>("has_provider_key", {
      kind: "summary",
      endpointUrl: summaryEndpointUrl,
      model: summaryModel,
    })
      .then((v) => !cancelled && setSummaryKeySaved(v))
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [summaryEndpointUrl, summaryModel]);

  // Trocar de provedor/modelo limpa o campo digitado: o que estava ali era a
  // chave do provedor anterior.
  useEffect(() => {
    setApiKey("");
    setTestResult((t) => ({ ...t, stt: "" }));
  }, [endpointUrl, model]);

  useEffect(() => {
    setSummaryKey("");
    setTestResult((t) => ({ ...t, summary: "" }));
  }, [summaryEndpointUrl, summaryModel]);

  async function testApi(which: "stt" | "summary" | "attio") {
    setTestResult((t) => ({ ...t, [which]: "Testando..." }));
    try {
      let res: string;
      if (which === "stt") {
        res = await invoke<string>("test_transcription_api", {
          endpointUrl,
          model,
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
      setSummaryPrompt(settings.summary_prompt);
      setSummaryProvider(providerFromEndpoint(SUMMARY_PROVIDERS, settings.summary_endpoint_url));
      setIcsUrl(settings.ics_url);
      setRecordAll(settings.record_all);
      setAutoSyncAgenda(settings.auto_sync_agenda);
      setAutoStopMinutes(settings.auto_stop_minutes);
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
        summaryPrompt,
        icsUrl,
        recordAll,
        attioUserEmail,
        theme,
        autoSyncAgenda,
        autoStopMinutes,
      });
      // A chave é guardada no escopo do provedor/modelo atual (na NVIDIA, por
      // modelo), então o backend precisa saber endpoint+modelo junto da chave.
      if (apiKey.trim()) {
        await invoke("set_api_key", { endpointUrl, model, key: apiKey });
        setApiKey("");
      }
      if (summaryKey.trim()) {
        await invoke("set_summary_key", {
          endpointUrl: summaryEndpointUrl,
          model: summaryModel,
          key: summaryKey,
        });
        setSummaryKey("");
      }
      if (attioKey.trim()) {
        await invoke("set_attio_key", { key: attioKey });
        setAttioKey("");
      }
      // Reavalia os indicadores: os efeitos só disparam quando muda
      // provedor/modelo, e aqui o que mudou foi a chave guardada.
      invoke<boolean>("has_provider_key", { kind: "stt", endpointUrl, model })
        .then(setSttKeySaved)
        .catch(() => {});
      invoke<boolean>("has_provider_key", {
        kind: "summary",
        endpointUrl: summaryEndpointUrl,
        model: summaryModel,
      })
        .then(setSummaryKeySaved)
        .catch(() => {});
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
            placeholder={
              sttKeySaved
                ? "•••••• (chave salva para este provedor)"
                : "cole a chave da transcrição"
            }
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
            placeholder={
              summaryKeySaved
                ? "•••••• (chave salva para este modelo)"
                : "cole a chave deste modelo"
            }
          />
          <button type="button" className="secondary" onClick={() => testApi("summary")}>
            Testar
          </button>
        </div>
        {testResult.summary && <TestLine text={testResult.summary} />}
      </div>

      <div className="form-row" style={{ maxWidth: 760 }}>
        <label>Prompt do resumo (base)</label>
        <span className="hint">
          Instrução enviada ao modelo em todos os resumos. Você pode ajustar um resumo específico na
          aba Gravações.
        </span>
        <textarea
          className="transcript"
          style={{ minHeight: 160 }}
          value={summaryPrompt}
          onChange={(e) => setSummaryPrompt(e.target.value)}
          placeholder="Instruções para o modelo gerar o resumo..."
        />
        <div className="actions">
          <button
            type="button"
            className="secondary"
            onClick={async () => {
              try {
                setSummaryPrompt(await invoke<string>("default_summary_prompt"));
              } catch (e) {
                logClient("resumo", e);
              }
            }}
          >
            Restaurar padrão
          </button>
        </div>
      </div>

      <p className="hint">
        As chaves ficam no keychain do sistema, nunca em texto puro. Cada provedor
        guarda a sua: ao trocar de provedor, a chave anterior continua salva. Na
        NVIDIA a chave é por modelo — cada modelo precisa da sua.
      </p>

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

      <h3 className="cfg-section">Gravação</h3>
      <p className="hint">Evita gravações esquecidas ligadas por muito tempo.</p>
      <label className="chk">
        <input
          type="checkbox"
          checked={autoStopMinutes > 0}
          onChange={(e) => setAutoStopMinutes(e.target.checked ? 120 : 0)}
        />
        Parar a gravação automaticamente após
        <select
          value={autoStopMinutes > 0 ? autoStopMinutes : 120}
          disabled={autoStopMinutes === 0}
          onChange={(e) => setAutoStopMinutes(Number(e.target.value))}
        >
          {AUTO_STOP_OPTIONS.map((o) => (
            <option key={o.min} value={o.min}>
              {o.label}
            </option>
          ))}
        </select>
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

      <h3 className="cfg-section">Sobre</h3>
      <p className="hint">Versão instalada: {appVersion || "..."}</p>
      <div className="actions">
        <button type="button" className="secondary" onClick={checkUpdate} disabled={updBusy}>
          Buscar atualização
        </button>
        {updateVersion && (
          <button type="button" onClick={installUpdate} disabled={updBusy}>
            {updBusy ? "Atualizando..." : `Atualizar para ${updateVersion}`}
          </button>
        )}
      </div>
      {updateStatus && <p className="hint">{updateStatus}</p>}

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
