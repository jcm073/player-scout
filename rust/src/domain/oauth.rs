use serde::{Deserialize, Serialize};

use super::{OAuthProvider, auth::validate_and_normalize_email};
use crate::error::{AppError, Result};

/// Perfil de usuário normalizado obtido de um provedor OAuth2 / OpenID Connect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUserProfile {
    pub provider: OAuthProvider,
    /// ID único e imutável do usuário no provedor (Google: `sub`, Microsoft: `sub`/`id`, Instagram: `id`)
    pub provider_user_id: String,
    /// E-mail verificado pelo provedor
    pub email: String,
    /// Nome de exibição retornado pelo provedor (opcional)
    pub name: Option<String>,
    /// Primeiro nome sugerido (opcional)
    pub given_name: Option<String>,
    /// Sobrenome sugerido (opcional)
    pub family_name: Option<String>,
    /// URL do avatar / foto de perfil (opcional)
    pub picture: Option<String>,
}

/// Resposta bruta do endpoint de token OAuth2 (`POST /token`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub id_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

/// Resposta bruta do endpoint OpenID Connect userinfo da Google
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoogleUserInfoResponse {
    sub: String,
    email: String,
    #[serde(default)]
    email_verified: Option<bool>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    given_name: Option<String>,
    #[serde(default)]
    family_name: Option<String>,
    #[serde(default)]
    picture: Option<String>,
}

/// Troca o authorization code pelo perfil do usuário na Google via OAuth2 / OpenID Connect
pub async fn exchange_google_authorization_code(
    http_client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<OAuthUserProfile> {
    if client_id.trim().is_empty() || client_secret.trim().is_empty() {
        return Err(AppError::Internal(
            "Credenciais Google OAuth2 não configuradas no ambiente (OAUTH_GOOGLE_CLIENT_ID e OAUTH_GOOGLE_CLIENT_SECRET)".to_string(),
        ));
    }

    // 1. Troca o authorization_code pelo access_token
    let token_params = [
        ("code", code),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
    ];

    let token_resp = http_client
        .post(OAuthProvider::Google.token_url())
        .form(&token_params)
        .send()
        .await
        .map_err(|err| {
            AppError::Internal(format!(
                "Falha na comunicação de rede com o endpoint de token da Google: {err}"
            ))
        })?;

    if !token_resp.status().is_success() {
        let err_body = token_resp.text().await.unwrap_or_default();
        return Err(AppError::Forbidden(format!(
            "A Google rejeitou o código de autorização OAuth2: {err_body}"
        )));
    }

    let token_data: OAuthTokenResponse = token_resp.json().await.map_err(|err| {
        AppError::Internal(format!(
            "Falha ao desserializar resposta de token da Google: {err}"
        ))
    })?;

    // 2. Consulta o perfil verificado via OpenID Connect UserInfo
    let userinfo_resp = http_client
        .get(OAuthProvider::Google.userinfo_url())
        .bearer_auth(&token_data.access_token)
        .send()
        .await
        .map_err(|err| {
            AppError::Internal(format!(
                "Falha na comunicação com o endpoint UserInfo da Google: {err}"
            ))
        })?;

    if !userinfo_resp.status().is_success() {
        let err_body = userinfo_resp.text().await.unwrap_or_default();
        return Err(AppError::Forbidden(format!(
            "Falha ao consultar perfil de usuário na Google: {err_body}"
        )));
    }

    let user_info: GoogleUserInfoResponse = userinfo_resp.json().await.map_err(|err| {
        AppError::Internal(format!(
            "Falha ao desserializar perfil UserInfo da Google: {err}"
        ))
    })?;

    // 3. Validação de Segurança: o e-mail deve ser formalmente verificado pela Google
    if user_info.email_verified != Some(true) {
        return Err(AppError::Forbidden(format!(
            "O e-mail '{}' não foi confirmado na sua conta Google. Confirme o e-mail na Google antes de prosseguir.",
            user_info.email
        )));
    }

    let normalized_email = validate_and_normalize_email(&user_info.email)?;

    // Derivação inteligente de primeiro e último nome caso venha apenas `name`
    let (given_name, family_name) = match (user_info.given_name, user_info.family_name) {
        (Some(g), Some(f)) => (Some(g), Some(f)),
        (Some(g), None) => (Some(g), None),
        (None, Some(f)) => (None, Some(f)),
        (None, None) => {
            if let Some(ref full) = user_info.name {
                let parts: Vec<&str> = full.split_whitespace().collect();
                if parts.len() > 1 {
                    (Some(parts[0].to_string()), Some(parts[1..].join(" ")))
                } else if !parts.is_empty() {
                    (Some(parts[0].to_string()), None)
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        }
    };

    Ok(OAuthUserProfile {
        provider: OAuthProvider::Google,
        provider_user_id: user_info.sub,
        email: normalized_email,
        name: user_info.name,
        given_name,
        family_name,
        picture: user_info.picture,
    })
}
