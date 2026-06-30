import { useState } from "react";
import "./App.css";

type Tab = "gravar" | "gravacoes" | "transcricao" | "config";

const TABS: { id: Tab; label: string }[] = [
  { id: "gravar", label: "Gravar" },
  { id: "gravacoes", label: "Gravações" },
  { id: "transcricao", label: "Transcrição" },
  { id: "config", label: "Configurações" },
];

function App() {
  const [tab, setTab] = useState<Tab>("gravar");

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
          <Placeholder title="Gravar" hint="Botão de gravação e medidor de nível chegam no PR4." />
        )}
        {tab === "gravacoes" && (
          <Placeholder title="Gravações" hint="Lista de gravações com data, duração e ações (PR4)." />
        )}
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

function Placeholder({ title, hint }: { title: string; hint: string }) {
  return (
    <section className="panel">
      <h2>{title}</h2>
      <p className="hint">{hint}</p>
    </section>
  );
}

export default App;
