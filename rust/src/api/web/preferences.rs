use axum::{
    Extension, Form,
    extract::State,
    http::{HeaderMap, HeaderValue, header},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    api::auth::{append_security_headers, build_session_cookie},
    db::Database,
    domain::{AuthenticatedUser, PreferredSide},
    error::Result,
};

#[derive(Debug, Deserialize)]
pub struct UpdateProfileForm {
    pub name: String,
    pub preferred_side: PreferredSide,
    pub whatsapp_phone: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordForm {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

/// Fragmento HTMX: Atualiza Dados Cadastrais / Preferências do Usuário (`POST /web/preferences/profile`)
pub async fn web_update_profile(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Form(form): Form<UpdateProfileForm>,
) -> Result<Html<String>> {
    let clean_whatsapp = form.whatsapp_phone.filter(|s| !s.trim().is_empty());
    match db
        .update_user_profile(
            auth_user.user.id,
            &form.name,
            form.preferred_side,
            clean_whatsapp,
        )
        .await
    {
        Ok(user) => {
            let html = format!(
                r#"<div class="p-3.5 rounded-xl bg-emerald-950/80 border border-emerald-700 text-emerald-200 text-xs flex items-center justify-between">
                    <span>✅ Preferências salvas: <strong>{}</strong> (Posição: <strong>{}</strong>). Recarregando...</span>
                    <button type="button" onclick="window.location.reload()" class="px-2.5 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-bold text-[11px] transition">Recarregar</button>
                </div>
                <script>setTimeout(() => window.location.reload(), 1200);</script>"#,
                user.name,
                user.preferred_side.label_pt()
            );
            Ok(Html(html))
        }
        Err(err) => {
            let html = format!(
                r#"<div class="p-3.5 rounded-xl bg-rose-950/80 border border-rose-700 text-rose-200 text-xs font-semibold">
                    ❌ Erro ao atualizar perfil: {}
                </div>"#,
                err
            );
            Ok(Html(html))
        }
    }
}

/// Fragmento HTMX: Atualiza Senha do Usuário com Rotação de Sessão (`POST /web/preferences/password`)
pub async fn web_update_password(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Form(form): Form<UpdatePasswordForm>,
) -> Result<Response> {
    if form.new_password != form.confirm_password {
        let html = r#"<div class="p-3.5 rounded-xl bg-rose-950/80 border border-rose-700 text-rose-200 text-xs font-semibold">
            ❌ A confirmação da nova senha não confere.
        </div>"#;
        return Ok(Html(html.to_string()).into_response());
    }

    match db
        .change_user_password(
            auth_user.user.id,
            &form.current_password,
            &form.new_password,
        )
        .await
    {
        Ok((_user, new_token, _session)) => {
            let html = r#"<div class="p-3.5 rounded-xl bg-emerald-950/80 border border-emerald-700 text-emerald-200 text-xs font-semibold">
                ✅ Senha alterada com sucesso! Sessão renovada com segurança.
            </div>"#;

            let mut headers = HeaderMap::new();
            if let Ok(cookie_val) = HeaderValue::from_str(&build_session_cookie(&new_token)) {
                headers.insert(header::SET_COOKIE, cookie_val);
            }
            append_security_headers(&mut headers);

            Ok((headers, Html(html.to_string())).into_response())
        }
        Err(err) => {
            let html = format!(
                r#"<div class="p-3.5 rounded-xl bg-rose-950/80 border border-rose-700 text-rose-200 text-xs font-semibold">
                    ❌ Falha ao alterar senha: {}
                </div>"#,
                err
            );
            Ok(Html(html).into_response())
        }
    }
}
