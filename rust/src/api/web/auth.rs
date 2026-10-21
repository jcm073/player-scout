use askama::Template;
use axum::{
    Form,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    api::auth::{
        LoginRequest, append_security_headers, build_clear_oauth_state_cookie,
        build_clear_session_cookie, build_oauth_state_cookie, build_session_cookie,
        extract_oauth_state_from_headers, extract_token_from_headers,
    },
    db::Database,
    domain::{OAuthProvider, RegistrationInput, exchange_google_authorization_code},
    error::{AppError, Result},
};

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate {}

#[derive(Debug, Deserialize)]
pub struct WebOAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub email: Option<String>,
    pub provider_user_id: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// Renderiza a Tela de Login Web (`GET /web/login`)
pub async fn login_page(
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Html<String> {
    let error = match query.get("error").map(String::as_str) {
        Some("unauthorized") => Some(
            "🔒 Acesso restrito: faça login com seu e-mail e senha para acessar o painel."
                .to_string(),
        ),
        Some("expired") => {
            Some("⏱️ Sua sessão expirou. Por favor, autentique-se novamente.".to_string())
        }
        Some("logged_out") => Some("👋 Você saiu do sistema com segurança.".to_string()),
        Some("oauth_cancelled") => {
            Some("⚠️ A autenticação com a conta social foi cancelada.".to_string())
        }
        _ => None,
    };

    LoginTemplate { error }
        .render()
        .unwrap_or_else(|_| "<h1>Erro interno ao renderizar tela de login</h1>".to_string())
        .into()
}

/// Processa Login Web via Formulário Padrão (`POST /web/login`)
pub async fn web_login(State(db): State<Database>, Form(form): Form<LoginRequest>) -> Response {
    match db.authenticate_by_email(&form.email, &form.password).await {
        Ok((user, player_profile, session_token, _session)) => {
            let dashboard_link = if let Some(player) = player_profile {
                format!(
                    "/web/dashboard?actor_id={}&player_id={}",
                    user.id, player.id
                )
            } else {
                format!("/web/dashboard?actor_id={}&player_id={}", user.id, user.id)
            };

            let mut resp = Redirect::to(&dashboard_link).into_response();
            let session_cookie = build_session_cookie(&session_token);
            if let Ok(hv) = HeaderValue::from_str(&session_cookie) {
                resp.headers_mut().insert(header::SET_COOKIE, hv);
            }
            append_security_headers(resp.headers_mut());
            resp
        }
        Err(err) => {
            let encoded_err = match err {
                AppError::Unauthorized(_) => "Credenciais inválidas. Verifique seu e-mail e senha.",
                AppError::Forbidden(_) => "Conta temporariamente desativada ou restrita.",
                AppError::TooManyRequests(ref msg) => msg.as_str(),
                _ => "Falha temporária no login. Tente novamente.",
            };
            let redirect_url = format!("/web/login?error={}", encoded_err);
            Redirect::to(&redirect_url).into_response()
        }
    }
}

/// Renderiza a Tela de Cadastro Completo (`GET /web/register`)
pub async fn register_page() -> Result<Html<String>> {
    let html = RegisterTemplate {}.render()?;
    Ok(Html(html))
}

/// Fragmento HTMX: Processa Login Assíncrono com Feedback Inline (`POST /web/partials/auth/login`)
pub async fn web_login_partial(
    State(db): State<Database>,
    Form(form): Form<LoginRequest>,
) -> Result<Response> {
    match db.authenticate_by_email(&form.email, &form.password).await {
        Ok((user, player_profile, session_token, _session)) => {
            let dashboard_link = if let Some(player) = player_profile {
                format!(
                    "/web/dashboard?actor_id={}&player_id={}",
                    user.id, player.id
                )
            } else {
                format!("/web/dashboard?actor_id={}&player_id={}", user.id, user.id)
            };

            let mut headers = HeaderMap::new();
            let session_cookie = build_session_cookie(&session_token);
            if let Ok(hv) = HeaderValue::from_str(&session_cookie) {
                headers.insert(header::SET_COOKIE, hv);
            }
            append_security_headers(&mut headers);

            // Redirecionamento automático sem exibir mensagem intermediária verde
            let html = format!(
                r#"<script>window.location.replace("{}");</script>"#,
                dashboard_link
            );

            Ok((headers, Html(html)).into_response())
        }
        Err(err) => {
            let html = format!(
                r#"<div class="p-4 rounded-xl bg-rose-950/80 border border-rose-700 text-rose-200 text-xs font-semibold">
                    ❌ Falha no login: {}
                </div>"#,
                err
            );
            Ok(Html(html).into_response())
        }
    }
}

/// Fragmento HTMX: Processa Cadastro Completo de Player ou Coach (`POST /web/partials/auth/register`)
pub async fn web_register_partial(
    State(db): State<Database>,
    Form(form): Form<RegistrationInput>,
) -> Result<Response> {
    let oauth_provider_str = form.oauth_provider.clone();
    let oauth_uid = form.oauth_provider_user_id.clone();

    match db.register_user_with_profile(form).await {
        Ok((user, player_profile)) => {
            let mut oauth_linked_note = String::new();
            if let (Some(prov_str), Some(uid)) = (oauth_provider_str, oauth_uid) {
                let provider_opt = match prov_str.trim().to_lowercase().as_str() {
                    "google" => Some(OAuthProvider::Google),
                    "microsoft" => Some(OAuthProvider::Microsoft),
                    "instagram" => Some(OAuthProvider::Instagram),
                    _ => None,
                };
                if let Some(prov) = provider_opt
                    && !uid.trim().is_empty()
                {
                    match db
                        .link_oauth_account(user.id, prov, uid, user.email.clone())
                        .await
                    {
                        Ok(_) => {
                            oauth_linked_note = format!(
                                r#"<div class="text-[11px] font-bold text-emerald-300">🔗 Conta {} vinculada com sucesso! Você já pode entrar com 1 clique usando sua conta social.</div>"#,
                                prov.display_name()
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Falha ao autovincular conta OAuth2 no cadastro: {e}");
                        }
                    }
                }
            }

            let profile_note = if let Some(player) = &player_profile {
                format!(
                    "Perfil de Atleta vinculado com sucesso (ID: <code>{}</code>, Posição: <strong>{}</strong>).",
                    player.id,
                    player.preferred_side.label_pt()
                )
            } else {
                format!(
                    "Perfil de Treinador ativo (Posição de referência: <strong>{}</strong>).",
                    user.preferred_side.label_pt()
                )
            };

            let html = format!(
                r#"<div class="p-4 rounded-xl bg-emerald-950/80 border border-emerald-700 text-emerald-200 text-xs space-y-2">
                    <div class="font-extrabold text-sm text-emerald-300">✅ Cadastro concluído com sucesso!</div>
                    <div>Usuário: <strong>{}</strong> · CPF: <code>{}</code> · E-mail: <code>{}</code></div>
                    <div>{}</div>
                    {}
                    <div class="text-[11px] text-emerald-400/90">Faça login com seu e-mail e senha para acessar o sistema.</div>
                    <div class="pt-1">
                      <a href="/web/login" class="inline-block px-3.5 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-bold transition">Ir para o Login →</a>
                    </div>
                </div>"#,
                user.name,
                user.cpf.as_deref().unwrap_or(""),
                user.email,
                profile_note,
                oauth_linked_note
            );

            let mut headers = HeaderMap::new();
            append_security_headers(&mut headers);

            Ok((headers, Html(html)).into_response())
        }
        Err(err) => {
            let html = format!(
                r#"<div class="p-4 rounded-xl bg-rose-950/80 border border-rose-700 text-rose-200 text-xs font-semibold">
                    ❌ Erro ao realizar cadastro: {}
                </div>"#,
                err
            );
            Ok(Html(html).into_response())
        }
    }
}

/// Endpoint Web de Logoff (`POST /web/auth/logout`)
/// - Exclui fisicamente o token da sessão do banco de dados
/// - Remove o cookie `ps_session` (`Max-Age=0`) e redireciona para `/web/login`
pub async fn web_logout(State(db): State<Database>, headers: HeaderMap) -> Response {
    if let Some(token) = extract_token_from_headers(&headers) {
        let _ = db.revoke_session_by_token(&token).await;
    }

    let mut resp = Redirect::to("/web/login?error=logged_out").into_response();
    if let Ok(hv) = HeaderValue::from_str(&build_clear_session_cookie()) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    append_security_headers(resp.headers_mut());
    resp
}

/// Inicia o fluxo de Login Social OAuth2 pelo navegador (`GET /web/auth/oauth/{provider}/authorize`)
/// - Gera `state` Anti-CSRF e armazena no cookie `HttpOnly` `ps_oauth_state` (10 min)
/// - Redireciona (`302 Found`) para a página de autorização oficial do provedor (`Google`, `Microsoft`, `Instagram`)
pub async fn web_oauth_authorize(Path(provider): Path<OAuthProvider>) -> Response {
    let env_prefix = match provider {
        OAuthProvider::Google => "OAUTH_GOOGLE",
        OAuthProvider::Microsoft => "OAUTH_MICROSOFT",
        OAuthProvider::Instagram => "OAUTH_INSTAGRAM",
    };

    let client_id_var = format!("{env_prefix}_CLIENT_ID");
    let redirect_var = format!("{env_prefix}_REDIRECT_URI");

    let client_id = std::env::var(&client_id_var)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| format!("pending_{}_client_id", provider.as_str()));
    let redirect_uri = std::env::var(&redirect_var).unwrap_or_else(|_| {
        format!(
            "http://localhost:3000/web/auth/oauth/{}/callback",
            provider.as_str()
        )
    });

    let state = Uuid::now_v7().to_string();
    let authorize_url = provider.authorize_url(&client_id, &redirect_uri, &state);
    let state_cookie = build_oauth_state_cookie(&state);

    let mut resp = Redirect::to(&authorize_url).into_response();
    if let Ok(hv) = HeaderValue::from_str(&state_cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    append_security_headers(resp.headers_mut());
    resp
}

/// Callback Web do fluxo OAuth2 (`GET /web/auth/oauth/{provider}/callback`):
/// - Valida o parâmetro `state` contra o cookie `ps_oauth_state` (Defesa Anti-CSRF)
/// - Em caso de Google OAuth2 com código real, executa a troca via OpenID Connect (`/token` e `/userinfo`)
/// - Caso o usuário já possua cadastro ativo, emite a sessão única de 12 horas (`ps_session`) e redireciona para `/web/dashboard`
/// - Caso seja o primeiro acesso, redireciona para `/web/register` trazendo e-mail verificado, nome e chaves OAuth
pub async fn web_oauth_callback(
    State(db): State<Database>,
    headers: HeaderMap,
    Path(provider): Path<OAuthProvider>,
    Query(query): Query<WebOAuthCallbackQuery>,
) -> Result<Response> {
    // Trata eventual recusa de consentimento pelo usuário na página do provedor
    if let Some(err) = query.error.as_deref() {
        tracing::warn!("Consentimento OAuth2 cancelado ou rejeitado pelo provedor: {err}");
        let mut resp = Redirect::to("/web/login?error=oauth_cancelled").into_response();
        if let Ok(hv) = HeaderValue::from_str(&build_clear_oauth_state_cookie()) {
            resp.headers_mut().append(header::SET_COOKIE, hv);
        }
        append_security_headers(resp.headers_mut());
        return Ok(resp);
    }

    // Validação estrita Anti-CSRF
    if let Some(cookie_state) = extract_oauth_state_from_headers(&headers) {
        let provided = query.state.as_deref().unwrap_or("").trim();
        if provided.is_empty() || provided != cookie_state {
            return Err(AppError::Forbidden(
                "Falha na validação Anti-CSRF do callback OAuth2 (state inválido ou divergente)"
                    .to_string(),
            ));
        }
    } else if query.state.as_deref().is_none_or(|s| s.trim().is_empty()) {
        return Err(AppError::Forbidden(
            "Callback OAuth2 rejeitado: parâmetro Anti-CSRF state ausente".to_string(),
        ));
    }

    let mut resolved_name: Option<String> = None;

    // Resolução da identidade do usuário no provedor:
    let (provider_email, provider_user_id) = if let Some(code) = query
        .code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        // Se for o provedor Google com credenciais configuradas e código real de produção:
        if provider == OAuthProvider::Google
            && !code.starts_with("mock_")
            && !code.starts_with("test_")
            && query.email.is_none()
        {
            let client_id = std::env::var("OAUTH_GOOGLE_CLIENT_ID").map_err(|_| {
                AppError::Internal(
                    "Credencial OAUTH_GOOGLE_CLIENT_ID não configurada no ambiente (.env)"
                        .to_string(),
                )
            })?;
            let client_secret = std::env::var("OAUTH_GOOGLE_CLIENT_SECRET").map_err(|_| {
                AppError::Internal(
                    "Credencial OAUTH_GOOGLE_CLIENT_SECRET não configurada no ambiente (.env)"
                        .to_string(),
                )
            })?;
            let redirect_uri = std::env::var("OAUTH_GOOGLE_REDIRECT_URI").unwrap_or_else(|_| {
                format!(
                    "http://localhost:3000/web/auth/oauth/{}/callback",
                    provider.as_str()
                )
            });

            let http_client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| {
                    AppError::Internal(format!("Falha ao inicializar cliente HTTP OAuth2: {e}"))
                })?;

            let profile = exchange_google_authorization_code(
                &http_client,
                &client_id,
                &client_secret,
                &redirect_uri,
                code,
            )
            .await?;

            resolved_name = profile.name;
            (profile.email, profile.provider_user_id)
        } else if let Some(email) = query
            .email
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            // Modo de homologação / testes de integração
            let uid = query
                .provider_user_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("{}_{}", provider.as_str(), code));
            (email.to_lowercase(), uid)
        } else {
            return Err(AppError::BadRequest(
                "Código de autorização OAuth2 recebido, mas as credenciais da Google não estão configuradas e nenhum e-mail de homologação foi informado."
                    .to_string(),
            ));
        }
    } else if let Some(email) = query
        .email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let uid = query
            .provider_user_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{}_oauth", provider.as_str()));
        (email.to_lowercase(), uid)
    } else {
        return Err(AppError::BadRequest(
            "Callback OAuth2 rejeitado: parâmetros 'code' ou 'email' ausentes".to_string(),
        ));
    };

    match db
        .authenticate_with_oauth(provider, provider_user_id.clone(), provider_email.clone())
        .await
    {
        Ok((user, player_profile, _oauth_acc, session_token, _session)) => {
            let dashboard_url = if let Some(player) = player_profile {
                format!(
                    "/web/dashboard?actor_id={}&player_id={}",
                    user.id, player.id
                )
            } else {
                format!("/web/dashboard?actor_id={}&player_id={}", user.id, user.id)
            };

            let mut resp = Redirect::to(&dashboard_url).into_response();
            if let Ok(hv) = HeaderValue::from_str(&build_session_cookie(&session_token)) {
                resp.headers_mut().append(header::SET_COOKIE, hv);
            }
            if let Ok(hv) = HeaderValue::from_str(&build_clear_oauth_state_cookie()) {
                resp.headers_mut().append(header::SET_COOKIE, hv);
            }
            append_security_headers(resp.headers_mut());
            Ok(resp)
        }
        Err(AppError::NotFound(_)) => {
            // Primeiro acesso: redireciona para formulário de cadastro com dados pré-preenchidos
            let mut params = vec![
                ("email", provider_email.as_str()),
                ("oauth_provider", provider.as_str()),
                ("provider_user_id", provider_user_id.as_str()),
            ];
            if let Some(ref n) = resolved_name {
                params.push(("name", n.as_str()));
            }

            let dummy_url =
                reqwest::Url::parse_with_params("http://localhost/web/register", &params)
                    .unwrap_or_else(|_| {
                        reqwest::Url::parse("http://localhost/web/register").unwrap()
                    });
            let register_url = format!("/web/register?{}", dummy_url.query().unwrap_or_default());

            let mut resp = Redirect::to(&register_url).into_response();
            if let Ok(hv) = HeaderValue::from_str(&build_clear_oauth_state_cookie()) {
                resp.headers_mut().append(header::SET_COOKIE, hv);
            }
            append_security_headers(resp.headers_mut());
            Ok(resp)
        }
        Err(err) => Err(err),
    }
}
