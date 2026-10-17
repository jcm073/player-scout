# Scripts de Teste da API — Plugin REST Client (`http/`)

Esta pasta contém as coleções de requisições HTTP para testar interativamente a API REST do **Player Scout** diretamente no editor (via extensão **REST Client** — `humao.rest-client`).

## 🚀 Pré-requisitos

1. Certifique-se de que o container do **PostgreSQL 18** está em execução:
   ```bash
   docker compose up -d
   ```
2. Inicie o servidor backend em Rust (`/rust`):
   ```bash
   cargo run --manifest-path rust/Cargo.toml
   ```
   O servidor ficará disponível em `http://localhost:3000`.

## 📂 Estrutura dos Arquivos

| Arquivo | Descrição |
| :--- | :--- |
| [`00_health.http`](./00_health.http) | Verificação de saúde do serviço (`/health` e `/api/v1/health`). |
| [`01_users.http`](./01_users.http) | Criação e listagem de usuários nos 3 perfis (`admin`, `coach`, `player`). |
| [`02_players_and_rbac.http`](./02_players_and_rbac.http) | Cadastro de atletas, vínculo N:N com múltiplos treinadores (`coach_players`) e testes de bloqueio RBAC (`403 Forbidden`). |
| [`03_matches_and_shots.http`](./03_matches_and_shots.http) | Abertura de partidas, ingestão dos 21 golpes do Padel, validações de domínio (`422`) e rollback da última jogada (`undo`). |
| [`04_analytics_and_compare.http`](./04_analytics_and_compare.http) | Agregação recursiva (Matriz Decisão × Execução, Radar de 6 fases, Zonas) e Modo Comparativo Lado a Lado (`Set 1 vs Set 2`). |
| [`99_full_e2e_workflow.http`](./99_full_e2e_workflow.http) | **Recomendado:** Fluxo completo ponta a ponta com variáveis encadeadas (`# @name`) e emails únicos gerados via `{{$guid}}`. |

## 💡 Como usar o Fluxo Auto-Encadeado (`99_full_e2e_workflow.http`)

Abra o arquivo [`99_full_e2e_workflow.http`](./99_full_e2e_workflow.http) e clique em **Send Request** em cada bloco de cima para baixo (`Passo 1` até `Passo 16`). O plugin REST Client captura automaticamente os IDs retornados pelo backend (ex.: `{{createCoachTech.response.body.id}}`) e os injeta nas requisições seguintes sem necessidade de copiar e colar UUIDs manualmente.
