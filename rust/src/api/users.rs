use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::Database,
    domain::{User, UserRole},
    error::Result,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserRequest {
    pub role: UserRole,
    pub name: String,
    pub email: String,
    pub password_hash: String,
}

pub async fn create_user(
    State(db): State<Database>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<User>)> {
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

pub async fn get_user(State(db): State<Database>, Path(id): Path<Uuid>) -> Result<Json<User>> {
    let user = db.get_user_by_id(id).await?;
    Ok(Json(user))
}

pub async fn list_users(State(db): State<Database>) -> Result<Json<Vec<User>>> {
    let users = db.list_users().await?;
    Ok(Json(users))
}
