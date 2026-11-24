# Invision Codex

Fork de openai/codex (base atual: upstream em 2026-11-21). Sincroniza com o upstream por ramo revisado (`invision/upstream-<data>`): merge do upstream, testes do provedor e do app-server no CI, pré-release sem `--latest` e teste de contrato na Invision antes de entrar na `main`. A Invision fixa a versão usada pela tag (`CODEX_RELEASE` no Dockerfile dela); trocar a tag é o caminho de ida e de volta.

- Provedor nativo `invision` (codex-rs/model-provider-info): Responses API em `INVISION_BASE_URL`, chave `INVISION_API_KEY`.
- A Invision roda `codex app-server` por stdio; sessões persistem em `CODEX_HOME` por usuário.
- `invision-release` compila o binário Linux glibc (o `codex-code-mode-host` embute V8) e publica a release `invision-<data>-<sha>`: release normal na `main`, pré-release nos outros ramos.
- Licença Apache-2.0 e NOTICE do upstream preservados.
