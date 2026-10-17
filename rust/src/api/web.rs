use askama::Template;
use axum::{
    Extension, Form,
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
    db::{Database, NewShotInput},
    domain::{
        AnalyticsComparisonReport, AnalyticsFilter, AuthenticatedUser, DecisionQuality,
        OAuthProvider, PlayerAnalyticsReport, RegistrationInput, ShotEvent, ShotOutcome, ShotType,
        SpecialTag,
    },
    error::{AppError, Result},
};

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {}

#[derive(Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate {}

#[derive(Template)]
#[template(path = "partials/analytics_panel.html")]
pub struct AnalyticsPanelTemplate {
    pub report: PlayerAnalyticsReport,
}

#[derive(Template)]
#[template(path = "partials/compare_panel.html")]
pub struct ComparePanelTemplate {
    pub label_a: String,
    pub label_b: String,
    pub comparison: AnalyticsComparisonReport,
}

#[derive(Template)]
#[template(path = "partials/court_feed.html")]
pub struct CourtFeedTemplate {
    pub status_banner: Option<String>,
    pub report: PlayerAnalyticsReport,
    pub recent_shots: Vec<ShotEvent>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub actor_id: Uuid,
    pub player_id: Uuid,
    pub match_id: Option<Uuid>,
    pub player_name: String,
    pub analytics_html: String,
    pub compare_html: String,
}

#[derive(Template)]
#[template(path = "court.html")]
pub struct CourtTemplate {
    pub actor_id: Uuid,
    pub player_id: Uuid,
    pub match_id: Uuid,
    pub player_name: String,
    pub serve_shots: &'static [ShotType],
    pub net_shots: &'static [ShotType],
    pub backcourt_shots: &'static [ShotType],
    pub feed_html: String,
}

