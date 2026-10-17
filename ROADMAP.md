# Player Scout — Roadmap do Projeto

Este arquivo é a fonte oficial do estado de evolução do projeto. Toda fase e etapa concluída deve ser marcada aqui com `[x]`, e a etapa atualmente em desenvolvimento deve ser indicada com a marcação `📍 ETAPA ATUAL`.

---

## 📊 Visão Geral do Progresso

| Fase | Descrição | Status | Progresso |
| :--- | :--- | :---: | :---: |
| **Fase 0** | Definições de Domínio, Wireframes & Documentação | **Concluído** | 100% |
| **Fase 1** | Banco de Dados PostgreSQL 18+ & Modelagem com Toasty ORM | **Concluído** | 100% |
| **Fase 2** | Backend Rust (Axum, API REST & Ingestão em Quadra) | **Concluído** | 100% |
| **Fase 3** | Motor Analítico, Agregação Recursiva & Modo Comparativo | **Concluído** | 100% |
| **Fase 4** | Frontend Web (Askama, HTMX 4.x, Chart.js & Mobile Touch) | **Concluído** | 100% |
| **Fase 5** | Clientes Móveis (Kotlin / KMP para Android e iOS) | **Pendente** | 0% |
| **Fase 6** | Testes Automatizados 100% em Rust, CI/CD & Auditoria | **Em Andamento** | 85% |

> **Legenda de Status:**
> - `[x]` Concluído
> - `[~]` Em andamento
> - `[ ]` Não iniciado / Pendente

---

## 🗺️ Detalhamento das Etapas

### Fase 0: Planejamento, UX & Documentação
- [x] Levantamento e definição do catálogo de jogadas do Padel (21 golpes mapeados).
- [x] Definição da regra de negócio de registro:
  - Resultado obrigatório: `Acertou` ou `Errou`.
  - Qualidade de decisão: `Boa` (padrão) com toggle opcional para `Ruim`.
  - Validações: `Winner` restrito a `Acertou`; `Erro Não Forçado` força `Errou`.
- [x] Wireframe interativo HTML com Tailwind (Dashboard, Matriz Decisão x Execução, Modo Comparativo e Tela Touch-first do Treinador).
- [x] Definição de requisitos de ergonomia: foco em Celular e Tablet, sem teclas de atalho, com botões primários na *Thumb Zone*.
- [x] Arquitetura de design responsivo e recursivo (drill-down estruturado).
- [x] Definição da stack: Rust Axum, Askama, Toasty ORM, PostgreSQL 18+, HTMX 4.x e Chart.js.
- [x] Atualização da documentação do repositório (`README.md`, `ROADMAP.md` e `CHANGELOG.md`).

---

### Fase 1: Banco de Dados PostgreSQL 18+ & Modelagem com Toasty ORM
- [x] Configuração do ambiente PostgreSQL 18+ com healthcheck e ponto de montagem compatível (`/var/lib/postgresql`).
- [x] Inclusão do serviço containerizado `postgres-mcp` (Node 22) no `docker-compose.yml`.
- [x] Definição dos schemas e tabelas relacionais via migração `001_initial_schema.sql`:
  - `coaches` (dados do treinador).
  - `players` (dados do atleta, mão dominante, lado preferido na quadra).
  - `tournaments` (torneios e eventos).
  - `matches` (partidas, adversários, data, status).
  - `shot_events` (eventos individuais de jogadas com set, tipo de golpe, resultado, decisão, tags especiais).
- [x] Criação e validação das constraints de integridade:
  - `CHECK (special_tag != 'unforced_error' OR outcome = 'miss')`
  - `CHECK (special_tag != 'winner' OR outcome = 'hit')`
  - `CHECK (set_number >= 1 AND set_number <= 5)`
  - Padrão `good` para qualidade de decisão.
- [x] Criação dos índices compostos para agregação rápida por partida/set e por range de datas do jogador.
- [x] Migração `002_user_roles.sql`: Criação da tabela `users` com perfis `admin` e `coach`.
- [x] Migração `003_player_role_and_multi_coach.sql`: Perfil `player` (read-only), `user_id` em `players` e tabela `coach_players` (N:N múltiplos treinadores).
- [x] Configuração e mapeamento das entidades usando o ORM **Toasty** (`tokio-rs/toasty` v0.11.0) no backend Rust (`src/db/models.rs` e `src/db/mod.rs`).

---

### Fase 2: Backend em Rust (`/rust`) com Axum
- [x] Criação do workspace Cargo em `/rust` (Rust Edition 2024, toolchain 1.99.x+).
- [x] Configuração das dependências centrais: `axum` (v0.8+), `toasty` (v0.11+), `askama` (v0.16+), `tokio`, `serde`, `tracing`, `thiserror`.
- [x] Implementação dos tipos de domínio fortemente tipados:
  - Enums para `ShotType` (21 golpes), `ShotOutcome`, `DecisionQuality`, `SpecialTag`.
  - Perfis de usuário `UserRole` (`Admin`, `Coach`, `Player`) com permissões (`can_write_shots`, `is_read_only`).
  - Entidades `Player`, `CoachPlayer` (relação N:N), `MatchSession`.
  - 8 testes unitários de regras esportivas com 100% de aprovação.
