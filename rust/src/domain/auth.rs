use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use chrono::{DateTime, Utc};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DomainError, DominantHand, PreferredSide, UserRole};

/// Provedores OAuth2 suportados pelo Player Scout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
#[serde(rename_all = "snake_case")]
#[column(type = enum("oauth_provider_enum"))]
pub enum OAuthProvider {
    Google,
    Microsoft,
    Instagram,
}

impl OAuthProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            OAuthProvider::Google => "google",
            OAuthProvider::Microsoft => "microsoft",
            OAuthProvider::Instagram => "instagram",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            OAuthProvider::Google => "Google (Gmail)",
            OAuthProvider::Microsoft => "Microsoft Office",
            OAuthProvider::Instagram => "Instagram",
        }
    }

    /// Constrói a URL padrão de autorização OAuth2 para redirecionamento do cliente
    pub fn authorize_url(&self, client_id: &str, redirect_uri: &str, state: &str) -> String {
        match self {
            OAuthProvider::Google => format!(
                "https://accounts.google.com/o/oauth2/v2/auth?client_id={client_id}&redirect_uri={redirect_uri}&response_type=code&scope=openid%20email%20profile&state={state}"
            ),
            OAuthProvider::Microsoft => format!(
                "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?client_id={client_id}&redirect_uri={redirect_uri}&response_type=code&scope=openid%20email%20profile&state={state}"
            ),
            OAuthProvider::Instagram => format!(
                "https://api.instagram.com/oauth/authorize?client_id={client_id}&redirect_uri={redirect_uri}&scope=user_profile&response_type=code&state={state}"
            ),
        }
    }
}

/// Conta OAuth2 vinculada a um usuário (Gmail, Microsoft Office ou Instagram)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOAuthAccount {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: OAuthProvider,
    pub provider_user_id: String,
    pub provider_email: String,
    pub created_at: DateTime<Utc>,
}

/// Dados brutos enviados no cadastro de Player ou Coach
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationInput {
    pub first_name: String,
    pub last_name: String,
    pub age: u8,
    pub cpf: String,
    pub email: String,
    pub whatsapp_phone: String,
    pub cep: String,
    pub street: String,
    pub street_number: String,
    #[serde(default)]
    pub complement: String,
    pub neighborhood: String,
    pub city: String,
    pub state: String,
    pub role: UserRole,
    /// Posição que joga na quadra: Direita (`right`), Esquerda/Revés (`left`) ou Ambos (`both`)
    pub preferred_side: PreferredSide,
    #[serde(default)]
    pub dominant_hand: DominantHand,
    pub password: String,
}

/// Dados normalizados e validados prontos para persistência no banco de dados
#[derive(Debug, Clone)]
pub struct ValidatedRegistration {
    pub full_name: String,
    pub first_name: String,
    pub last_name: String,
    pub age: u8,
    pub cpf: String,
    pub email: String,
    pub whatsapp_phone: String,
    pub cep: String,
    pub street: String,
    pub street_number: String,
    pub complement: String,
    pub neighborhood: String,
    pub city: String,
    pub state: String,
    pub role: UserRole,
    pub preferred_side: PreferredSide,
    pub dominant_hand: DominantHand,
    pub password_hash: String,
}

