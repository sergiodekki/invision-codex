# Invision Codex

Fork próprio e independente de openai/codex (base: upstream em 2026-10-12). Evolui do jeito da Invision; não sincroniza com o upstream.

- Provedor nativo `invision` (codex-rs/model-provider-info): Responses API em `INVISION_BASE_URL`, chave `INVISION_API_KEY`.
- A Invision roda `codex app-server` por stdio; sessões persistem em `CODEX_HOME` por usuário.
- `invision-release` compila o binário Linux musl e publica release `invision-<data>-<sha>`; a Invision fixa a release usada no Dockerfile.
- Licença Apache-2.0 e NOTICE do upstream preservados.
