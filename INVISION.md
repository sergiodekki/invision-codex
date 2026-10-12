# Invision Codex

Fork mantido de openai/codex usado pelo agente do chat da Invision.

- Provedor nativo `invision` (codex-rs/model-provider-info): Responses API em `INVISION_BASE_URL` (padrão https://invision.dreamvision.com.br/v1), chave `INVISION_API_KEY`.
- Uso: `codex exec -c model_provider=invision -m <modelo> --json "..."`.
- `invision-sync` faz merge semanal do upstream; conflito abre issue.
- `invision-release` compila o binário Linux musl a cada mudança e publica release `invision-*`.