impl RegistrationInput {
    /// Valida todos os campos obrigatórios, bloqueia auto-cadastro como `Admin` e gera o hash Argon2i da senha
    pub fn validate_and_hash(self) -> Result<ValidatedRegistration, DomainError> {
        if self.role == UserRole::Admin {
            return Err(DomainError::ForbiddenRoleInRegistration);
        }

        let first_name = self.first_name.trim().to_string();
        if first_name.is_empty() {
            return Err(DomainError::MissingRequiredField("nome (first_name)"));
        }

        let last_name = self.last_name.trim().to_string();
        if last_name.is_empty() {
            return Err(DomainError::MissingRequiredField("sobrenome (last_name)"));
        }

        if !(5..=120).contains(&self.age) {
            return Err(DomainError::InvalidAge(self.age));
        }

        let cpf = validate_and_normalize_cpf(&self.cpf)?;
        let email = validate_and_normalize_email(&self.email)?;
        let whatsapp_phone = validate_and_normalize_whatsapp(&self.whatsapp_phone)?;
        let cep = validate_and_normalize_cep(&self.cep)?;

        let street = self.street.trim().to_string();
        if street.is_empty() {
            return Err(DomainError::MissingRequiredField("rua/avenida (street)"));
        }

        let street_number = self.street_number.trim().to_string();
        if street_number.is_empty() {
            return Err(DomainError::MissingRequiredField("número (street_number)"));
        }

        let complement = validate_complement(&self.complement)?;

        let neighborhood = self.neighborhood.trim().to_string();
        if neighborhood.is_empty() {
            return Err(DomainError::MissingRequiredField("bairro (neighborhood)"));
        }

        let city = self.city.trim().to_string();
        if city.is_empty() {
            return Err(DomainError::MissingRequiredField("cidade (city)"));
        }

        let state = validate_and_normalize_state_uf(&self.state)?;
        let password_hash = hash_password_argon2i(&self.password)?;
        let full_name = format!("{first_name} {last_name}");

        Ok(ValidatedRegistration {
            full_name,
            first_name,
            last_name,
            age: self.age,
            cpf,
            email,
            whatsapp_phone,
            cep,
            street,
            street_number,
            complement,
            neighborhood,
            city,
            state,
            role: self.role,
            preferred_side: self.preferred_side,
            dominant_hand: self.dominant_hand,
            password_hash,
        })
    }
}

/// Valida a política de senha:
/// - Mínimo de 12 caracteres
/// - Ao menos 1 letra maiúscula
/// - Ao menos 1 letra minúscula
/// - Ao menos 1 número
/// - Ao menos 1 caractere especial
pub fn validate_password(password: &str) -> Result<(), DomainError> {
    if password.chars().count() < 12 {
        return Err(DomainError::InvalidPassword(
            "A senha deve conter no mínimo 12 caracteres".to_string(),
        ));
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    if !has_uppercase {
        return Err(DomainError::InvalidPassword(
            "A senha deve conter ao menos 1 letra maiúscula".to_string(),
        ));
    }

    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    if !has_lowercase {
        return Err(DomainError::InvalidPassword(
            "A senha deve conter ao menos 1 letra minúscula".to_string(),
        ));
    }

    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    if !has_digit {
        return Err(DomainError::InvalidPassword(
            "A senha deve conter ao menos 1 número".to_string(),
        ));
    }

    let has_special = password
        .chars()
        .any(|c| !c.is_alphanumeric() && !c.is_whitespace());
    if !has_special {
        return Err(DomainError::InvalidPassword(
            "A senha deve conter ao menos 1 caractere especial (ex: !@#$%&*)".to_string(),
        ));
    }

    Ok(())
}

/// Duração padrão do token de sessão após o login: 12 horas (43.200 segundos)
pub const SESSION_TTL_HOURS: i64 = 12;
pub const SESSION_TTL_SECONDS: i64 = SESSION_TTL_HOURS * 3600;

/// Política de proteção contra Força Bruta no Login:
/// Bloqueia temporariamente a conta por 15 minutos após 5 tentativas consecutivas de senha inválida
pub const MAX_FAILED_LOGIN_ATTEMPTS: u8 = 5;
pub const ACCOUNT_LOCKOUT_SECONDS: i64 = 15 * 60;

/// Nome do cookie HttpOnly de sessão utilizado na interface Web/HTMX
pub const SESSION_COOKIE_NAME: &str = "ps_session";

/// Representa a sessão ativa única de um usuário autenticado (validade de 12 horas)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub created_at_epoch: i64,
    pub expires_at_epoch: i64,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub ttl_seconds: i64,
}

