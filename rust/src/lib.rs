//! # Player Scout: Core Engine & API Server
//!
//! O **Player Scout** é uma plataforma analítica desenvolvida em Rust para auxiliar treinadores
//! e atletas de esportes de raquete (com foco principal em Padel) a registrar jogadas ponto a ponto,
//! avaliar tomadas de decisão e visualizar relatórios estatísticos de evolução tática.
//!
//! ## Arquitetura em Camadas (Domain-Driven Design)
//! - [`domain`]: Entidades puras, regras de negócio invariantes (Winner/Erro Não Forçado, 21 jogadas de Padel)
//!   e algoritmos analíticos (Matriz Decisão × Execução, Radar por Zonas). Não realiza I/O nem acessa banco.
//! - [`db`]: Mapeamento do Toasty ORM e repositório centralizado sobre PostgreSQL 18.
//! - [`api`]: Camada de apresentação HTTP Axum, dividida entre endpoints REST JSON (`/api/v1`) e a
//!   interface web interativa baseada em Askama e HTMX (`/web` e [`api::web`]).
//! - [`config`]: Carregamento e validação type-safe das variáveis de ambiente (`AppConfig`).
//! - [`error`]: Tratamento unificado de erros com [`AppError`] e códigos HTTP correspondentes.
//!
//! ## Camada Web Modularizada ([`api::web`])
//! A interface Web é segmentada em submódulos coesos para facilitar a manutenção:
//! - [`api::web::auth`]: Login, cadastro completo, logout e integração Google OAuth2 (OpenID Connect).
//! - [`api::web::dashboard`]: Painel do treinador com seletor de atleta, histórico de partidas e métricas de carreira.
//! - [`api::web::court`]: Interface touch-first de quadra com 4 atletas ativos, registro de golpe com 1 clique e feed.
//! - [`api::web::matches`]: Setup de nova partida, criação rápida de sparring (duplas) e finalização de jogo.
//! - [`api::web::analytics`]: Fragmentos HTMX do painel analítico recursivo e comparação de sets lado a lado.
//! - [`api::web::preferences`]: Formulários inline de atualização cadastral e rotação segura de senhas.

pub mod api;
pub mod config;
pub mod db;
pub mod domain;
pub mod error;

pub use config::AppConfig;
pub use db::Database;
pub use error::{AppError, Result};

/// Cria a aplicação Axum com todas as rotas e o pool Toasty configurados
pub fn create_app(db: Database) -> axum::Router {
    api::app_router(db)
}