#[derive(Debug, Deserialize)]
pub struct WebDashboardQuery {
    pub actor_id: Option<Uuid>,
    pub player_id: Option<Uuid>,
    pub match_id: Option<Uuid>,
    pub set_number: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct WebCourtQuery {
    pub actor_id: Option<Uuid>,
    pub player_id: Option<Uuid>,
    pub match_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct WebAnalyticsPartialQuery {
    pub actor_id: Option<Uuid>,
    pub player_id: Uuid,
    pub match_id: Option<Uuid>,
    pub set_number: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct WebComparePartialQuery {
    pub actor_id: Option<Uuid>,
    pub player_id: Uuid,
    pub match_id: Option<Uuid>,
    pub set_a: Option<u8>,
    pub set_b: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct CourtShotForm {
    pub actor_id: Option<Uuid>,
    pub player_id: Uuid,
    pub match_id: Uuid,
    pub set_number: u8,
    pub shot_type: ShotType,
    pub outcome: ShotOutcome,
    pub decision: Option<DecisionQuality>,
    pub special_tag: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CourtUndoForm {
    pub actor_id: Option<Uuid>,
    pub match_id: Uuid,
}

/// Renderiza o Dashboard Analítico Web (`GET /` e `GET /web/dashboard`)
pub async fn dashboard_page(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Query(query): Query<WebDashboardQuery>,
) -> Result<Html<String>> {
    let actor_id = auth_user.resolve_actor_id(query.actor_id)?;

    if let Some(player_id) = query.player_id {
        let player = db.verify_player_read_access(actor_id, player_id).await?;
        let filter = AnalyticsFilter {
            match_id: query.match_id,
            set_number: query.set_number,
            from_date: None,
            to_date: None,
        };
        let report = db
            .compute_player_analytics(actor_id, player_id, filter)
            .await?;

        let analytics_html = AnalyticsPanelTemplate { report }.render()?;

        let compare_html = if let Some(match_id) = query.match_id {
            let scope_a = AnalyticsFilter {
                match_id: Some(match_id),
                set_number: Some(1),
                from_date: None,
                to_date: None,
            };
            let scope_b = AnalyticsFilter {
                match_id: Some(match_id),
                set_number: Some(2),
                from_date: None,
                to_date: None,
            };
            let comparison = db
                .compare_player_analytics(actor_id, player_id, scope_a, scope_b)
                .await?;
            ComparePanelTemplate {
                label_a: "Set 1".to_string(),
                label_b: "Set 2".to_string(),
                comparison,
            }
            .render()?
        } else {
            String::new()
        };

        let html = DashboardTemplate {
            actor_id,
            player_id,
            match_id: query.match_id,
            player_name: player.name,
            analytics_html,
            compare_html,
        }
        .render()?;

        return Ok(Html(html));
    }

    // Modo inicial sem player_id selecionado (exibição de estrutura vazia com o ator autenticado)
    let nil_id = Uuid::nil();
    let empty_report = PlayerAnalyticsReport::from_events(nil_id, AnalyticsFilter::default(), &[]);
    let analytics_html = AnalyticsPanelTemplate {
        report: empty_report,
    }
    .render()?;

    let html = DashboardTemplate {
        actor_id,
        player_id: nil_id,
        match_id: None,
        player_name: "Atleta Demonstração (Informe actor_id e player_id)".to_string(),
        analytics_html,
        compare_html: String::new(),
    }
    .render()?;

    Ok(Html(html))
}

/// Renderiza a Tela Touch-First de Quadra do Treinador (`GET /web/court`)
pub async fn court_page(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Query(query): Query<WebCourtQuery>,
) -> Result<Html<String>> {
    let actor_id = auth_user.resolve_actor_id(query.actor_id)?;

    if let (Some(player_id), Some(match_id)) = (query.player_id, query.match_id) {
        let player = db.verify_player_read_access(actor_id, player_id).await?;
        let feed_html = build_court_feed_html(&db, actor_id, player_id, match_id, None).await?;

        let html = CourtTemplate {
            actor_id,
            player_id,
            match_id,
            player_name: player.name,
            serve_shots: ShotType::serve_shots(),
            net_shots: ShotType::net_shots(),
            backcourt_shots: ShotType::backcourt_shots(),
            feed_html,
        }
        .render()?;

        return Ok(Html(html));
    }

    let nil_id = Uuid::nil();
    let empty_report = PlayerAnalyticsReport::from_events(nil_id, AnalyticsFilter::default(), &[]);
    let feed_html = CourtFeedTemplate {
        status_banner: None,
        report: empty_report,
        recent_shots: Vec::new(),
    }
    .render()?;

    let html = CourtTemplate {
        actor_id,
        player_id: nil_id,
        match_id: nil_id,
        player_name: "Atleta em Quadra".to_string(),
        serve_shots: ShotType::serve_shots(),
        net_shots: ShotType::net_shots(),
        backcourt_shots: ShotType::backcourt_shots(),
        feed_html,
    }
    .render()?;

    Ok(Html(html))
}

/// Fragmento HTMX: Painel Analítico Recursivo (`GET /web/partials/analytics`)
pub async fn analytics_partial(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Query(query): Query<WebAnalyticsPartialQuery>,
) -> Result<Html<String>> {
    let actor_id = auth_user.resolve_actor_id(query.actor_id)?;
    let filter = AnalyticsFilter {
        match_id: query.match_id,
        set_number: query.set_number,
        from_date: None,
        to_date: None,
    };
    let report = db
        .compute_player_analytics(actor_id, query.player_id, filter)
        .await?;

    let html = AnalyticsPanelTemplate { report }.render()?;
    Ok(Html(html))
}

/// Fragmento HTMX: Modo Comparativo Lado a Lado (`GET /web/partials/compare`)
pub async fn compare_partial(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Query(query): Query<WebComparePartialQuery>,
) -> Result<Html<String>> {
    let actor_id = auth_user.resolve_actor_id(query.actor_id)?;
    let set_a = query.set_a.unwrap_or(1);
    let set_b = query.set_b.unwrap_or(2);

    let scope_a = AnalyticsFilter {
        match_id: query.match_id,
        set_number: Some(set_a),
        from_date: None,
        to_date: None,
    };
    let scope_b = AnalyticsFilter {
        match_id: query.match_id,
        set_number: Some(set_b),
        from_date: None,
        to_date: None,
    };

    let comparison = db
        .compare_player_analytics(actor_id, query.player_id, scope_a, scope_b)
        .await?;

    let html = ComparePanelTemplate {
        label_a: format!("Set {set_a}"),
        label_b: format!("Set {set_b}"),
        comparison,
    }
    .render()?;

    Ok(Html(html))
}

/// Fragmento HTMX: Registro Touch-First de Golpe em Quadra (`POST /web/partials/court/shot`)
pub async fn court_record_shot_partial(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Form(form): Form<CourtShotForm>,
) -> Result<Html<String>> {
    let actor_id = auth_user.resolve_actor_id(form.actor_id)?;
    let parsed_special_tag = match form.special_tag.as_deref().map(str::trim) {
        None | Some("") => None,
        Some("winner") => Some(SpecialTag::Winner),
        Some("unforced_error") => Some(SpecialTag::UnforcedError),
        Some(other) => {
            return Err(AppError::BadRequest(format!(
                "Marcador especial inválido: {other}"
            )));
        }
    };

    // Regra de ergonomia Touch-First:
    // - Se o treinador marcou "Erro Não Forçado (ENF)", garante automaticamente `Miss`
    // - Se o treinador marcou "Winner", garante automaticamente `Hit`
    let effective_outcome = match parsed_special_tag {
        Some(SpecialTag::UnforcedError) => ShotOutcome::Miss,
        Some(SpecialTag::Winner) => ShotOutcome::Hit,
        None => form.outcome,
    };

    let recorded = db
        .record_shot(
            actor_id,
            form.match_id,
            NewShotInput {
                player_id: form.player_id,
                set_number: form.set_number,
                shot_type: form.shot_type,
                outcome: effective_outcome,
                decision: form.decision,
                special_tag: parsed_special_tag,
            },
        )
        .await?;

    let outcome_label = match recorded.outcome {
        ShotOutcome::Hit => "ACERTOU",
        ShotOutcome::Miss => "ERROU",
    };
    let banner = format!(
        "Golpe registrado: {} ({}) — Set {}",
        recorded.shot_type.label_pt(),
        outcome_label,
        recorded.set_number
    );

    let html =
        build_court_feed_html(&db, actor_id, form.player_id, form.match_id, Some(banner)).await?;

    Ok(Html(html))
}

/// Fragmento HTMX: Desfazer Última Jogada em Quadra (`POST /web/partials/court/undo`)
pub async fn court_undo_shot_partial(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Form(form): Form<CourtUndoForm>,
) -> Result<Html<String>> {
    let actor_id = auth_user.resolve_actor_id(form.actor_id)?;
    let undone = db.undo_last_shot(actor_id, form.match_id).await?;
    let banner = format!(
        "↩️ Jogada desfeita: {} (Set {})",
        undone.shot_type.label_pt(),
        undone.set_number
    );

    let html =
        build_court_feed_html(&db, actor_id, undone.player_id, form.match_id, Some(banner)).await?;

    Ok(Html(html))
}

async fn build_court_feed_html(
    db: &Database,
    actor_id: Uuid,
    player_id: Uuid,
    match_id: Uuid,
    status_banner: Option<String>,
) -> Result<String> {
    let mut shots = db.list_shots_by_match(actor_id, match_id).await?;
    let filter = AnalyticsFilter {
        match_id: Some(match_id),
        set_number: None,
        from_date: None,
        to_date: None,
    };
    let report = PlayerAnalyticsReport::from_events(player_id, filter, &shots);

    shots.reverse();
    shots.truncate(5);

    let html = CourtFeedTemplate {
        status_banner,
        report,
        recent_shots: shots,
    }
    .render()?;

    Ok(html)
}

/// Renderiza a Tela de Login (`GET /web/login`)
pub async fn login_page() -> Result<Html<String>> {
    let html = LoginTemplate {}.render()?;
    Ok(Html(html))
}

/// Renderiza a Tela de Cadastro Completo (`GET /web/register`)
pub async fn register_page() -> Result<Html<String>> {
    let html = RegisterTemplate {}.render()?;
    Ok(Html(html))
}

/// Fragmento HTMX: Processa Login por E-mail (`POST /web/partials/auth/login`)
/// - Emite o cookie HttpOnly `ps_session` com duração de 12 horas
/// - Substitui e exclui qualquer sessão anterior do mesmo usuário (sessão única)
pub async fn web_login_partial(
    State(db): State<Database>,
    Form(form): Form<LoginRequest>,
) -> Result<Response> {
    match db.authenticate_by_email(&form.email, &form.password).await {
        Ok((user, player_profile, session_token, session)) => {
            let dashboard_link = if let Some(player) = &player_profile {
                format!(
                    "/web/dashboard?actor_id={}&player_id={}",
                    user.id, player.id
                )
            } else {
                format!("/web/dashboard?actor_id={}&player_id={}", user.id, user.id)
            };

            let html = format!(
                r#"<div class="p-4 rounded-xl bg-emerald-950/80 border border-emerald-700 text-emerald-200 text-xs space-y-2">
                    <div class="font-extrabold text-sm text-emerald-300">✅ Login autenticado com sucesso!</div>
                    <div>Bem-vindo(a), <strong>{}</strong> (Perfil: <code>{}</code> · Posição: <code>{}</code>).</div>
                    <div class="text-[11px] text-emerald-400/90">🔒 Sessão única ativa por <strong>12 horas</strong> (expira em <code>{}</code>).</div>
                    <div class="pt-1">
                      <a href="{}" class="inline-block px-3.5 py-2 rounded-lg bg-emerald-600 text-white font-bold">Acessar Dashboard Analítico →</a>
                    </div>
                </div>"#,
                user.name,
                user.role.as_str(),
                user.preferred_side.label_pt(),
                session.expires_at.format("%d/%m/%Y %H:%M UTC"),
                dashboard_link
            );

            let mut headers = HeaderMap::new();
            if let Ok(cookie_val) = HeaderValue::from_str(&build_session_cookie(&session_token)) {
                headers.insert(header::SET_COOKIE, cookie_val);
            }
            append_security_headers(&mut headers);

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
    match db.register_user_with_profile(form).await {
        Ok((user, player_profile)) => {
            let (session_token, _session) = db.create_or_replace_session(user.id).await?;
            let profile_note = if let Some(player) = &player_profile {
                format!(
                    "Perfil de Atleta vinculado automaticamente (ID: <code>{}</code>, Posição: <strong>{}</strong>).",
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
                    <div class="pt-1">
                      <a href="/web/login" class="inline-block px-3.5 py-2 rounded-lg bg-emerald-600 text-white font-bold">Ir para o Login →</a>
                    </div>
                </div>"#,
                user.name,
                user.cpf.as_deref().unwrap_or(""),
                user.email,
                profile_note
            );

            let mut headers = HeaderMap::new();
            if let Ok(cookie_val) = HeaderValue::from_str(&build_session_cookie(&session_token)) {
                headers.insert(header::SET_COOKIE, cookie_val);
            }
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

    let is_htmx = headers
        .get("HX-Request")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));

    let mut resp = if is_htmx {
        let mut r = Html(String::new()).into_response();
        if let Ok(hv) = HeaderValue::from_str("/web/login") {
            r.headers_mut().insert("HX-Redirect", hv);
        }
        r
    } else {
        Redirect::to("/web/login").into_response()
    };

    if let Ok(cookie_val) = HeaderValue::from_str(&build_clear_session_cookie()) {
        resp.headers_mut().insert(header::SET_COOKIE, cookie_val);
    }
    append_security_headers(resp.headers_mut());
    resp
}

#[derive(Debug, Deserialize)]
pub struct WebOAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub email: Option<String>,
    pub provider_user_id: Option<String>,
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
/// - Caso o e-mail já possua cadastro ativo, emite a sessão única de 12 horas (`ps_session`) e redireciona para `/web/dashboard`
/// - Caso o e-mail ainda não possua cadastro completo (CPF/WhatsApp/endereço), redireciona para `/web/register?email=...&oauth_provider=...`
pub async fn web_oauth_callback(
    State(db): State<Database>,
    headers: HeaderMap,
    Path(provider): Path<OAuthProvider>,
    Query(query): Query<WebOAuthCallbackQuery>,
) -> Result<Response> {
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

    let provider_email = query
        .email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            AppError::BadRequest(
                "E-mail do provedor OAuth2 ausente no callback de homologação".to_string(),
            )
        })?
        .to_lowercase();

    let provider_user_id = query
        .provider_user_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            let code_suffix = query.code.as_deref().unwrap_or("oauth");
            format!("{}_{}", provider.as_str(), code_suffix)
        });

    match db
        .authenticate_with_oauth(provider, provider_user_id, provider_email.clone())
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
                resp.headers_mut().insert(header::SET_COOKIE, hv);
            }
            append_security_headers(resp.headers_mut());
            Ok(resp)
        }
        Err(AppError::NotFound(_)) => {
            let register_url = format!(
                "/web/register?email={}&oauth_provider={}",
                provider_email,
                provider.as_str()
            );
            let mut resp = Redirect::to(&register_url).into_response();
            if let Ok(hv) = HeaderValue::from_str(&build_clear_oauth_state_cookie()) {
                resp.headers_mut().insert(header::SET_COOKIE, hv);
            }
            append_security_headers(resp.headers_mut());
            Ok(resp)
        }
        Err(err) => Err(err),
    }
}
