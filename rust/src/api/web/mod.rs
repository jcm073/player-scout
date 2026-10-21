//! Camada Web do Player Scout (HTMX + Templates Askama Type-Safe)
//!
//! Este módulo gerencia todas as rotas e interações da interface web do Player Scout.
//! Adota uma arquitetura focada em **HTML sobre o cabo** (*HTML-over-the-wire*), minimizando
//! dependência de frameworks JavaScript pesados e priorizando desempenho, tipagem forte e
//! simplicidade de manutenção.
//!
//! # Paradigmas & Tecnologias Adotadas
//! - **Askama Templates:** Templates HTML compilados em tempo de compilação Rust (`#[derive(Template)]`).
//!   Qualquer erro de sintaxe, variável ausente ou incompatibilidade de tipo no HTML quebra
//!   a compilação antes de atingir o ambiente de produção.
//! - **HTMX 2.x:** Interações dinâmicas (troca de sets, cadastro de jogada rápida em quadra,
//!   filtros analíticos e formulários inline) realizadas através de fragmentos HTML retornados
//!   pelo Axum (`hx-post`, `hx-get`, `hx-swap`, `hx-target`).
//! - **Sessões Seguras:** Sessão única de 12 horas por usuário emitida através do cookie
//!   `HttpOnly`, `SameSite=Lax` denominado `ps_session` (contendo o token assinado `ps_sess_...`).
//! - **Anti-CSRF:** Fluxos OAuth2 e formulários críticos protegidos pelo cookie `ps_oauth_state`.
//!
//! # Submódulos Temáticos
//! A camada Web é dividida em 6 submódulos coesos para facilitar a navegação e manutenção:
//! - [`auth`]: Telas de login, cadastro completo, logoff e fluxo Google OAuth2 / OpenID Connect.
//! - [`dashboard`]: Painel do treinador com resumo de carreira, seleção de atleta e gráficos.
//! - [`court`]: Interface touch-first de quadra com 4 atletas ativos, botões ergonômicos e feed de jogadas.
//! - [`matches`]: Configuração de nova partida, seleção de duplas, criação de sparring e finalização de jogo.
//! - [`analytics`]: Fragmentos HTMX do painel estatístico e comparador lado a lado de sets.
//! - [`preferences`]: Atualização de perfil (lado de quadra, WhatsApp) e troca segura de senha.

pub mod analytics;
pub mod auth;
pub mod court;
pub mod dashboard;
pub mod matches;
pub mod preferences;

pub use analytics::*;
pub use auth::*;
pub use court::*;
pub use dashboard::*;
pub use matches::*;
pub use preferences::*;
