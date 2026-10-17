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
| **Fase 4.5** | Autenticação, Cadastro Completo, Hash Argon2i & OAuth2 | **Concluído** | 100% |
| **Fase 5** | Clientes Móveis (Kotlin / KMP para Android e iOS) | **Pendente** | 0% |
| **Fase 6** | Testes Automatizados 100% em Rust, CI/CD & Auditoria | **Em Andamento** | 90% |

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

### Fase 4.5: Autenticação, Cadastro Completo, Hash Argon2i & OAuth2
- [x] Migração `004_auth_registration_and_oauth.sql` adicionando campos cadastrais obrigatórios (`first_name`, `last_name`, `age`, `cpf` UNIQUE, `whatsapp_phone`, `cep`, `street`, `street_number`, `complement` até 10 caracteres, `neighborhood`, `city`, `state` UF e `preferred_side` [posição que joga: Direita/Drive ou Esquerda/Revés]) e tabela `user_oauth_accounts`.
- [x] Segurança de Senha com **Argon2i** (`argon2 = "0.5.3"`):
  - Validação estrita exigindo mínimo de **12 caracteres**, ao menos **1 letra maiúscula**, **1 letra minúscula**, **1 número** e **1 caractere especial**.
  - Armazenamento em formato PHC `$argon2i$v=19$...` com salt criptográfico por usuário.
- [x] Validações Cadastrais e Criação Automática do Perfil `Player`:
  - Validação matemática dos dígitos verificadores do **CPF** (Módulo 11) e normalização de CPF (11 dígitos) e CEP (8 dígitos).
  - Validação estrita do **Complemento**: limitado a até 10 caracteres alfanuméricos sem caracteres especiais (podendo ficar em branco para casas sem bloco/apto).
  - Criação automática do perfil esportivo na tabela `players` quando o cadastro é do tipo `Player`, incluindo a posição em quadra (`preferred_side`).
- [x] Login Exclusivo por E-mail & Chaves Únicas de Identificação (`CPF` e `Email`):
  - `POST /api/v1/auth/register` e `POST /api/v1/auth/login` (restrito a E-mail + senha Argon2i).
  - `GET /api/v1/auth/lookup` permitindo localizar o usuário tanto por `cpf` quanto por `email`.
  - Telas Web SSR + HTMX (`GET /web/login`, `GET /web/register`, `POST /web/partials/auth/login` e `POST /web/partials/auth/register`).
- [x] Estrutura de Integração OAuth2 (`Google/Gmail`, `Microsoft Office` e `Instagram`):
  - Endpoints `GET /api/v1/auth/oauth/{provider}/authorize` e `POST /api/v1/auth/oauth/{provider}/callback` mapeados no Toasty ORM (`UserOAuthRecord`).
- [x] **Task Dedicada OAuth2 (Homologação de Provedores Externos):**
  - Implementação completa do Login Social OAuth2 (`Google/Gmail`, `Microsoft Office` e `Instagram`).
  - Proteção Anti-CSRF via cookie HttpOnly `ps_oauth_state` (10 min) validado estritamente no callback.
  - Emissão de sessão única de 12 horas com hash `Argon2i` (`ps_session` / `ps_sess_...`) no login social com substituição física de sessões anteriores.
  - Fluxo Web (`GET /web/auth/oauth/{provider}/authorize` e `GET /web/auth/oauth/{provider}/callback`) com redirecionamento inteligente para `/web/register` (pré-preenchendo e-mail) para novos usuários ou `/web/dashboard` para usuários já cadastrados.
  - Documentação de homologação nos consoles oficiais (Google Cloud, Microsoft Entra ID e Meta) e suporte a credenciais de cliente no `.env`.

---

