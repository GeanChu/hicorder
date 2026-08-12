//! Resumo pelo **Claude Code instalado na máquina do usuário**.
//!
//! Diferente dos outros provedores de resumo, aqui não há HTTP nem chave de
//! API: o texto vai para o CLI `claude` em modo não-interativo (`--print`) e a
//! autenticação é a assinatura Claude que o usuário já tem na máquina.
//!
//! Importante: **não é local nem offline** — o Claude Code fala com a API da
//! Anthropic. O ganho é não precisar cadastrar mais uma chave.
//!
//! Decisões de invocação (validadas contra o CLI 2.1.220):
//! - a transcrição vai por **stdin**, nunca em argumento: reunião de 1h passa
//!   fácil do limite de ~32k caracteres da linha de comando no Windows;
//! - `--output-format json` dá `is_error`/`result`, então dá para distinguir
//!   "modelo respondeu" de "falhou" sem adivinhar pelo texto;
//! - `--max-turns 1` + `--disallowed-tools` mantêm o CLI como resumidor, sem
//!   virar agente que lê arquivos da máquina;
//! - `--system-prompt` substitui o prompt de agente de código pelo prompt de
//!   resumo do Hicorder;
//! - `--exclude-dynamic-system-prompt-sections` derruba o overhead de contexto
//!   de ~27k para ~14k tokens por chamada;
//! - roda numa pasta temporária, para o CLI não enxergar (nem gravar sessão
//!   em) o diretório de trabalho do usuário.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Result};

/// Endpoint-sentinela. Não é URL de verdade: marca que o resumo sai pelo
/// Claude Code local em vez de uma chamada HTTP.
pub const ENDPOINT: &str = "claude-code://local";

/// Modelo usado no teste de instalação (o mais barato/rápido).
const TEST_MODEL: &str = "claude-haiku-4-5";

/// Teto de tempo do resumo. Reunião longa com modelo de raciocínio demora;
/// o mesmo teto dos provedores HTTP (180s) seria curto demais aqui porque o
/// CLI ainda sobe o processo e carrega o contexto antes de chamar a API.
const SUMMARY_TIMEOUT_SECS: u64 = 300;
const TEST_TIMEOUT_SECS: u64 = 90;

/// Ferramentas bloqueadas: sem isso o CLI pode tentar ler/escrever arquivos da
/// máquina em vez de só resumir o texto que recebeu.
const DISALLOWED_TOOLS: &[&str] = &[
    "Bash",
    "Read",
    "Write",
    "Edit",
    "Glob",
    "Grep",
    "WebFetch",
    "WebSearch",
    "Task",
    "NotebookEdit",
];

/// Este endpoint usa o Claude Code local?
///
/// Aceita o valor canônico e qualquer coisa no mesmo esquema — o que a UI
/// grava é `ENDPOINT`, mas um banco antigo ou edição manual não deve virar
/// chamada HTTP para uma URL que não existe.
pub fn is_endpoint(endpoint_url: &str) -> bool {
    let u = endpoint_url.trim();
    u == ENDPOINT || u.starts_with("claude-code:")
}

/// O app **Claude Desktop** instala um `claude.exe` próprio (Electron, ~232 MB)
/// em `AnthropicClaude/` — nome idêntico ao do CLI, programa completamente
/// diferente. Se essa pasta estiver no PATH, a busca acharia o Desktop e
/// tentaria rodar uma GUI com `--print`. Descarta esses caminhos.
/// Compara sobre o caminho como texto, não por componentes: `components()` só
/// reconhece `\` como separador no Windows, então um caminho do Windows
/// avaliado no macOS/Linux virava um componente único e escapava do filtro.
fn is_desktop_app(path: &Path) -> bool {
    let p = path.to_string_lossy().to_lowercase();
    // Windows: .../AppData/Local/AnthropicClaude/... · macOS: .../Claude.app/...
    p.contains("anthropicclaude") || p.contains("claude.app")
}

/// Nomes possíveis do executável, por SO.
fn binary_names() -> &'static [&'static str] {
    if cfg!(windows) {
        // `.cmd` é o shim gerado por `npm i -g @anthropic-ai/claude-code`.
        &["claude.exe", "claude.cmd", "claude.bat", "claude"]
    } else {
        &["claude"]
    }
}