impl UserSession {
    pub fn is_expired_at(&self, now_epoch: i64) -> bool {
        now_epoch >= self.expires_at_epoch
    }

    pub fn remaining_seconds(&self, now_epoch: i64) -> i64 {
        (self.expires_at_epoch - now_epoch).max(0)
    }
}

/// Contexto injetado pelo middleware nas requisições autenticadas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub user: super::User,
    pub session: UserSession,
}

impl AuthenticatedUser {
    /// Valida e resolve o `actor_id` da operação (Proteção Anti-Spoofing / BOLA - OWASP API1):
    /// - Usuários `Player` e `Coach` só podem operar com o seu próprio `user.id` da sessão ativa.
    /// - Apenas um `Admin` autenticado pode atuar em nome de outro `actor_id`.
    pub fn resolve_actor_id(&self, requested: Option<Uuid>) -> crate::error::Result<Uuid> {
        match requested {
            Some(req_id) => {
                if req_id == self.user.id || self.user.role.is_admin() {
                    Ok(req_id)
                } else {
                    Err(crate::error::AppError::Forbidden(
                        "Violação de controle de acesso (BOLA): o actor_id informado não corresponde ao usuário autenticado na sessão"
                            .to_string(),
                    ))
                }
            }
            None => Ok(self.user.id),
        }
    }

    /// Exige que o usuário autenticado na sessão possua perfil `Coach` ou `Admin`
    pub fn require_coach_or_admin(&self) -> crate::error::Result<()> {
        if self.user.role.can_write_shots() {
            Ok(())
        } else {
            Err(crate::error::AppError::Forbidden(
                "Acesso restrito a Treinadores (Coach) e Administradores (Admin)".to_string(),
            ))
        }
    }
}

/// Gera o hash da senha utilizando explicitamente o algoritmo `Argon2i` (PHC string format `$argon2i$v=19$...`)
pub fn hash_password_argon2i(password: &str) -> Result<String, DomainError> {
    validate_password(password)?;

    let salt = SaltString::generate(&mut OsRng);
    let argon2i = Argon2::new(Algorithm::Argon2i, Version::V0x13, Params::default());

    argon2i
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| DomainError::InvalidPassword(format!("Falha ao gerar hash Argon2i: {e}")))
}