### Fase 4.6 & 4.7: Controle de Sessão Única (12h), Força Bruta & Hardening de Segurança (Concluída ✅)
- [x] **Controle de Sessão Única com Duração de 12 Horas (`43.200s`) e Exclusão Física (`DELETE`):**
  - Migração [005_user_sessions_and_brute_force.sql](file:///home/jc/codigos/rust/player-scout/migrations/005_user_sessions_and_brute_force.sql) criando `user_sessions` (`UNIQUE (user_id)`) e colunas `failed_login_attempts` / `locked_until_epoch` em `users`.
  - Uso unificado da mesma biblioteca `argon2` (`Argon2i`) tanto para senhas quanto para o hash de verificação dos tokens de sessão (`ps_sess_<uuid_v7_simple>_<secret>`), sem necessidade de JWT.
  - Middleware `require_session_middleware` exigindo sessão ativa em todas as páginas Web (`/`, `/web/dashboard`, `/web/court`, `/web/partials/*`) e rotas protegidas da API (`/api/v1/*`).
  - Exclusão física imediata (`DELETE FROM user_sessions`) no logoff (`POST /api/v1/auth/logout` e `POST /web/auth/logout`), na expiração de 12 horas e ao iniciar novo login do mesmo usuário.
- [x] **Proteção contra Força Bruta & Mitigação de Timing Attack no Login:**
  - Bloqueio automático da conta por **15 minutos** (`429 Too Many Requests`) após **5 tentativas consecutivas** de senha incorreta.
  - Execução de `verify_dummy_password_argon2i` quando o e-mail não existe no banco e mensagens uniformes (`"E-mail ou senha inválidos"`) nas tentativas 1..4 para impedir enumeração de contas.
- [x] **Hardening de Controles de Acesso (RBAC, Anti-Spoofing / BOLA, LGPD, Rotação de Senha & Anti-CSRF):**
  - Bloqueio de escalação de privilégio no auto-cadastro público (`role: Admin` rejeitado com `422 FORBIDDEN_ROLE_IN_REGISTRATION`).
  - Proteção Anti-Spoofing / BOLA (`AuthenticatedUser::resolve_actor_id`): impede que `Player` ou `Coach` falsifiquem o `actor_id` de outro usuário na query string ou no payload JSON (`403 Forbidden`).
  - Proteção de Dados Pessoais (LGPD): restringe `GET /api/v1/users`, `POST /api/v1/users` e `GET /api/v1/auth/lookup` a `Coach` e `Admin`, e restringe `GET /api/v1/users/{id}` para que um `Player` acesse somente o seu próprio registro.
  - Endpoint `POST /api/v1/auth/password` para troca de senha com validação `Argon2i` e rotação imediata da sessão única de 12 horas.
  - Defesa Anti-CSRF baseada em Fetch Metadata (`Sec-Fetch-Site: cross-site` bloqueado com `403 Forbidden`) e cabeçalhos `Cache-Control: no-store, no-cache, must-revalidate` e `Content-Security-Policy`.

---

### Fase 5: Clientes Móveis (Kotlin / KMP)
- [ ] Avaliação da arquitetura Kotlin Multiplatform (KMP). 📍 **ETAPA ATUAL**
- [ ] Implementação do módulo compartilhado (camada de rede, modelos e cache offline para uso em quadra sem sinal).
- [ ] Cliente Android (Android 13+) em Jetpack Compose.
- [ ] Camada cliente iOS (avaliação de Compose Multiplatform ou camada hospedeira Swift conforme [AGENTS.md](file:///home/jc/codigos/rust/player-scout/AGENTS.md)).

---

### Fase 6: Testes Automatizados (100% em Rust) & Auditoria
- [x] Testes unitários em Rust para validações de regras de negócio esportivas, CPF (Módulo 11), complemento, política de senha Argon2i, tokens de sessão Argon2i (12h), bloqueio de escalação `Admin` e resolução BOLA de `actor_id` (15 testes unitários).
- [x] Testes de integração em Rust contra o PostgreSQL 18+ (validando persistência, constraints, sessão única de 12h, exclusão no logoff/expiração, força bruta, BOLA, LGPD, rotação de senha e Anti-CSRF).
- [x] Testes em Rust para cálculos matemáticos analíticos e deltas do comparativo.
- [x] Testes de contrato de interface Web e Autenticação (validando respostas HTML/Askama, Chart.js 4.4.9, fluxo Touch-First, cadastro completo, login Argon2i e OAuth2).
- [x] Pipeline de validação estática: `cargo fmt --check`, `cargo clippy -- -D warnings`.
- [ ] Auditoria de segurança de dependências com `cargo audit`.

---

## 📌 Regra de Atualização do Roadmap

Sempre que uma etapa for concluída ou iniciada:
1. Atualize o marcador `[x]` ou `[~]` correspondente.
2. Posicione o marcador `📍 ETAPA ATUAL` na tarefa que está sendo executada no momento.
3. Registre o progresso equivalente no [CHANGELOG.md](file:///home/jc/codigos/rust/player-scout/CHANGELOG.md).