/// Caminhos onde o instalador nativo e o npm costumam deixar o binário.
fn known_locations() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);
    if let Some(h) = &home {
        // Instalador nativo (curl -fsSL claude.ai/install.sh | bash).
        out.push(h.join(".local").join("bin"));
        out.push(h.join(".claude").join("local"));
    }
    if cfg!(windows) {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            out.push(PathBuf::from(appdata).join("npm")); // npm global
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            out.push(PathBuf::from(local).join("Programs").join("claude"));
        }
    } else {
        out.push(PathBuf::from("/usr/local/bin"));
        out.push(PathBuf::from("/opt/homebrew/bin"));
        if let Some(h) = &home {
            out.push(h.join(".npm-global").join("bin"));
        }
    }
    out
}

/// Acha o executável do Claude Code.
///
/// Ordem: caminho informado nas Configurações → PATH → locais conhecidos. O
/// PATH sozinho não basta: o app pode ser aberto pela bandeja/autostart, e aí
/// herda um PATH mais pobre que o do terminal onde o usuário instalou o CLI.
pub fn resolve(explicit: Option<&str>) -> Result<PathBuf> {
    if let Some(p) = explicit.map(str::trim).filter(|p| !p.is_empty()) {
        let path = PathBuf::from(p);
        if !path.is_file() {
            bail!("o caminho informado para o Claude Code não existe: {p}");
        }
        if is_desktop_app(&path) {
            bail!(
                "esse é o app Claude Desktop, não o Claude Code (CLI). \
                 São programas diferentes com o mesmo nome de executável."
            );
        }
        return Ok(path);
    }

    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(path_var) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path_var));
    }
    dirs.extend(known_locations());

    for dir in dirs {
        if is_desktop_app(&dir) {
            continue; // é o Claude Desktop, não o CLI.
        }
        for name in binary_names() {
            let cand = dir.join(name);
            if cand.is_file() {
                return Ok(cand);
            }
        }
    }
    bail!(
        "Claude Code não encontrado nesta máquina. Instale seguindo \
         docs.claude.com/en/docs/claude-code/setup e rode `claude` uma vez no terminal para \
         autenticar (ter o app Claude Desktop instalado não basta — é outro programa). \
         Se já estiver instalado, informe o caminho do executável nas Configurações."
    )
}

/// Monta o `Command`. Shims `.cmd`/`.bat` do npm não podem ser executados
/// direto pelo CreateProcess do Windows — precisam passar pelo `cmd.exe`.
fn command_for(bin: &Path) -> Command {
    let ext = bin
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mut cmd = if cfg!(windows) && (ext == "cmd" || ext == "bat") {
        let mut c = Command::new("cmd.exe");
        c.arg("/C").arg(bin);
        c
    } else {
        Command::new(bin)
    };

    // Pasta neutra: o CLI não deve enxergar o diretório de trabalho do app.
    let workdir = std::env::temp_dir().join("hicorder-claude-code");
    let _ = std::fs::create_dir_all(&workdir);
    cmd.current_dir(&workdir);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

/// Executa o CLI com `input` no stdin e devolve (stdout, stderr, sucesso).
fn run_raw(bin: &Path, args: &[String], input: &str, timeout_secs: u64) -> Result<(String, String, bool)> {
    let mut cmd = command_for(bin);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        anyhow!("falha ao executar o Claude Code ('{}'): {e}", bin.display())
    })?;

    // stdin numa thread: a transcrição pode ser maior que o buffer do pipe, e
    // escrever tudo aqui travaria enquanto o filho ainda não lê.
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("Claude Code sem stdin"))?;
    let payload = input.to_string();
    let writer = thread::spawn(move || {
        let _ = stdin.write_all(payload.as_bytes());
        // Dropar aqui fecha o pipe → o CLI recebe EOF e começa a processar.
    });

    // stdout/stderr também em threads, senão um pipe cheio trava o filho.
    let out_h = reader(child.stdout.take());
    let err_h = reader(child.stderr.take());

    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {
                if start.elapsed() > Duration::from_secs(timeout_secs) {
                    let _ = child.kill();
                    let _ = child.wait();
                    bail!("o Claude Code não respondeu em {timeout_secs}s e foi encerrado");
                }
                thread::sleep(Duration::from_millis(200));
            }
            Err(e) => bail!("falha ao aguardar o Claude Code: {e}"),
        }
    };

    let _ = writer.join();
    let stdout = out_h.map(|h| h.join().unwrap_or_default()).unwrap_or_default();
    let stderr = err_h.map(|h| h.join().unwrap_or_default()).unwrap_or_default();
    Ok((stdout, stderr, status.success()))
}

fn reader<R: Read + Send + 'static>(src: Option<R>) -> Option<thread::JoinHandle<String>> {
    src.map(|mut r| {
        thread::spawn(move || {
            let mut buf = String::new();
            let _ = r.read_to_string(&mut buf);
            buf
        })
    })
}

