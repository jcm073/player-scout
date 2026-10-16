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