/// Verifica uma senha em texto plano contra um hash `Argon2i` armazenado no banco
pub fn verify_password_argon2i(password: &str, stored_hash: &str) -> bool {
    if !stored_hash.starts_with("$argon2i$") {
        return false;
    }
    let Ok(parsed_hash) = PasswordHash::new(stored_hash) else {
        return false;
    };
    let argon2i = Argon2::new(Algorithm::Argon2i, Version::V0x13, Params::default());
    argon2i
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// Executa o cálculo `Argon2i` em tempo constante quando um e-mail não é encontrado no banco,
/// prevenindo enumeração de contas por *Timing Side-Channel Attack*.
pub fn verify_dummy_password_argon2i(password: &str) {
    if let Ok(dummy_salt) = SaltString::from_b64("cGxheWVyc2NvdXRkdW1teQ") {
        let argon2i = Argon2::new(Algorithm::Argon2i, Version::V0x13, Params::default());
        let _ = argon2i.hash_password(password.as_bytes(), &dummy_salt);
    }
}

/// Gera um token de sessão opaco de alta entropia vinculado ao `session_id`
/// e calcula o seu hash utilizando a mesma biblioteca `Argon2i` (`$argon2i$v=19$...`)
/// para armazenamento seguro na tabela `user_sessions`.
pub fn generate_and_hash_session_token_argon2i(
    session_id: Uuid,
) -> Result<(String, String), DomainError> {
    let secret_entropy = SaltString::generate(&mut OsRng);
    let raw_token = format!(
        "ps_sess_{}_{}",
        session_id.simple(),
        secret_entropy.as_str()
    );

    let salt = SaltString::generate(&mut OsRng);
    let params = Params::new(4096, 1, 1, None)
        .map_err(|e| DomainError::InvalidPassword(format!("Parâmetros Argon2i inválidos: {e}")))?;
    let argon2i = Argon2::new(Algorithm::Argon2i, Version::V0x13, params);

    let token_hash = argon2i
        .hash_password(raw_token.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| {
            DomainError::InvalidPassword(format!("Falha ao gerar hash Argon2i da sessão: {e}"))
        })?;

    Ok((raw_token, token_hash))
}

/// Extrai o `Uuid` da sessão a partir do prefixo estruturado `ps_sess_<uuid_simple>_<secret>`
pub fn parse_session_token_id(raw_token: &str) -> Option<Uuid> {
    let rest = raw_token.trim().strip_prefix("ps_sess_")?;
    let (uuid_hex, secret) = rest.split_once('_')?;
    if secret.len() < 16 {
        return None;
    }
    Uuid::parse_str(uuid_hex).ok()
}

/// Verifica criptograficamente o token de sessão recebido contra o hash `$argon2i$v=19$...` salvo em `user_sessions`
pub fn verify_session_token_argon2i(raw_token: &str, stored_hash: &str) -> bool {
    if !stored_hash.starts_with("$argon2i$") {
        return false;
    }
    let Ok(parsed_hash) = PasswordHash::new(stored_hash) else {
        return false;
    };
    let argon2i = Argon2::new(Algorithm::Argon2i, Version::V0x13, Params::default());
    argon2i
        .verify_password(raw_token.trim().as_bytes(), &parsed_hash)
        .is_ok()
}

/// Extrai o token `ps_session` de um cabeçalho HTTP `Cookie`
pub fn extract_session_token_from_cookie(cookie_header: &str) -> Option<String> {
    for part in cookie_header.split(';') {
        let trimmed = part.trim();
        if let Some((name, value)) = trimmed.split_once('=')
            && name.trim() == SESSION_COOKIE_NAME
        {
            let token = value.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }
    None
}

/// Extrai o token Bearer de um cabeçalho HTTP `Authorization: Bearer <token>`
pub fn extract_bearer_token(auth_header: &str) -> Option<String> {
    let trimmed = auth_header.trim();
    let token = trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))?
        .trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

/// Valida matematicamente o CPF (11 dígitos e dígitos verificadores Módulo 11) e retorna apenas os 11 dígitos numéricos
pub fn validate_and_normalize_cpf(raw_cpf: &str) -> Result<String, DomainError> {
    let digits: Vec<u32> = raw_cpf
        .chars()
        .filter(|c| !matches!(c, '.' | '-' | ' '))
        .map(|c| c.to_digit(10))
        .collect::<Option<Vec<u32>>>()
        .ok_or_else(|| {
            DomainError::InvalidCpf("O CPF deve conter apenas números (11 dígitos)".to_string())
        })?;

    if digits.len() != 11 {
        return Err(DomainError::InvalidCpf(
            "O CPF deve conter exatamente 11 dígitos".to_string(),
        ));
    }

    // Rejeita sequências de dígitos repetidos (ex: 111.111.111-11)
    if digits.iter().all(|&d| d == digits[0]) {
        return Err(DomainError::InvalidCpf(
            "CPF com todos os dígitos iguais é inválido".to_string(),
        ));
    }

    // Cálculo do 1º dígito verificador
    let sum1: u32 = digits[0..9]
        .iter()
        .enumerate()
        .map(|(i, &d)| d * (10 - i as u32))
        .sum();
    let rem1 = (sum1 * 10) % 11;
    let dv1 = if rem1 == 10 { 0 } else { rem1 };

    if digits[9] != dv1 {
        return Err(DomainError::InvalidCpf(
            "Primeiro dígito verificador do CPF inválido".to_string(),
        ));
    }

    // Cálculo do 2º dígito verificador
    let sum2: u32 = digits[0..10]
        .iter()
        .enumerate()
        .map(|(i, &d)| d * (11 - i as u32))
        .sum();
    let rem2 = (sum2 * 10) % 11;
    let dv2 = if rem2 == 10 { 0 } else { rem2 };

    if digits[10] != dv2 {
        return Err(DomainError::InvalidCpf(
            "Segundo dígito verificador do CPF inválido".to_string(),
        ));
    }

    Ok(digits
        .into_iter()
        .map(|d| char::from_digit(d, 10).unwrap_or('0'))
        .collect())
}

/// Valida e normaliza o endereço de e-mail para minúsculas
pub fn validate_and_normalize_email(raw_email: &str) -> Result<String, DomainError> {
    let email = raw_email.trim().to_lowercase();
    let Some((local, domain)) = email.split_once('@') else {
        return Err(DomainError::InvalidEmail(
            "O e-mail deve conter '@'".to_string(),
        ));
    };

    if local.is_empty() || domain.len() < 3 || !domain.contains('.') || email.contains(' ') {
        return Err(DomainError::InvalidEmail(
            "Formato de e-mail inválido".to_string(),
        ));
    }

    Ok(email)
}

/// Valida e normaliza o telefone de WhatsApp (10 a 13 dígitos numéricos)
pub fn validate_and_normalize_whatsapp(raw_phone: &str) -> Result<String, DomainError> {
    let digits: String = raw_phone
        .chars()
        .filter(|c| !matches!(c, '(' | ')' | '-' | '+' | ' '))
        .collect();

    if !digits.chars().all(|c| c.is_ascii_digit()) || !(10..=13).contains(&digits.len()) {
        return Err(DomainError::InvalidWhatsappPhone(
            "O telefone de WhatsApp deve conter DDD + número (10 a 13 dígitos numéricos)"
                .to_string(),
        ));
    }

    Ok(digits)
}

/// Valida e normaliza o CEP (exatamente 8 dígitos numéricos)
pub fn validate_and_normalize_cep(raw_cep: &str) -> Result<String, DomainError> {
    let digits: String = raw_cep
        .chars()
        .filter(|c| !matches!(c, '-' | '.' | ' '))
        .collect();

    if digits.len() != 8 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(DomainError::InvalidCep(
            "O CEP deve conter exatamente 8 dígitos numéricos".to_string(),
        ));
    }

    Ok(digits)
}