- [x] Configuração do Toasty ORM e pool de conexão com PostgreSQL 18.
- [x] Endpoints REST de Gestão (`src/api/`):
  - `POST /api/v1/users`, `GET /api/v1/users`, `GET /api/v1/users/{id}`
  - `POST /api/v1/players`, `GET /api/v1/players`, `GET /api/v1/players/{id}`
  - `POST /api/v1/players/{id}/coaches`, `GET /api/v1/players/{id}/coaches`
  - `POST /api/v1/matches`, `GET /api/v1/matches/{id}`
- [x] Endpoints de Registro em Quadra (Baixa Latência):
  - `POST /api/v1/matches/{id}/shots` e `GET /api/v1/matches/{id}/shots` (ingestão atômica de jogada com confirmação instantânea).
  - `POST /api/v1/matches/{id}/shots/undo` (desfazer o último registro ordenado por UUIDv7 com rollback seguro).

---

### Fase 3: Motor Analítico & Modo Comparativo
- [x] Implementação de queries agregadas e métricas recursivas (`src/domain/analytics.rs`):
  - Taxa geral de acerto (% In vs % Out).
  - Qualidade de decisão (% Boa vs % Ruim).
  - Saldo W/UE (`Winners` vs `Erros Não Forçados`).
  - Matriz Decisão × Execução (4 quadrantes: `hit_good`, `hit_bad`, `miss_good`, `miss_bad`).
  - Eficiência por zona (Início, Rede/Ataque, Fundo/Vidros).
  - Radar de habilidades (6 fases do Padel).
- [x] Endpoint de Agregação Recursiva (`src/api/analytics.rs`):
  - `GET /api/v1/analytics/players/{id}` com filtros flexíveis por período ou escopo (`match_id`, `set_number`, range de datas).
- [x] Endpoint de Comparação Lado a Lado (`src/api/analytics.rs`):
  - `POST /api/v1/analytics/compare` recebendo Escopo A e Escopo B (ex: Set 1 vs Set 2, ou Mês 1 vs Mês 2) e retornando métricas pareadas e deltas (▲/▼).

---

### Fase 4: Frontend Web (Askama, HTMX 4.x & Chart.js)
- [x] Configuração de templates server-side compilados com **Askama** (`0.16.1`) em `rust/templates/`.
- [x] Integração com **HTMX** para interações dinâmicas e troca de fragmentos parciais (`/web/partials/*`).
- [x] Interface do Treinador em Quadra (Touch-First) (`GET /web/court`):
  - Layout responsivo para celular e tablet sem atalhos de teclado.
  - Alvos de toque grandes (mínimo `48px` para seletores e `64px` para botões da *Thumb Zone*).
  - Barra de polegar (*Thumb Zone*) com botões `ACERTOU (IN)` e `ERROU (OUT)` fixos na base.
  - Toggle de *Decisão Ruim* e botões de *Winner* e *Erro Não Forçado (ENF)* com auto-vínculo de resultado (`ENF` força `Miss`, `Winner` força `Hit`).
  - Botão de *Desfazer (Undo)* visível no topo da tela.
  - Feedback tátil com API `navigator.vibrate` e atualização parcial do feed via HTMX (`POST /web/partials/court/shot` e `POST /web/partials/court/undo`).
- [x] Dashboard do Atleta & Visualizações com **Chart.js 4.4.9** (auditado contra CVEs) (`GET /` e `GET /web/dashboard`):
  - Cards de KPIs reativos com filtros recursivos de escopo (Carreira, Partida, Set 1, Set 2).
  - Matriz Tática Decisão × Execução em 4 quadrantes.
  - Gráfico Radar (Spider Chart) das 6 fases do Padel renderizado com Chart.js `4.4.9`.
  - Tabela e indicadores de aproveitamento por zona e por tipo de jogada.
- [x] Visualização do Modo Comparativo (`GET /web/partials/compare`):
  - Interface para comparar `Set 1 vs Set 2` com cartões de deltas (▲/▼) e barras pareadas de aproveitamento por golpe.

---

### Fase 5: Clientes Móveis (Kotlin / KMP)
- [ ] Avaliação da arquitetura Kotlin Multiplatform (KMP). 📍 **ETAPA ATUAL**
- [ ] Implementação do módulo compartilhado (camada de rede, modelos e cache offline para uso em quadra sem sinal).
- [ ] Cliente Android (Android 13+) em Jetpack Compose.
- [ ] Camada cliente iOS (avaliação de Compose Multiplatform ou camada hospedeira Swift conforme [AGENTS.md](file:///home/jc/codigos/rust/player-scout/AGENTS.md)).

---

### Fase 6: Testes Automatizados (100% em Rust) & Auditoria
- [x] Testes unitários em Rust para validações de regras de negócio esportivas.
- [x] Testes de integração em Rust contra o PostgreSQL 18+ (validando persistência, constraints e queries do Toasty).
- [x] Testes em Rust para cálculos matemáticos analíticos e deltas do comparativo.
- [x] Testes de contrato de interface Web (validando respostas HTML/Askama, Chart.js 4.4.9, fluxo Touch-First e fragmentos HTMX).
- [x] Pipeline de validação estática: `cargo fmt --check`, `cargo clippy -- -D warnings`.
- [ ] Auditoria de segurança de dependências com `cargo audit`.

---

## 📌 Regra de Atualização do Roadmap

Sempre que uma etapa for concluída ou iniciada:
1. Atualize o marcador `[x]` ou `[~]` correspondente.
2. Posicione o marcador `📍 ETAPA ATUAL` na tarefa que está sendo executada no momento.
3. Registre o progresso equivalente no [CHANGELOG.md](file:///home/jc/codigos/rust/player-scout/CHANGELOG.md).
