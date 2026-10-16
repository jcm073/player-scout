# Player Scout: instrucoes do projeto

Este arquivo e a fonte central dos requisitos tecnicos e das convencoes do Player Scout. Leia-o antes de propor ou implementar mudancas. Claude Code e Gemini CLI devem carregar este arquivo como instrucoes do projeto.

## Objetivo do produto

O Player Scout ajuda treinadores a registrar e analisar o desempenho de jogadores em esportes de raquete, como padel, tenis de mesa e pickleball. Para cada jogada, o treinador registra o resultado (certa ou errada); o sistema preserva o historico e apresenta estatisticas de evolucao por data ou periodo (dias, meses e anos).

## Arquitetura desejada

O produto tem tres partes:

1. **Backend:** servico em Rust, dentro de `/rust`, expondo uma API REST que recebe e retorna JSON.
2. **Web:** interface baseada em HTMX 4.x e htmx-ui, com graficos em Chart.js. Minimize JavaScript e TypeScript; use-os somente quando a plataforma ou uma interacao exigir.
3. **Aplicativos moveis:** cliente para Android 13 ou superior e cliente para iOS. Kotlin e o requisito de linguagem indicado para os clientes. Antes de escolher a arquitetura iOS, verifique o estado do repositorio e avalie Kotlin Multiplatform; documente qualquer necessidade de uma camada hospedeira Swift para APIs de interface da Apple. Nao substitua o requisito por outra tecnologia sem aprovacao.

## Stack e requisitos tecnicos

- Rust Edition 2024 e toolchain Rust 1.99.x ou superior. Confirme a disponibilidade da versao solicitada no ambiente antes de configurar o projeto; nao reduza o requisito silenciosamente.
- PostgreSQL 18 ou superior.
- `cargo clippy` para analise estatica e `cargo-audit` (`cargo audit`) para consultar vulnerabilidades conhecidas em dependencias Rust.
- Chart.js para visualizacao de dados no cliente web.
- Use as versoes compativeis mais recentes das dependencias. Atualize de forma incremental e mantenha compatibilidade com a toolchain, a plataforma e as demais dependencias.

## Seguranca e dependencias

- Antes de adicionar ou atualizar uma dependencia, confira manutencao, compatibilidade, licenca e avisos de seguranca conhecidos nas fontes oficiais e nos respectivos bancos de avisos.
- Para dependencias Rust, revise a resolucao com Cargo e execute `cargo audit` quando a ferramenta estiver disponivel. Para dependencias web, use o mecanismo de auditoria do gerenciador de pacotes adotado pelo repositorio.
- Antes de adicionar, atualizar ou publicar uma versao de Chart.js, verifique especificamente os avisos de seguranca conhecidos e use uma versao corrigida compativel. Nao presuma que uma versao e segura sem verificar.
- Nao ignore um aviso de vulnerabilidade para concluir uma tarefa. Informe o impacto, a dependencia afetada e as opcoes de correcao ou mitigacao.
- Nao inclua segredos, credenciais, dados reais de jogadores ou informacoes pessoais em codigo, logs, testes ou exemplos.

## Regras de trabalho

- Inspecione o codigo e os testes existentes antes de escolher uma implementacao; siga as convencoes reais do repositorio quando elas nao conflitarem com estes requisitos.
- Faca mudancas pequenas e focadas. Nao reverta nem sobrescreva alteracoes preexistentes do usuario.
- Preserve a separacao entre backend, web, aplicativos moveis e banco de dados. Ao cruzar limites entre componentes, explicite contratos, formatos e responsabilidades.
- Valide entradas e trate erros de forma consistente na API. Considere privacidade e controle de acesso para historicos e estatisticas de jogadores.
- Escreva todos os testes automatizados do projeto em Rust, inclusive os que validam contratos ou comportamentos da web, dos aplicativos moveis e do banco de dados. Nao introduza testes em outras linguagens.
- Execute a suite de testes aplicavel e exija 100% de aprovacao antes de considerar a tarefa concluida. Se algum teste falhar, investigue e corrija a causa; nunca o ignore, desative ou enfraqueca para obter aprovacao.
- Antes de alterar qualquer teste, analise o codigo de producao e confirme se a falha vem de um defeito de implementacao ou de um teste comprovadamente incorreto ou desatualizado. Nao mude um teste apenas para faze-lo passar; preserve sua intencao e explique a justificativa de qualquer ajuste.
- Rode os testes e verificacoes relevantes para os arquivos alterados. Para Rust, use pelo menos `cargo fmt --check`, `cargo clippy` e os testes aplicaveis; rode `cargo audit` quando disponivel. Relate verificacoes que nao puder executar.
- Nao adicione frameworks, dependencias ou camadas de abstracao sem necessidade demonstravel.

## Delegacao a especialistas

Use os subagentes do projeto quando a tarefa exigir conhecimento especializado ou puder ser investigada em paralelo. Encaminhe cada parte ao especialista correspondente e sintetize os resultados antes de alterar contratos compartilhados.

- Rust e API: `rust-specialist`.
- Implementacao web, HTMX e Chart.js: `frontend-specialist`.
- Fluxos, acessibilidade e arquitetura de informacao: `ux-specialist`.
- PostgreSQL, esquema, consultas e migracoes: `postgres-dba`.
- Aplicativos Android/iOS e Kotlin Multiplatform: `mobile-specialist`.
- Vulnerabilidades, dependencias e revisao de seguranca: `security-specialist`.

Todo especialista deve primeiro ler este guia, permanecer dentro do seu escopo, conferir o estado dos arquivos que pretende alterar e devolver uma sintese objetiva com evidencias, riscos, mudancas e verificacoes realizadas.


