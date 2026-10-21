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
    error::{AppError, ErrorResponse, Result},
};

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateUserRequest {
    pub role: UserRole,
    #[schema(example = "Treinador Silva")]
    pub name: String,
    #[schema(example = "treinador.silva@padel.com")]
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
#[utoipa::path(
    post,
    path = "/api/v1/users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "Usuário criado com sucesso", body = User),
        (status = 400, description = "Dados cadastrais inválidos", body = ErrorResponse),
        (status = 403, description = "Permissão insuficiente para criar o perfil solicitado", body = ErrorResponse)
    ),
    tag = "Users"
)]
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
#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    params(
        ("id" = Uuid, Path, description = "UUID do usuário")
    ),
    responses(
        (status = 200, description = "Dados cadastrais do usuário", body = User),
        (status = 403, description = "Acesso negado para consultar dados de terceiros (LGPD)", body = ErrorResponse),
        (status = 404, description = "Usuário não encontrado", body = ErrorResponse)
    ),
    tag = "Users"
)]
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
#[utoipa::path(
    get,
    path = "/api/v1/users",
    responses(
        (status = 200, description = "Lista de usuários cadastrados", body = Vec<User>),
        (status = 403, description = "Acesso negado para perfil Player (LGPD)", body = ErrorResponse)
    ),
    tag = "Users"
)]
pub async fn list_users(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<User>>> {
    auth_user.require_coach_or_admin()?;
    let users = db.list_users().await?;
    Ok(Json(users))
}

/// Desativa logicamente a conta de um usuário (Soft Delete) (`DELETE /api/v1/users/{id}`)
/// - Proteção RBAC / BOLA / LGPD:
///   - Um `Player` só pode desativar a sua própria conta (`id == auth_user.user.id`)
///   - Um `Coach` pode desativar atletas ou a si próprio
///   - Um `Admin` pode desativar qualquer usuário
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    params(
        ("id" = Uuid, Path, description = "UUID do usuário a ser desativado")
    ),
    responses(
        (status = 200, description = "Conta desativada com sucesso", body = User),
        (status = 403, description = "Violação de permissão de acesso", body = ErrorResponse),
        (status = 404, description = "Usuário não encontrado", body = ErrorResponse)
    ),
    tag = "Users"
)]
pub async fn deactivate_user(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<User>> {
    if auth_user.user.role == UserRole::Player && id != auth_user.user.id {
        return Err(AppError::Forbidden(
            "Violação de controle de acesso (BOLA): um atleta só pode desativar o seu próprio perfil"
                .to_string(),
        ));
    }

    let target_user = db.get_user_by_id(id).await?;
    if target_user.role == UserRole::Admin && !auth_user.user.role.is_admin() {
        return Err(AppError::Forbidden(
            "Apenas administradores podem desativar contas com perfil Admin".to_string(),
        ));
    }

    let deactivated = db.deactivate_user(id).await?;
    Ok(Json(deactivated))
}

/// Reativa logicamente uma conta desativada (`POST /api/v1/users/{id}/reactivate`)
/// - Restrito a Administradores (Admin) e Treinadores (Coach)
#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/reactivate",
    params(
        ("id" = Uuid, Path, description = "UUID do usuário a ser reativado")
    ),
    responses(
        (status = 200, description = "Conta reativada com sucesso", body = User),
        (status = 403, description = "Acesso negado para reativação de contas", body = ErrorResponse),
        (status = 404, description = "Usuário não encontrado", body = ErrorResponse)
    ),
    tag = "Users"
)]
pub async fn reactivate_user(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<User>> {
    auth_user.require_coach_or_admin()?;
    let reactivated = db.reactivate_user(id).await?;
    Ok(Json(reactivated))
}
