# Assinatura de código — SignPath Foundation

Plano para assinar os executáveis Windows **gratuitamente** via [SignPath Foundation](https://signpath.org/), que oferece certificados de assinatura para projetos open source qualificados.

## Por que
Instaladores não assinados disparam SmartScreen e antivírus (Kaspersky etc.). Assinatura digital com timestamp resolve a maior parte disso no Windows. macOS (notarização) exige Apple Developer pago — fora do escopo por ora.

## Pré-requisitos (status)
- [x] Licença OSI (MIT — [LICENSE](../LICENSE)).
- [x] Código-fonte público: https://github.com/GeanChu/hicorder
- [x] Build 100% em CI público (GitHub Actions — [release.yml](../.github/workflows/release.yml)); os artefatos assinados devem vir do CI, nunca de build local.
- [x] Metadados do produto no binário (publisher, versão, descrição — `tauri.conf.json`).
- [x] README com descrição do projeto e política de segurança ([SECURITY.md](../SECURITY.md)).
- [x] Repo renomeado para `hicorder` (consistência com o produto).

## Como aplicar (feito pelo dono do repo)
1. Criar conta em https://app.signpath.io (login com GitHub).
2. Submeter o formulário do programa OSS: https://signpath.org/apply
   - Project name: **Hicorder**
   - Repository: URL do repo público
   - License: MIT
   - Descrição curta do que o app faz (gravador de reuniões com transcrição por IA) e do público (times).
   - Committers: conta(s) GitHub com acesso de escrita.
3. Aguardar a avaliação (dias a poucas semanas). A Foundation verifica atividade do projeto e identidade dos mantenedores.

## Depois de aprovado
A SignPath cria uma organização com um projeto e uma *signing policy* (`release-signing`). Integração no `release.yml` (job do Windows, após o build):

```yaml
      - name: Sign Windows installers (SignPath)
        if: matrix.platform == 'windows-latest'
        uses: signpath/github-action-submit-signing-request@v1
        with:
          api-token: ${{ secrets.SIGNPATH_API_TOKEN }}
          organization-id: ${{ vars.SIGNPATH_ORG_ID }}
          project-slug: hicorder
          signing-policy-slug: release-signing
          github-artifact-id: <id do artifact com .msi/.exe>
          wait-for-completion: true
          output-artifact-directory: signed
```

Requisitos da integração:
- Publicar os instaladores como **artifact** do workflow antes do passo de assinatura (a SignPath baixa do GitHub, assina no ambiente deles e devolve).
- Segredos: `SIGNPATH_API_TOKEN` (Settings → Secrets) e `SIGNPATH_ORG_ID` (Settings → Variables).
- A Foundation normalmente exige atribuição no README, ex.: *"Free code signing provided by [SignPath.io](https://signpath.io), certificate by SignPath Foundation"* — adicionar quando ativo.

## Complementos (independentes da assinatura)
- Reportar falso positivo do instalador: Kaspersky (https://opentip.kaspersky.com) e Microsoft (https://www.microsoft.com/wdsi/filesubmission).
- SmartScreen ganha reputação com downloads ao longo do tempo; a assinatura acelera muito.