/// Valida o complemento de endereço:
/// - Pode ser vazio ("") para casas sem bloco/apto
/// - Limitado a no máximo 10 caracteres
/// - Apenas caracteres NÃO especiais (letras, números e espaços)
pub fn validate_complement(raw_complement: &str) -> Result<String, DomainError> {
    let trimmed = raw_complement.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    if trimmed.chars().count() > 10 {
        return Err(DomainError::InvalidComplement(
            "O complemento deve ter no máximo 10 caracteres".to_string(),
        ));
    }

    if !trimmed.chars().all(|c| c.is_alphanumeric() || c == ' ') {
        return Err(DomainError::InvalidComplement(
            "O complemento deve conter apenas letras, números e espaços (sem caracteres especiais)"
                .to_string(),
        ));
    }

    Ok(trimmed.to_string())
}

const VALID_BRAZIL_UFS: [&str; 27] = [
    "AC", "AL", "AP", "AM", "BA", "CE", "DF", "ES", "GO", "MA", "MT", "MS", "MG", "PA", "PB", "PR",
    "PE", "PI", "RJ", "RN", "RS", "RO", "RR", "SC", "SP", "SE", "TO",
];

/// Valida e normaliza a sigla do Estado (UF brasileira de 2 letras)
pub fn validate_and_normalize_state_uf(raw_uf: &str) -> Result<String, DomainError> {
    let uf = raw_uf.trim().to_uppercase();
    if !VALID_BRAZIL_UFS.contains(&uf.as_str()) {
        return Err(DomainError::InvalidStateUf(format!(
            "Sigla de estado '{raw_uf}' inválida. Informe uma UF válida (ex: SP, RS, SC, PR, RJ)"
        )));
    }
    Ok(uf)
}