/// Primeira linha não vazia do stderr — o resto costuma ser ruído de stack.
fn first_line(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("sem detalhes")
        .chars()
        .take(300)
        .collect()
}

fn args_for(model: &str, system_prompt: &str) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--print".into(),
        "--max-turns".into(),
        "1".into(),
        "--output-format".into(),
        "json".into(),
        "--exclude-dynamic-system-prompt-sections".into(),
        "--system-prompt".into(),
        system_prompt.to_string(),
    ];
    args.push("--disallowed-tools".into());
    args.extend(DISALLOWED_TOOLS.iter().map(|t| t.to_string()));
    let m = model.trim();
    if !m.is_empty() {
        args.push("--model".into());
        args.push(m.to_string());
    }
    args
}

/// Extrai o texto do JSON de resposta do `--output-format json`.
fn text_from_json(stdout: &str) -> Result<String> {
    let json: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| anyhow!("resposta do Claude Code não é JSON ({e}): {}", first_line(stdout)))?;

    if json.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false) {
        let motivo = json
            .get("api_error_status")
            .and_then(|v| v.as_str())
            .or_else(|| json.get("subtype").and_then(|v| v.as_str()))
            .unwrap_or("erro não identificado");
        let detalhe = json.get("result").and_then(|v| v.as_str()).unwrap_or("");
        bail!("Claude Code retornou erro ({motivo}): {detalhe}");
    }

    let text = json
        .get("result")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if text.is_empty() {
        bail!("Claude Code respondeu sem texto: {}", first_line(stdout));
    }
    Ok(text)
}

/// Gera o resumo. `input` é o conteúdo (transcrição + anotações) já montado.
pub fn summarize(
    explicit_path: Option<&str>,
    model: &str,
    system_prompt: &str,
    input: &str,
) -> Result<String> {
    let bin = resolve(explicit_path)?;
    let args = args_for(model, system_prompt);
    let (stdout, stderr, ok) = run_raw(&bin, &args, input, SUMMARY_TIMEOUT_SECS)?;
    if !ok {
        bail!("Claude Code terminou com erro: {}", first_line(&stderr));
    }
    text_from_json(&stdout)
}

/// Versão do CLI (`claude --version`), para o teste mostrar ao usuário.
///
/// Também serve de prova de identidade: o CLI responde algo como
/// `2.1.220 (Claude Code)`. Se o que estiver ali for outro programa com o
/// mesmo nome (o Claude Desktop, por exemplo), a assinatura não bate.
fn version(bin: &Path) -> Result<String> {
    let (stdout, stderr, ok) = run_raw(bin, &["--version".to_string()], "", 30)?;
    if !ok {
        bail!("`claude --version` falhou: {}", first_line(&stderr));
    }
    let v = first_line(&stdout);
    if !v.to_lowercase().contains("claude code") {
        bail!(
            "o executável em {} não é o Claude Code (respondeu: {v}). \
             Verifique o caminho nas Configurações.",
            bin.display()
        );
    }
    Ok(v)
}

