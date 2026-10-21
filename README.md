# Player Scout

> Sistema de scouting, estatísticas avançadas e análise tática para esportes de raquete (foco exclusivo inicial: **Padel**).

O **Player Scout** permite que treinadores e atletas registrem cada jogada em tempo real diretamente na quadra (via tablet ou celular) e analisem a evolução técnica e maturidade tática do atleta por meio de um dashboard analítico recursivo e comparativo.

---

## 🎯 Principais Funcionalidades

- **Scouting Ágil em Quadra (Touch-First):**
  - Ergonomia 100% pensada para celular e tablet, sem necessidade de teclas de atalho.
  - Botões de ação rápida na *Thumb Zone* (base inferior da tela) para marcar **ACERTOU** ou **ERROU** com um toque.
  - Registro de qualidade de decisão: padrão **Decisão Boa** com toggle opcional para **Decisão Ruim**.
  - Marcadores especiais: **Winner** (ponto vencedor direto) e **Erro Não Forçado (ENF)**.
  - Botão imediato de desfazer (*Undo*) para correções instantâneas durante o jogo.
- **Catálogo Completo do Padel (21 Golpes):**
  - *Início de Ponto:* Saque e Recepção.
  - *Rede & Ataque:* Voleio Direita, Voleio Esquerda, Bandeja, Víbora, Smash Trazer, Smash x3, Smash x4, Rulo na Grade, Deixadinha (Drop shot), Bloqueio.
  - *Fundo & Vidros:* Batida Direita, Batida Esquerda, Globo Direita, Globo Esquerda, Chiquita, Saída Parede Direita, Saída Parede Esquerda, Bajada de Parede, Contra-parede.
- **Matriz Tática de Ouro (Decisão × Execução):**
  - *Acertou + Boa Decisão:* Jogada ideal.
  - *Acertou + Decisão Ruim:* Compensação por sorte/falha rival.
  - *Errou + Boa Decisão:* Erro puramente técnico de execução (ideia tática correta).
  - *Errou + Decisão Ruim:* Erro tático grave.
- **Design Responsivo & Recursivo:**
  - As métricas e gráficos operam com consistência idêntica em qualquer nível de granularidade (*drill-down*):
    $$\text{Jogador} \longrightarrow \text{Temporada/Ano} \longrightarrow \text{Torneio/Mês} \longrightarrow \text{Partida} \longrightarrow \text{Set} \longrightarrow \text{Jogada}$$
- **Modo Comparativo Lado a Lado (A vs B):**
  - Compare a evolução entre dois sets de uma mesma partida (ex: Set 1 vs Set 2), duas partidas ou dois ciclos de treino, com cálculo automático de deltas (▲/▼) e barras emparelhadas de aproveitamento.
- **Visualizações com Chart.js:**
  - Radar de habilidades (6 fases do Padel), evolução temporal cronológica e gráficos de volume x eficácia.

---

## 🛠️ Stack Tecnológica

