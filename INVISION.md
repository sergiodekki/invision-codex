# Invision Codex

Fork de openai/codex (base atual: upstream em 2026-11-21). Sincroniza com o upstream por ramo revisado (`invision/upstream-<data>`): merge do upstream, testes do provedor e do app-server no CI, pré-release sem `--latest` e teste de contrato na Invision antes de entrar na `main`. A Invision fixa a versão usada pela tag (`CODEX_RELEASE` no Dockerfile dela); trocar a tag é o caminho de ida e de volta.

- Provedor nativo `invision` (codex-rs/model-provider-info): Responses API em `INVISION_BASE_URL`, chave `INVISION_API_KEY`.
- A Invision roda `codex app-server` por stdio; sessões persistem em `CODEX_HOME` por usuário.
- `invision-release` compila o binário Linux glibc (o `codex-code-mode-host` embute V8) e publica a release `invision-<data>-<sha>`: release normal na `main`, pré-release nos outros ramos.
- Licença Apache-2.0 e NOTICE do upstream preservados.

## Mudanças da Invision sobre o upstream (manter em toda sincronização)

- **Pastas protegidas na área gravável:** `.claude`, `.cursor`, `.gemini`, `.vscode` e `.invision` ficam somente leitura dentro do sandbox, como `.codex`, e protegidas mesmo antes de existirem. Aparecem em `PROTECTED_METADATA_PATH_NAMES` e `INVISION_HOST_EXECUTED_CONFIG_PATH_NAMES` (`codex-rs/protocol/src/permissions.rs`). **Por quê:** são configurações que outra ferramenta executa no computador depois (hooks, servidores MCP, tarefas), e um hook plantado em `.claude/settings.json` rodou fora do sandbox (CVE-2026-25725).
- **Proxy de rede:** `Host::parse` (`codex-rs/network-proxy/src/policy.rs`) recusa caractere de controle (inclusive NUL), espaço e `/ \ @ ? #`. **Por quê:** `evil.example\0.permitido.example` casava com `*.permitido.example` enquanto o resolvedor discava `evil.example` (desvio por SOCKS5 relatado em 2026).
- **Máscara de saídas antigas:** o crate `codex-invision-prompt-mask` entra atrás do recurso `features.mask_old_tool_outputs`, desligado por padrão. As saídas de ferramenta antigas e grandes vão ao modelo como um aviso curto, em degraus, para o começo do prompt não mudar entre degraus e o cache continuar valendo. O histórico guardado não muda. **Por quê:** o estudo "The Complexity Trap" (NeurIPS 2025) mediu metade do custo com o mesmo acerto. A simulação de custo está em `invision-prompt-mask/examples/simulate.rs`.
