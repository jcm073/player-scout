use axum::{
    Extension, Json,
    extract::{Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::Database,
    domain::{
        AuthenticatedUser, OAuthProvider, Player, RegistrationInput, SESSION_COOKIE_NAME,
        SESSION_TTL_SECONDS, User, UserOAuthAccount, UserSession, extract_bearer_token,
        extract_session_token_from_cookie,
    },
    error::{AppError, Result},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub user: User,
    pub player_profile: Option<Player>,
    pub session_token: String,
    pub session: UserSession,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutResponse {
    pub revoked: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordResponse {
    pub user: User,
    pub session_token: String,
    pub session: UserSession,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserLookupQuery {
    pub email: Option<String>,
    pub cpf: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAuthorizeResponse {
    pub provider: OAuthProvider,
    pub provider_name: String,
    pub authorize_url: String,
    pub state: String,
    pub configured_in_env: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCallbackRequest {
    pub user_id: Uuid,
    pub provider_user_id: String,
    pub provider_email: String,
}

/// Constrói o cabeçalho `Set-Cookie` seguro (`HttpOnly`, `SameSite=Lax`, `Max-Age=43200` / 12 horas)
pub fn build_session_cookie(raw_token: &str) -> String {
    format!(
        "{SESSION_COOKIE_NAME}={raw_token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_TTL_SECONDS}"
    )
}

/// Constrói o cabeçalho `Set-Cookie` que limpa imediatamente o cookie de sessão no navegador
pub fn build_clear_session_cookie() -> String {
    format!("{SESSION_COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

/// Extrai o token de sessão a partir do header `Authorization: Bearer <token>` ou do cookie `ps_session`
pub fn extract_token_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(auth_val) = headers.get(header::AUTHORIZATION)
        && let Ok(auth_str) = auth_val.to_str()
        && let Some(token) = extract_bearer_token(auth_str)
    {
        return Some(token);
    }

    if let Some(cookie_val) = headers.get(header::COOKIE)
        && let Ok(cookie_str) = cookie_val.to_str()
        && let Some(token) = extract_session_token_from_cookie(cookie_str)
    {
        return Some(token);
    }

    None
}

/// Middleware de Controle de Sessão Obrigatória (12 horas) + Defesa Anti-CSRF (`Sec-Fetch-Site`):
/// - Bloqueia requisições mutáveis (`POST`, `PUT`, `DELETE`, `PATCH`) originadas de `Sec-Fetch-Site: cross-site`.
/// - Garante que qualquer página Web (`/`, `/web/dashboard`, `/web/court`, `/web/partials/*`)
///   e rota protegida da API (`/api/v1/*`) só possa ser acessada caso exista um token de sessão
///   ativo e válido no banco de dados.
/// - Sessões expiradas (> 12h) são fisicamente excluídas do banco (`DELETE`) no ato da verificação.
pub async fn require_session_middleware(
    State(db): State<Database>,
    mut req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let is_web_route = path == "/" || path.starts_with("/web");
    let is_htmx = req.headers().contains_key("hx-request");

    // Defesa Anti-CSRF baseada em Fetch Metadata (`Sec-Fetch-Site: cross-site`)
    let is_state_changing = matches!(
        *req.method(),
        Method::POST | Method::PUT | Method::DELETE | Method::PATCH
    );
    if is_state_changing
        && let Some(sec_site) = req
            .headers()
            .get("sec-fetch-site")
            .and_then(|v| v.to_str().ok())
        && sec_site.eq_ignore_ascii_case("cross-site")
    {
        return AppError::Forbidden(
            "Requisição cross-site bloqueada pela política Anti-CSRF (Sec-Fetch-Site)".to_string(),
        )
        .into_response();
    }

    let Some(raw_token) = extract_token_from_headers(req.headers()) else {
        return unauthorized_session_response(
            is_web_route,
            is_htmx,
            "Sessão não autenticada. Faça login para acessar o sistema.",
        );
    };

    match db.verify_and_get_active_session(&raw_token).await {
        Ok(auth_user) => {
            req.extensions_mut().insert(auth_user);
            let mut resp = next.run(req).await;
            append_security_headers(resp.headers_mut());
            resp
        }
        Err(_) => unauthorized_session_response(
            is_web_route,
            is_htmx,
            "Sessão inválida, expirada (limite de 12h) ou encerrada.",
        ),
    }
}

fn unauthorized_session_response(is_web_route: bool, is_htmx: bool, message: &str) -> Response {
    let clear_cookie = build_clear_session_cookie();
    if is_web_route {
        let mut resp = if is_htmx {
            let mut r = (StatusCode::UNAUTHORIZED, message.to_string()).into_response();
            if let Ok(hv) = HeaderValue::from_str("/web/login?reason=session_required") {
                r.headers_mut().insert("HX-Redirect", hv);
            }
            r
        } else {
            Redirect::to("/web/login?reason=session_required").into_response()
        };
        if let Ok(cookie_hv) = HeaderValue::from_str(&clear_cookie) {
            resp.headers_mut().insert(header::SET_COOKIE, cookie_hv);
        }
        append_security_headers(resp.headers_mut());
        resp
    } else {
        let mut resp = AppError::Unauthorized(message.to_string()).into_response();
        if let Ok(cookie_hv) = HeaderValue::from_str(&clear_cookie) {
            resp.headers_mut().insert(header::SET_COOKIE, cookie_hv);
        }
        append_security_headers(resp.headers_mut());
        resp
    }
}

/// Adiciona cabeçalhos de segurança HTTP (Hardening OWASP + `Cache-Control: no-store` + `Content-Security-Policy`)
pub fn append_security_headers(headers: &mut HeaderMap) {
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline' https://unpkg.com https://cdn.jsdelivr.net; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data:; frame-ancestors 'none'",
        ),
    );
}

/// Endpoint de Cadastro Completo (`POST /api/v1/auth/register`)
/// - Valida todos os campos obrigatórios e bloqueia escalação de privilégio (`role: admin`)
/// - Aplica hash Argon2i na senha (mínimo de 12 caracteres, 1 maiúscula, 1 minúscula, 1 número e 1 especial)
/// - Cria automaticamente o perfil de atleta em `players` quando `role == player`
/// - Já cria uma sessão única autenticada de 12 horas com hash Argon2i
pub async fn register(
    State(db): State<Database>,
    Json(payload): Json<RegistrationInput>,
) -> Result<Response> {
    let (user, player_profile) = db.register_user_with_profile(payload).await?;
    let (session_token, session) = db.create_or_replace_session(user.id).await?;
    let cookie = build_session_cookie(&session_token);

    let mut resp = (
        StatusCode::CREATED,
        Json(AuthResponse {
            user,
            player_profile,
            session_token,
            session,
        }),
    )
        .into_response();

    if let Ok(hv) = HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    append_security_headers(resp.headers_mut());

    Ok(resp)
}

/// Endpoint de Login exclusivo por E-mail (`POST /api/v1/auth/login`)
/// - Verifica proteção contra Força Bruta (bloqueio de 15 min após 5 falhas consecutivas)
/// - Mitiga enumeração por Timing Attack com `verify_dummy_password_argon2i`
/// - Verifica a senha contra o hash Argon2i armazenado no banco de dados
/// - Cria sessão única de 12 horas (revogando qualquer sessão anterior do mesmo usuário)
pub async fn login(
    State(db): State<Database>,
    Json(payload): Json<LoginRequest>,
) -> Result<Response> {
    let (user, player_profile, session_token, session) = db
        .authenticate_by_email(&payload.email, &payload.password)
        .await?;
    let cookie = build_session_cookie(&session_token);

    let mut resp = (
        StatusCode::OK,
        Json(AuthResponse {
            user,
            player_profile,
            session_token,
            session,
        }),
    )
        .into_response();

    if let Ok(hv) = HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    append_security_headers(resp.headers_mut());

    Ok(resp)
}

/// Endpoint de Logoff (`POST /api/v1/auth/logout`)
/// - Exclui fisicamente (`DELETE`) o token da sessão no banco de dados e limpa o cookie
pub async fn logout(State(db): State<Database>, headers: HeaderMap) -> Result<Response> {
    let revoked = if let Some(raw_token) = extract_token_from_headers(&headers) {
        db.revoke_session_by_token(&raw_token).await?
    } else {
        false
    };

    let clear_cookie = build_clear_session_cookie();
    let mut resp = (
        StatusCode::OK,
        Json(LogoutResponse {
            revoked,
            message: "Sessão encerrada e token excluído com sucesso.".to_string(),
        }),
    )
        .into_response();

    if let Ok(hv) = HeaderValue::from_str(&clear_cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    append_security_headers(resp.headers_mut());

    Ok(resp)
}

/// Endpoint de Inspeção da Sessão Atual (`GET /api/v1/auth/session`)
pub async fn session_status(
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<AuthenticatedUser>> {
    Ok(Json(auth_user))
}

/// Endpoint de Troca de Senha com Rotação Imediata de Sessão (`POST /api/v1/auth/password`)
/// - Verifica a `current_password` usando `Argon2i`
/// - Valida e aplica hash `Argon2i` na `new_password`
/// - Exclui a sessão atual e emite um novo token único de 12 horas
pub async fn change_password(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Response> {
    let (user, session_token, session) = db
        .change_user_password(
            auth_user.user.id,
            &payload.current_password,
            &payload.new_password,
        )
        .await?;

    let cookie = build_session_cookie(&session_token);
    let mut resp = (
        StatusCode::OK,
        Json(ChangePasswordResponse {
            user,
            session_token,
            session,
            message: "Senha atualizada com hash Argon2i e sessão rotacionada com sucesso."
                .to_string(),
        }),
    )
        .into_response();

    if let Ok(hv) = HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    append_security_headers(resp.headers_mut());

    Ok(resp)
}

/// Endpoint de Busca de Usuário por Chave Única (`CPF` ou `Email`) (`GET /api/v1/auth/lookup`)
/// - Proteção de Privacidade / LGPD: restrito a `Coach` e `Admin` (bloqueado para `Player`)
pub async fn lookup_user(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Query(query): Query<UserLookupQuery>,
) -> Result<Json<User>> {
    auth_user.require_coach_or_admin()?;
    let user = db
        .find_user_by_key(query.email.as_deref(), query.cpf.as_deref())
        .await?;
    Ok(Json(user))
}

/// Endpoint de Início do Fluxo OAuth2 (`GET /api/v1/auth/oauth/{provider}/authorize`)
/// Suporta `google` (Gmail), `microsoft` (Microsoft Office) e `instagram`
pub async fn oauth_authorize(
    Path(provider): Path<OAuthProvider>,
) -> Result<Json<OAuthAuthorizeResponse>> {
    let env_prefix = match provider {
        OAuthProvider::Google => "OAUTH_GOOGLE",
        OAuthProvider::Microsoft => "OAUTH_MICROSOFT",
        OAuthProvider::Instagram => "OAUTH_INSTAGRAM",
    };

    let client_id_var = format!("{env_prefix}_CLIENT_ID");
    let redirect_var = format!("{env_prefix}_REDIRECT_URI");

    let client_id_env = std::env::var(&client_id_var).ok();
    let configured_in_env = client_id_env
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty());

    let client_id =
        client_id_env.unwrap_or_else(|| format!("pending_{}_client_id", provider.as_str()));
    let redirect_uri = std::env::var(&redirect_var).unwrap_or_else(|_| {
        format!(
            "http://localhost:3000/api/v1/auth/oauth/{}/callback",
            provider.as_str()
        )
    });
    let state = Uuid::now_v7().to_string();
    let authorize_url = provider.authorize_url(&client_id, &redirect_uri, &state);

    Ok(Json(OAuthAuthorizeResponse {
        provider,
        provider_name: provider.display_name().to_string(),
        authorize_url,
        state,
        configured_in_env,
    }))
}

/// Endpoint de Callback / Vínculo OAuth2 (`POST /api/v1/auth/oauth/{provider}/callback`)
pub async fn oauth_callback(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(provider): Path<OAuthProvider>,
    Json(payload): Json<OAuthCallbackRequest>,
) -> Result<(StatusCode, Json<UserOAuthAccount>)> {
    let resolved_user_id = auth_user.resolve_actor_id(Some(payload.user_id))?;
    let account = db
        .link_oauth_account(
            resolved_user_id,
            provider,
            payload.provider_user_id,
            payload.provider_email,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(account)))
}
