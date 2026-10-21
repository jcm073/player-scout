//! # Camada de Persistência & Repositório (Toasty ORM + PostgreSQL 18)
//!
//! Este módulo gerencia o pool de conexões assíncronas, o mapeamento de tabelas e o repositório
//! centralizado de consultas sobre o banco relacional **PostgreSQL 18** utilizando o **Toasty ORM**.
//!
//! ## Arquitetura Modularizada
//! - **`users`:** Autenticação Argon2i, sessões com TTL, OAuth2 (Google), RBAC e perfis.
//! - **`players`:** Atletas, perfis desportivos e associações treinador-atleta.
//! - **`matches`:** Partidas, convocação de duplas/jogadores e encerramento.
//! - **`shots`:** Ingestão ultra-rápida de jogadas em quadra e operação de undo.
//! - **`analytics`:** Agregação de estatísticas evolutivas e comparações temporais.

pub mod analytics;
pub mod matches;
pub mod models;
pub mod players;
pub mod shots;
pub mod users;

pub use shots::NewShotInput;

use crate::{
    db::models::{
        CoachPlayerRecord, MatchPlayerRecord, MatchRecord, PlayerRecord, ShotEventRecord,
        UserOAuthRecord, UserRecord, UserSessionRecord,
    },
    error::Result,
};

/// Repositório central de persistência sobre Toasty ORM + PostgreSQL 18
#[derive(Clone)]
pub struct Database {
    pub(crate) inner: toasty::Db,
}

impl Database {
    /// Inicializa o pool de conexões do Toasty ORM registrando todos os modelos e enums
    pub async fn connect(database_url: &str) -> Result<Self> {
        let db = toasty::Db::builder()
            .models(toasty::models!(
                UserRecord,
                UserSessionRecord,
                UserOAuthRecord,
                PlayerRecord,
                CoachPlayerRecord,
                MatchPlayerRecord,
                MatchRecord,
                ShotEventRecord,
            ))
            .connect(database_url)
            .await?;

        Ok(Self { inner: db })
    }

    pub fn raw(&self) -> toasty::Db {
        self.inner.clone()
    }
}
