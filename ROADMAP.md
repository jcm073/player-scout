# Player Scout — Roadmap do Projeto

Este arquivo é a fonte oficial do estado de evolução do projeto. Toda fase e etapa concluída deve ser marcada aqui com `[x]`, e a etapa atualmente em desenvolvimento deve ser indicada com a marcação `📍 ETAPA ATUAL`.

---

## 📊 Visão Geral do Progresso

| Fase | Descrição | Status | Progresso |
| :--- | :--- | :---: | :---: |
| **Fase 0** | Definições de Domínio, Wireframes & Documentação | **Concluído** | 100% |
| **Fase 1** | Banco de Dados PostgreSQL 18+ & Modelagem com Toasty ORM | **Em Andamento** | 80% |
| **Fase 2** | Backend Rust (Axum, API REST & Ingestão em Quadra) | **Pendente** | 0% |
| **Fase 3** | Motor Analítico, Agregação Recursiva & Modo Comparativo | **Pendente** | 0% |
| **Fase 4** | Frontend Web (Askama, HTMX 4.x, Chart.js & Mobile Touch) | **Pendente** | 0% |
| **Fase 5** | Clientes Móveis (Kotlin / KMP para Android e iOS) | **Pendente** | 0% |
| **Fase 6** | Testes Automatizados 100% em Rust, CI/CD & Auditoria | **Pendente** | 0% |

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
- [~] Configuração e mapeamento das entidades usando o ORM **Toasty** (`tokio-rs/toasty`) no backend Rust. 📍 **ETAPA ATUAL**

---

### Fase 2: Backend em Rust (`/rust`) com Axum
- [ ] Criação do workspace Cargo em `/rust` (Rust Edition 2024, toolchain 1.99.x+).
- [ ] Configuração das dependências centrais: `axum`, `tokio`, `toasty`, `serde`, `tracing`, `validator`.
- [ ] Implementação dos tipos de domínio fortemente tipados:
  - Enums para `ShotType`, `ShotOutcome`, `DecisionQuality`, `SpecialTag`.
- [ ] Endpoints REST de Gestão:
  - `POST /api/v1/coaches` / Autenticação
  - `POST /api/v1/players` / `GET /api/v1/players`
  - `POST /api/v1/matches` / `GET /api/v1/matches/{id}`
- [ ] Endpoints de Registro em Quadra (Baixa Latência):
  - `POST /api/v1/matches/{id}/shots` (ingestão atômica de jogada com confirmação instantânea).
  - `POST /api/v1/matches/{id}/shots/undo` (desfazer o último registro com rollback seguro).

---

### Fase 3: Motor Analítico & Modo Comparativo
- [ ] Implementação de queries agregadas e métricas recursivas:
  - Taxa geral de acerto (% In vs % Out).
  - Qualidade de decisão (% Boa vs % Ruim).
  - Saldo W/UE (`Winners` vs `Erros Não Forçados`).
  - Matriz Decisão × Execução (4 quadrantes).
  - Eficiência por zona (Início, Rede/Ataque, Fundo/Vidros).
  - Radar de habilidades (6 fases do Padel).
- [ ] Endpoint de Agregação Recursiva:
  - `GET /api/v1/analytics/players/{id}` com filtros flexíveis por período ou escopo (`match_id`, `set_number`, range de datas).
- [ ] Endpoint de Comparação Lado a Lado:
  - `POST /api/v1/analytics/compare` recebendo Escopo A e Escopo B (ex: Set 1 vs Set 2, ou Mês 1 vs Mês 2) e retornando métricas pareadas e deltas (▲/▼).

---

### Fase 4: Frontend Web (Askama, HTMX 4.x & Chart.js)
- [ ] Configuração de templates server-side compilados com **Askama**.
- [ ] Integração com **HTMX 4.x** para interações dinâmicas sem SPAs pesadas.
- [ ] Interface do Treinador em Quadra (Touch-First):
  - Layout responsivo para celular e tablet.
  - Alvos de toque grandes (mínimo 48px).
  - Barra de polegar (*Thumb Zone*) com botões `ACERTOU` e `ERROU` fixos na base.
  - Toggle de *Decisão Ruim* e botões de *Winner* e *ENF*.
  - Botão de *Undo* visível.
  - Feedback tátil com API `navigator.vibrate` e atualização via HTMX.
- [ ] Dashboard do Atleta & Visualizações com **Chart.js** (auditado):
  - Cards de KPIs reativos com filtros de período.
  - Gráfico Radar (Spider Chart) das 6 fases do Padel.
  - Linha do tempo de evolução de acertos e qualidade tática.
  - Tabela/Barras de aproveitamento por tipo de jogada.
- [ ] Visualização do Modo Comparativo:
  - Interface para comparar Set 1 vs Set 2 ou períodos com cartões de deltas e barras emparelhadas.

---

### Fase 5: Clientes Móveis (Kotlin / KMP)
- [ ] Avaliação da arquitetura Kotlin Multiplatform (KMP).
- [ ] Implementação do módulo compartilhado (camada de rede, modelos e cache offline para uso em quadra sem sinal).
- [ ] Cliente Android (Android 13+) em Jetpack Compose.
- [ ] Camada cliente iOS (avaliação de Compose Multiplatform ou camada hospedeira Swift conforme [AGENTS.md](file:///home/jc/codigos/rust/player-scout/AGENTS.md)).

---

### Fase 6: Testes Automatizados (100% em Rust) & Auditoria
- [ ] Testes unitários em Rust para validações de regras de negócio esportivas.
- [ ] Testes de integração em Rust contra o PostgreSQL 18+ (validando persistência, constraints e queries do Toasty).
- [ ] Testes em Rust para cálculos matemáticos analíticos e deltas do comparativo.
- [ ] Testes de contrato de interface Web (validando respostas HTMX e Askama).
- [ ] Pipeline de validação estática: `cargo fmt --check`, `cargo clippy -- -D warnings`.
- [ ] Auditoria de segurança de dependências com `cargo audit`.

---

## 📌 Regra de Atualização do Roadmap

Sempre que uma etapa for concluída ou iniciada:
1. Atualize o marcador `[x]` ou `[~]` correspondente.
2. Posicione o marcador `📍 ETAPA ATUAL` na tarefa que está sendo executada no momento.
3. Registre o progresso equivalente no [CHANGELOG.md](file:///home/jc/codigos/rust/player-scout/CHANGELOG.md).