O projeto segue estritamente as diretrizes de arquitetura, segurança e convenções definidas em [AGENTS.md](file:///home/jc/codigos/rust/player-scout/AGENTS.md):

- **Backend:**
  - Linguagem: **Rust Edition 2024** (toolchain `1.99.x` ou superior).
  - Framework HTTP: [axum](https://github.com/tokio-rs/axum) (roteamento assíncrono e tratamento de requisições).
  - ORM / Acesso a Dados: [Toasty](https://github.com/tokio-rs/toasty) (ORM assíncrono do ecossistema Tokio para PostgreSQL).
  - Templates HTML Server-Side: [Askama](https://github.com/rinja-rs/askama) (templates compilados e type-safe).
- **Banco de Dados:**
  - **PostgreSQL 18** ou superior, com constraints relacionais garantindo integridade das regras esportivas.
- **Frontend Web:**
  - **HTMX 4.x** + **htmx-ui** para hipertexto reativo com mínimo JavaScript.
  - **Chart.js** (versão estritamente auditada contra vulnerabilidades conhecidas) para gráficos e dashboards.
- **Clientes Móveis (Fase Futura):**
  - Kotlin para Android 13+ e avaliação de Kotlin Multiplatform (KMP) para iOS.
- **Qualidade, Segurança e Testes:**
  - **100% dos testes automatizados escritos em Rust**, incluindo validações de regras de negócio, integrações com o banco e contratos da web.
  - Análise estática com `cargo clippy -- -D warnings`.
  - Auditoria contínua de dependências com `cargo-audit`.

---

## 🚀 Como Executar o Ambiente Local

### 1. Configurar o arquivo `.env`
Copie o modelo de exemplo e defina suas credenciais locais:
```bash
cp .env.example .env
```

### 2. Iniciar o Banco de Dados e MCP via Docker Compose
Suba os containers do PostgreSQL 18 e do MCP Server em segundo plano:
```bash
docker compose up -d
```

Para verificar o status e o healthcheck:
```bash
docker compose ps
```

### 3. Conexão do MCP no VSCode
Adicione a configuração no seu cliente MCP / VSCode para inspecionar o banco diretamente pelo container:
```json
{
  "mcpServers": {
    "postgres": {
      "command": "docker",
      "args": [
        "exec",
        "-i",
        "player_scout_postgres_mcp",
        "npx",
        "-y",
        "@modelcontextprotocol/server-postgres",
        "postgresql://postgres:sua_senha@postgres:5432/playerscout_dev"
      ]
    }
  }
}
```

---

## 📁 Estrutura do Repositório

```text
player-scout/
├── AGENTS.md            # Instruções e diretrizes centrais dos especialistas
├── ROADMAP.md           # Roteiro de fases, status atual e acompanhamento
├── CHANGELOG.md         # Histórico detalhado de mudanças e commits
├── README.md            # Esta documentação geral
├── docker-compose.yml   # PostgreSQL 18 e serviço MCP containerizado
├── docs/                # Manuais técnicos e guias de arquitetura
│   ├── ONBOARDING_JUNIOR.md        # Guia passo a passo de onboarding para novos desenvolvedores
│   ├── MODULARIZACAO_WEB.md        # Documentação da arquitetura modular da camada Web (api::web)
│   └── MIGRATIONS_E_TOASTY_ORM.md  # Manual de migrações SQL e persistência com Toasty ORM
├── migrations/          # Migrações relacionais do PostgreSQL
├── http/                # Scripts de teste interativo da API REST para o plugin REST Client
└── rust/                # Backend em Rust (Axum, Toasty ORM, Domínio e Testes)
```

## 📚 Documentação Técnica e Guias

- 🐣 **[Guia de Onboarding para Desenvolvedores Juniores](docs/ONBOARDING_JUNIOR.md):** Ponto de partida ideal! Explica o glossário do Padel (21 golpes e matriz tática), setup do ambiente local, ciclo de vida das requisições HTMX e a "receita de bolo" em 5 passos para novas features.
- 🌐 **[Arquitetura da Camada Web](docs/MODULARIZACAO_WEB.md):** Detalhamento dos 6 submódulos da interface (`auth`, `dashboard`, `court`, `matches`, `analytics`, `preferences`).
- 💾 **[Migrações SQL e Toasty ORM](docs/MIGRATIONS_E_TOASTY_ORM.md):** Manual completo de modelagem relacional, catálogo de migrações e boas práticas com o Toasty ORM.

### 🧪 Executando os Testes da API

- **Testes Automatizados em Rust (100% de cobertura de contratos e integração):**
  ```bash
  cargo test --manifest-path rust/Cargo.toml
  ```
- **Testes Manuais / Interativos com o plugin REST Client:**
  Consulte a pasta [`http/`](file:///home/jc/codigos/rust/player-scout/http) e abra [`http/99_full_e2e_workflow.http`](file:///home/jc/codigos/rust/player-scout/http/99_full_e2e_workflow.http) para executar o fluxo sequencial auto-encadeado.

---

## 🚦 Acompanhamento do Projeto

Para verificar o progresso atual e em qual etapa estamos trabalhando:
- Consulte o [ROADMAP.md](file:///home/jc/codigos/rust/player-scout/ROADMAP.md).
- Consulte o histórico de alterações no [CHANGELOG.md](file:///home/jc/codigos/rust/player-scout/CHANGELOG.md).
