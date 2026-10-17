use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::Database,
    domain::{AuthenticatedUser, User, UserRole},
    error::{AppError, Result},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserRequest {
    pub role: UserRole,
    pub name: String,
    pub email: String,
    #[serde(default = "default_password_hash")]
    pub password_hash: String,
}

fn default_password_hash() -> String {
    "argon2id_placeholder_hash".to_string()
}

/// Cria um usuário administrativo/interno (`POST /api/v1/users`)
/// - Restrito a `Coach` e `Admin` (bloqueado para `Player` - LGPD / RBAC)
/// - Apenas um `Admin` pode criar outro usuário com perfil `Admin`
pub async fn create_user(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<User>)> {
    auth_user.require_coach_or_admin()?;
    if payload.role == UserRole::Admin && !auth_user.user.role.is_admin() {
        return Err(AppError::Forbidden(
            "Apenas um Administrador (Admin) pode criar contas com perfil Admin".to_string(),
        ));
    }

    let user = db
        .create_user(
            payload.role,
            payload.name,
            payload.email,
            payload.password_hash,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(user)))
}

/// Consulta os dados cadastrais de um usuário (`GET /api/v1/users/{id}`)
/// - Proteção LGPD / BOLA: um `Player` só pode consultar o seu próprio cadastro (`id == auth_user.user.id`)
pub async fn get_user(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<User>> {
    if auth_user.user.role == UserRole::Player && id != auth_user.user.id {
        return Err(AppError::Forbidden(
            "Proteção de privacidade (LGPD): um jogador só pode visualizar os seus próprios dados cadastrais"
                .to_string(),
        ));
    }
    let user = db.get_user_by_id(id).await?;
    Ok(Json(user))
}

/// Lista todos os usuários cadastrados (`GET /api/v1/users`)
/// - Proteção de privacidade (LGPD): restrito a `Coach` e `Admin`
pub async fn list_users(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<User>>> {
    auth_user.require_coach_or_admin()?;
    let users = db.list_users().await?;
    Ok(Json(users))
}
