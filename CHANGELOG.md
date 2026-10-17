# Changelog

Todas as mudanças notáveis deste projeto serão documentadas neste arquivo.

O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/) e este projeto segue [Semantic Versioning](https://semver.org/lang/pt-BR/).

> **⚠️ Regra do Projeto:** Todo commit, implementação de funcionalidade, refatoração ou correção de defeito deve ser documentado neste arquivo, mantendo o histórico de evolução auditável e transparente.

---

## [Não lançado] - Unreleased

### Adicionado
- **Definições de Domínio do Padel:**
  - Catálogo de 21 jogadas agrupadas em três zonas táticas:
    - *Início de Ponto:* Saque e Recepção.
    - *Rede & Ataque:* Voleio Direita, Voleio Esquerda, Bandeja, Víbora, Smash Trazer, Smash x3, Smash x4, Rulo na Grade, Deixadinha (Drop shot), Bloqueio.
    - *Fundo & Vidros:* Batida Direita, Batida Esquerda, Globo Direita, Globo Esquerda, Chiquita, Saída Parede Direita, Saída Parede Esquerda, Bajada de Parede, Contra-parede.
  - Regra de registro obrigatório: `Acertou` (In) ou `Errou` (Out).
  - Regra de qualidade de decisão: padrão `Boa`, com alternador opcional para `Decisão Ruim`.
  - Validações de integridade esportiva: `Winner` restrito a jogadas com resultado `Acertou`; `Erro Não Forçado (ENF)` computa e força automaticamente o resultado como `Errou`.
- **Wireframe Interativo do Sistema:**
  - Arquivo interativo HTML/Tailwind com Dashboard Analítico e Tela de Registro do Treinador.
  - Matriz Decisão × Execução em 4 quadrantes (*Acertou + Boa*, *Acertou + Ruim*, *Errou + Boa*, *Errou + Ruim*).
  - Gráfico Radar (Spider Chart) com as 6 fases do Padel.
  - Modo Comparativo (Set 1 vs Set 2 / Período A vs Período B) com deltas percentuais automáticos e barras pareadas de aproveitamento por golpe.
- **Ergonomia e UX Touch-First:**
  - Interface desenhada especificamente para celular e tablet, sem teclas de atalho.
  - Botões de ação rápida `ACERTOU` e `ERROU` fixados na base (*Thumb Zone*) com áreas de toque confortáveis (mínimo de 48px) e feedback tátil por vibração.
- **Arquitetura de Design Responsivo e Recursivo:**
  - Modelo de componentes analíticos aplicáveis com a mesma consistência visual em qualquer nível da hierarquia:
    `Jogador -> Temporada/Ano -> Torneio/Mês -> Partida -> Set -> Jogada`.
- **Definição da Stack Tecnológica:**
  - Rust Edition 2024 (toolchain 1.99.x+) com **Axum** para roteamento e HTTP.
  - **Toasty ORM** (`tokio-rs/toasty`) para mapeamento e acesso assíncrono ao **PostgreSQL 18+**.
  - **Askama** para templates HTML compilados e type-safe.
  - **HTMX 4.x** e **htmx-ui** para hipertexto dinâmico e reativo no frontend.
  - **Chart.js** auditado para visualização de dados.
- **Documentação do Repositório:**
  - Criação do [ROADMAP.md](file:///home/jc/codigos/rust/player-scout/ROADMAP.md) com acompanhamento detalhado de etapas, status percentual e indicador de etapa atual.
  - Criação deste [CHANGELOG.md](file:///home/jc/codigos/rust/player-scout/CHANGELOG.md) com diretrizes de documentação contínua.
  - Atualização completa do [README.md](file:///home/jc/codigos/rust/player-scout/README.md) detalhando escopo, stack e estrutura do projeto.
- **Banco de Dados PostgreSQL 18 & MCP (Fase 1):**
  - Criação do `docker-compose.yml` para orquestração local do **PostgreSQL 18-alpine** e do container **`postgres-mcp`** (Node 22) com healthcheck integrado.
  - Configuração do volume persistente no ponto de montagem oficial `/var/lib/postgresql` conforme a arquitetura de clusters do PostgreSQL 18+.
  - Migração inicial [`migrations/001_initial_schema.sql`](file:///home/jc/codigos/rust/player-scout/migrations/001_initial_schema.sql):
    - Enum com os 21 golpes oficiais do Padel (`shot_type_enum`).
    - Schemas e tabelas relacionais: `coaches`, `players`, `tournaments`, `matches` e `shot_events`.
    - Constraints estritas de integridade: `chk_winner_must_be_hit` (Winner exige 'hit'), `chk_unforced_error_must_be_miss` (Erro Não Forçado exige 'miss') e sets de 1 a 5.
    - Decisão tática padrão configurada como `good` em nível de banco.
    - Índices compostos de alta performance para o motor analítico e dashboards recursivos.
  - Segurança de credenciais: remoção de fallbacks hardcoded no `docker-compose.yml` com uso da sintaxe de validação `${VAR:?msg}` e garantia de exclusão do `.env` no `.gitignore`.
  - Validação prática com 100% de aprovação na criação de tabelas, enums, execução de constraints e funcionamento do MCP.
- **Modelagem Relacional Avançada (Perfis & Relação N:N):**
  - Migração `migrations/002_user_roles.sql`: Tabela `users` com `role user_role_enum` (`admin` e `coach`).
  - Migração `migrations/003_player_role_and_multi_coach.sql`: Perfil `player` (read-only), adição de `user_id` em `players` e criação da tabela associativa `coach_players` permitindo múltiplos treinadores por jogador.
- **Backend Rust & Camada de Domínio (Fase 2):**
  - Inicialização do workspace Cargo em `/rust` (Rust Edition 2024, toolchain 1.99.1).
  - Dependências configuradas nas versões mais recentes: `axum` (0.8.9), `toasty` (0.11.0 com postgresql), `askama` (0.16.1), `tokio` (1.43), `thiserror` (2.0), `uuid`, `chrono`, `tower-http`.
  - Módulos de domínio tipados: `domain::user` (`Admin`, `Coach`, `Player`), `domain::shot` (21 golpes do Padel, `ShotOutcome`, `DecisionQuality`, `SpecialTag`), `domain::player` (`Player`, `CoachPlayer`), `domain::match_session`.
  - Tratamento de erros centralizado (`error::AppError` implementando `IntoResponse` do Axum) e configuração de ambiente (`config::AppConfig`).
  - Router Axum e endpoint de health check (`/health` e `/api/v1/health`).
  - Suíte de 8 testes unitários automatizados em Rust com 100% de aprovação e validação estrita com `cargo fmt` e `cargo clippy`.