/// Testa a instalação: acha o binário, lê a versão e faz uma chamada mínima
/// de verdade. Só a versão não bastaria — o CLI pode estar instalado e **não
/// autenticado**, que é a falha mais provável na máquina de um usuário novo.
pub fn test(explicit_path: Option<&str>) -> Result<String> {
    let bin = resolve(explicit_path)?;
    let ver = version(&bin)?;

    let args = args_for(TEST_MODEL, "Responda apenas com a palavra: ok");
    let (stdout, stderr, ok) = run_raw(&bin, &args, "ping", TEST_TIMEOUT_SECS)?;
    if !ok {
        bail!(
            "Claude Code {ver} encontrado em {}, mas a chamada falhou: {}. \
             Rode `claude` no terminal uma vez para autenticar.",
            bin.display(),
            first_line(&stderr)
        );
    }
    text_from_json(&stdout).map_err(|e| {
        anyhow!(
            "Claude Code {ver} encontrado em {}, mas não respondeu: {e}. \
             Rode `claude` no terminal uma vez para autenticar.",
            bin.display()
        )
    })?;

    Ok(format!(
        "Claude Code OK — {ver}, em {}. Não precisa de chave de API.",
        bin.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconhece_o_endpoint_sentinela() {
        assert!(is_endpoint(ENDPOINT));
        assert!(is_endpoint("  claude-code://local  "));
        assert!(!is_endpoint("https://api.openai.com/v1/chat/completions"));
        assert!(!is_endpoint(""));
    }

    #[test]
    fn caminho_informado_e_inexistente_falha_claro() {
        let e = resolve(Some("Z:/nao/existe/claude.exe")).unwrap_err().to_string();
        assert!(e.contains("não existe"), "mensagem inesperada: {e}");
    }

    #[test]
    fn caminho_vazio_cai_na_busca_automatica() {
        // String vazia não pode ser tratada como caminho explícito inválido.
        let r = resolve(Some("   "));
        // Pode achar ou não (depende da máquina), mas nunca é o erro de
        // "caminho informado não existe".
        if let Err(e) = r {
            assert!(e.to_string().contains("não encontrado"), "mensagem inesperada: {e}");
        }
    }

    #[test]
    fn nao_confunde_o_claude_desktop_com_o_cli() {
        // Mesmo nome de executável, programa diferente. O teste roda nos três
        // SOs, então cobre os caminhos de todos eles — a comparação é textual
        // justamente para não depender do separador da plataforma que executa.
        for desktop in [
            r"C:\Users\x\AppData\Local\AnthropicClaude\app-1.24012.11\claude.exe",
            r"C:\Users\x\AppData\Local\anthropicclaude\claude.exe",
            "/Applications/Claude.app/Contents/MacOS/claude",
            "/Users/x/Applications/claude.app/Contents/MacOS/claude",
        ] {
            assert!(is_desktop_app(Path::new(desktop)), "não filtrou: {desktop}");
        }
        for cli in [
            r"C:\Users\x\.local\bin\claude.exe",
            r"C:\Users\x\AppData\Roaming\npm\claude.cmd",
            "/usr/local/bin/claude",
            "/Users/x/.local/bin/claude",
            "/opt/homebrew/bin/claude",
        ] {
            assert!(!is_desktop_app(Path::new(cli)), "filtrou o CLI: {cli}");
        }
    }

    #[test]
    fn args_sem_modelo_nao_mandam_flag_model() {
        let a = args_for("", "prompt");
        assert!(!a.iter().any(|x| x == "--model"));
        assert!(a.iter().any(|x| x == "--print"));
        assert!(a.iter().any(|x| x == "--system-prompt"));
    }

    #[test]
    fn args_com_modelo_mandam_flag_model() {
        let a = args_for("claude-sonnet-5", "prompt");
        let i = a.iter().position(|x| x == "--model").expect("--model ausente");
        assert_eq!(a[i + 1], "claude-sonnet-5");
    }

    #[test]
    fn args_bloqueiam_ferramentas_de_arquivo() {
        let a = args_for("", "prompt");
        assert!(a.iter().any(|x| x == "--disallowed-tools"));
        for t in ["Bash", "Read", "Write"] {
            assert!(a.iter().any(|x| x == t), "ferramenta {t} não bloqueada");
        }
    }

    #[test]
    fn extrai_texto_do_json_de_sucesso() {
        let j = r#"{"is_error":false,"result":"resumo aqui","subtype":"success"}"#;
        assert_eq!(text_from_json(j).unwrap(), "resumo aqui");
    }

    #[test]
    fn json_com_is_error_vira_erro() {
        let j = r#"{"is_error":true,"subtype":"error_max_turns","result":"","api_error_status":null}"#;
        let e = text_from_json(j).unwrap_err().to_string();
        assert!(e.contains("error_max_turns"), "mensagem inesperada: {e}");
    }

    #[test]
    fn json_sem_texto_vira_erro() {
        let j = r#"{"is_error":false,"result":"   "}"#;
        assert!(text_from_json(j).is_err());
    }

    #[test]
    fn saida_nao_json_vira_erro_legivel() {
        let e = text_from_json("command not found").unwrap_err().to_string();
        assert!(e.contains("não é JSON"), "mensagem inesperada: {e}");
    }

    /// Smoke test contra o Claude Code REAL da máquina (ignored, como o
    /// `rec_smoke`): exercita resolve + version + chamada de verdade.
    /// Consome cota da assinatura.
    ///
    ///   cargo test --lib claude_code_smoke -- --ignored --nocapture
    #[test]
    #[ignore]
    fn claude_code_smoke() {
        let bin = resolve(None).expect("Claude Code não encontrado");
        println!("binário: {}", bin.display());
        println!("teste:   {}", test(None).expect("teste falhou"));

        let resumo = summarize(
            None,
            TEST_MODEL,
            "Você resume reuniões em português do Brasil. Responda só com tópicos curtos.",
            "[00:01] Você: fechamos o term sheet na sexta.\n\
             [00:04] Participantes: mando o cap table atualizado amanhã.",
        )
        .expect("resumo falhou");
        println!("resumo:\n{resumo}");
        assert!(!resumo.trim().is_empty());
    }
}
