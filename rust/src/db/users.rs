use chrono::Utc;
use uuid::Uuid;

use super::Database;
use crate::error::{AppError, Result};

use crate::{
    db::models::{PlayerRecord, UserOAuthRecord, UserRecord, UserSessionRecord},
    domain::{
        ACCOUNT_LOCKOUT_SECONDS, AuthenticatedUser, DomainError, MAX_FAILED_LOGIN_ATTEMPTS,
        OAuthProvider, Player, PreferredSide, RegistrationInput, SESSION_TTL_SECONDS, User,
        UserOAuthAccount, UserRole, UserSession, generate_and_hash_session_token_argon2i,
        hash_password_argon2i, parse_session_token_id, validate_and_normalize_cpf,
        validate_and_normalize_email, verify_dummy_password_argon2i, verify_password_argon2i,
        verify_session_token_argon2i,
    },
};

impl Database {
    pub async fn create_user(
        &self,
        role: UserRole,
        name: String,
        email: String,
        password_hash: String,
    ) -> Result<User> {
        if name.trim().is_empty() {
            return Err(AppError::BadRequest(
                "O nome do usuário não pode ser vazio".to_string(),
            ));
        }
        if email.trim().is_empty() {
            return Err(AppError::BadRequest(
                "O e-mail do usuário não pode ser vazio".to_string(),
            ));
        }

        let mut db = self.inner.clone();
        let rec = toasty::create!(UserRecord {
            id: Uuid::now_v7(),
            role: role,
            name: name,
            first_name: None,
            last_name: None,
            age: None,
            cpf: None,
            email: email.trim().to_lowercase(),
            whatsapp_phone: None,
            cep: None,
            street: None,
            street_number: None,
            complement: None,
            neighborhood: None,
            city: None,
            state: None,
            preferred_side: PreferredSide::Left,
            password_hash: password_hash,
            failed_login_attempts: 0,
            locked_until_epoch: None,
            is_active: true,
        })
        .exec(&mut db)
        .await?;

        Ok(User::from(rec))
    }

    /// Realiza o cadastro completo de Player ou Coach:
    /// - Valida todos os campos obrigatórios (nome, sobrenome, idade, CPF, email, WhatsApp, CEP, endereço, complemento <= 10 chars sem especiais, estado, posição em quadra e senha >= 12 chars)
    /// - Bloqueia auto-cadastro com perfil `Admin`
    /// - Salva a senha com hash Argon2i
    /// - Garante unicidade das chaves CPF e Email
    /// - Se o perfil for `Player`, cria automaticamente o perfil esportivo na tabela `players`
    pub async fn register_user_with_profile(
        &self,
        input: RegistrationInput,
    ) -> Result<(User, Option<Player>)> {
        let validated = input.validate_and_hash()?;
        let mut db = self.inner.clone();

        // Verifica duplicidade de Email ou CPF
        let existing_users = self.list_users().await?;
        if existing_users.iter().any(|u| u.email == validated.email) {
            return Err(AppError::Conflict(format!(
                "Já existe um usuário cadastrado com o e-mail '{}'",
                validated.email
            )));
        }
        if existing_users
            .iter()
            .any(|u| u.cpf.as_deref() == Some(validated.cpf.as_str()))
        {
            return Err(AppError::Conflict(format!(
                "Já existe um usuário cadastrado com o CPF '{}'",
                validated.cpf
            )));
        }

        let complement_opt = if validated.complement.is_empty() {
            None
        } else {
            Some(validated.complement.clone())
        };

        let user_rec = toasty::create!(UserRecord {
            id: Uuid::now_v7(),
            role: validated.role,
            name: validated.full_name.clone(),
            first_name: Some(validated.first_name),
            last_name: Some(validated.last_name),
            age: Some(validated.age),
            cpf: Some(validated.cpf),
            email: validated.email,
            whatsapp_phone: Some(validated.whatsapp_phone),
            cep: Some(validated.cep),
            street: Some(validated.street),
            street_number: Some(validated.street_number),
            complement: complement_opt,
            neighborhood: Some(validated.neighborhood),
            city: Some(validated.city),
            state: Some(validated.state),
            preferred_side: validated.preferred_side,
            password_hash: validated.password_hash,
            failed_login_attempts: 0,
            locked_until_epoch: None,
            is_active: true,
        })
        .exec(&mut db)
        .await?;

        let user = User::from(user_rec);

        // Se o cadastro for do perfil Player, cria automaticamente o perfil em `players`
        let player_profile = if user.role == UserRole::Player {
            let player_rec = toasty::create!(PlayerRecord {
                id: Uuid::now_v7(),
                user_id: Some(user.id),
                coach_id: None,
                name: user.name.clone(),
                dominant_hand: validated.dominant_hand,
                preferred_side: validated.preferred_side,
                category: None,
            })
            .exec(&mut db)
            .await?;

            Some(Player::from(player_rec))
        } else {
            None
        };

        Ok((user, player_profile))
    }

    /// Autentica o usuário exclusivamente por E-mail, aplica proteção contra Força Bruta
    /// (5 tentativas -> 15 min de bloqueio), mitiga Timing Side-Channel com `verify_dummy_password_argon2i`,
    /// verifica a senha contra o hash Argon2i e emite uma sessão única com validade de 12 horas
    /// (revogando qualquer sessão anterior do mesmo usuário).
    pub async fn authenticate_by_email(
        &self,
        email: &str,
        password: &str,
    ) -> Result<(User, Option<Player>, String, UserSession)> {
        let normalized_email = validate_and_normalize_email(email).map_err(|_| {
            verify_dummy_password_argon2i(password);
            AppError::Unauthorized("E-mail ou senha inválidos".to_string())
        })?;

        let mut db = self.inner.clone();
        let all_records = UserRecord::all().exec(&mut db).await?;
        let Some(mut user_rec) = all_records
            .into_iter()
            .find(|u| u.email == normalized_email)
        else {
            verify_dummy_password_argon2i(password);
            return Err(AppError::Unauthorized(
                "E-mail ou senha inválidos".to_string(),
            ));
        };

        // Verifica se a conta está desativada (Soft Delete)
        if !user_rec.is_active {
            return Err(AppError::Forbidden(
                "Conta de usuário desativada. Entre em contato com o suporte ou administrador."
                    .to_string(),
            ));
        }

        let now_epoch = Utc::now().timestamp();
        let locked_until = user_rec.locked_until_epoch.unwrap_or(0);

        // 1. Verifica se a conta está temporariamente bloqueada por força bruta
        if locked_until > now_epoch {
            let remaining_secs = locked_until - now_epoch;
            let remaining_mins = (remaining_secs + 59) / 60;
            return Err(AppError::TooManyRequests(format!(
                "Conta temporariamente bloqueada devido a múltiplas tentativas falhas de login. Tente novamente em {remaining_mins} minuto(s) ({remaining_secs}s)."
            )));
        }

        // Se havia um bloqueio anterior já expirado, zera o contador antes de avaliar a nova tentativa
        if locked_until > 0 && locked_until <= now_epoch {
            user_rec.failed_login_attempts = 0;
            user_rec.locked_until_epoch = None;
        }

        // 2. Verifica a senha usando Argon2i
        if !verify_password_argon2i(password, &user_rec.password_hash) {
            let new_attempts = user_rec.failed_login_attempts.saturating_add(1);
            if new_attempts >= MAX_FAILED_LOGIN_ATTEMPTS {
                let lock_until = now_epoch + ACCOUNT_LOCKOUT_SECONDS;
                user_rec
                    .update()
                    .failed_login_attempts(new_attempts)
                    .locked_until_epoch(Some(lock_until))
                    .exec(&mut db)
                    .await?;
                return Err(AppError::TooManyRequests(format!(
                    "Conta bloqueada por 15 minutos após {MAX_FAILED_LOGIN_ATTEMPTS} tentativas consecutivas de senha inválida."
                )));
            } else {
                user_rec
                    .update()
                    .failed_login_attempts(new_attempts)
                    .locked_until_epoch(None)
                    .exec(&mut db)
                    .await?;
                return Err(AppError::Unauthorized(
                    "E-mail ou senha inválidos".to_string(),
                ));
            }
        }

        // 3. Login bem-sucedido: zera contadores de falha caso existam
        if user_rec.failed_login_attempts > 0 || user_rec.locked_until_epoch.is_some() {
            user_rec
                .update()
                .failed_login_attempts(0)
                .locked_until_epoch(None)
                .exec(&mut db)
                .await?;
        }

        let user = User::from(user_rec);

        let player_profile = if user.role == UserRole::Player {
            let all_players = PlayerRecord::all().exec(&mut db).await?;
            all_players
                .into_iter()
                .find(|p| p.user_id == Some(user.id))
                .map(Player::from)
        } else {
            None
        };

        // 4. Emite sessão única de 12 horas (excluindo eventual sessão anterior do mesmo usuário)
        let (raw_token, session) = self.create_or_replace_session(user.id).await?;

        Ok((user, player_profile, raw_token, session))
    }

    /// Cria uma nova sessão de 12 horas para o usuário, garantindo que NUNCA existam múltiplas
    /// sessões ativas para o mesmo `user_id` (exclui fisicamente qualquer sessão anterior).
    /// O segredo do token é armazenado em formato de hash `Argon2i`.
    pub async fn create_or_replace_session(&self, user_id: Uuid) -> Result<(String, UserSession)> {
        self.revoke_session_by_user_id(user_id).await?;

        let session_id = Uuid::now_v7();
        let (raw_token, token_hash) = generate_and_hash_session_token_argon2i(session_id)?;
        let now_epoch = Utc::now().timestamp();
        let expires_at_epoch = now_epoch + SESSION_TTL_SECONDS;

        let mut db = self.inner.clone();
        let rec = toasty::create!(UserSessionRecord {
            id: session_id,
            user_id: user_id,
            token_hash: token_hash,
            created_at_epoch: now_epoch,
            expires_at_epoch: expires_at_epoch,
        })
        .exec(&mut db)
        .await?;

        Ok((raw_token, UserSession::from(rec)))
    }

    /// Valida o token de sessão enviado pelo cliente:
    /// - Localiza a sessão pelo ID estruturado no token
    /// - Caso a sessão tenha expirado (12 horas), executa `DELETE` imediato no banco e retorna `401 Unauthorized`
    /// - Verifica criptograficamente o segredo contra o hash `Argon2i` armazenado
    pub async fn verify_and_get_active_session(
        &self,
        raw_token: &str,
    ) -> Result<AuthenticatedUser> {
        let session_id = parse_session_token_id(raw_token).ok_or_else(|| {
            AppError::Unauthorized("Token de sessão ausente ou em formato inválido".to_string())
        })?;

        let mut db = self.inner.clone();
        let rec = UserSessionRecord::get_by_id(&mut db, &session_id)
            .await
            .map_err(|_| {
                AppError::Unauthorized("Sessão inexistente, expirada ou já encerrada".to_string())
            })?;

        let now_epoch = Utc::now().timestamp();
        if now_epoch >= rec.expires_at_epoch {
            // Exclui fisicamente do banco de dados sempre que a sessão expirar
            rec.delete().exec(&mut db).await?;
            return Err(AppError::Unauthorized(
                "Sessão expirada após 12 horas. Faça login novamente.".to_string(),
            ));
        }

        if !verify_session_token_argon2i(raw_token, &rec.token_hash) {
            return Err(AppError::Unauthorized(
                "Assinatura Argon2i do token de sessão inválida".to_string(),
            ));
        }

        let session = UserSession::from(rec);
        let user = self.get_user_by_id(session.user_id).await?;

        Ok(AuthenticatedUser { user, session })
    }

    /// Exclui fisicamente a sessão ativa a partir do token (Logoff)
    pub async fn revoke_session_by_token(&self, raw_token: &str) -> Result<bool> {
        let Some(session_id) = parse_session_token_id(raw_token) else {
            return Ok(false);
        };

        let mut db = self.inner.clone();
        let Ok(rec) = UserSessionRecord::get_by_id(&mut db, &session_id).await else {
            return Ok(false);
        };

        if verify_session_token_argon2i(raw_token, &rec.token_hash) {
            rec.delete().exec(&mut db).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Exclui fisicamente qualquer sessão associada ao `user_id` informado (garantia de sessão única)
    pub async fn revoke_session_by_user_id(&self, user_id: Uuid) -> Result<u64> {
        let mut db = self.inner.clone();
        let all_sessions = UserSessionRecord::all().exec(&mut db).await?;
        let mut deleted = 0;
        for sess in all_sessions {
            if sess.user_id == user_id {
                sess.delete().exec(&mut db).await?;
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    /// Troca a senha do usuário autenticado:
    /// - Verifica a `current_password` contra o hash `Argon2i` atual
    /// - Rejeita se a nova senha for idêntica à senha atual
    /// - Valida a complexidade da `new_password` (>= 12 chars, maiúscula, minúscula, número, especial) e gera novo hash `Argon2i`
    /// - Atualiza o registro do usuário e rotaciona imediatamente a sessão única de 12 horas
    pub async fn change_user_password(
        &self,
        user_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<(User, String, UserSession)> {
        let mut db = self.inner.clone();
        let mut user_rec = UserRecord::get_by_id(&mut db, &user_id).await?;

        if !verify_password_argon2i(current_password, &user_rec.password_hash) {
            return Err(AppError::Unauthorized(
                "A senha atual informada está incorreta".to_string(),
            ));
        }

        if current_password == new_password {
            return Err(AppError::Domain(DomainError::InvalidPassword(
                "A nova senha deve ser diferente da senha atual".to_string(),
            )));
        }

        let new_hash = hash_password_argon2i(new_password)?;
        user_rec
            .update()
            .password_hash(new_hash)
            .failed_login_attempts(0)
            .locked_until_epoch(None)
            .exec(&mut db)
            .await?;

        let updated_user = User::from(user_rec);
        let (raw_token, session) = self.create_or_replace_session(user_id).await?;

        Ok((updated_user, raw_token, session))
    }

    /// Atualiza os dados cadastrais e preferências de quadra do usuário:
    /// - Nome e Posição Preferida em quadra (`Direita (Drive)` ou `Esquerda (Revés)`)
    /// - Se o usuário for um `Player`, sincroniza automaticamente seu perfil esportivo em `players`
    pub async fn update_user_profile(
        &self,
        user_id: Uuid,
        name: &str,
        preferred_side: PreferredSide,
        whatsapp_phone: Option<String>,
    ) -> Result<User> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err(AppError::BadRequest(
                "O nome não pode ser vazio".to_string(),
            ));
        }

        let mut db = self.inner.clone();
        let mut user_rec = UserRecord::get_by_id(&mut db, &user_id)
            .await
            .map_err(|_| {
                AppError::NotFound(format!("Usuário com ID '{user_id}' não encontrado"))
            })?;

        user_rec
            .update()
            .name(trimmed_name.to_string())
            .preferred_side(preferred_side)
            .whatsapp_phone(whatsapp_phone)
            .exec(&mut db)
            .await?;

        // Se o usuário possui um perfil esportivo em `players`, sincroniza nome e posição
        let player_records = PlayerRecord::filter(PlayerRecord::fields().user_id().eq(user_id))
            .exec(&mut db)
            .await
            .unwrap_or_default();

        for mut player in player_records {
            let _ = player
                .update()
                .name(trimmed_name.to_string())
                .preferred_side(preferred_side)
                .exec(&mut db)
                .await;
        }

        Ok(User::from(user_rec))
    }

    /// Busca um usuário utilizando CPF ou E-mail como chave única de identificação
    pub async fn find_user_by_key(&self, email: Option<&str>, cpf: Option<&str>) -> Result<User> {
        let users = self.list_users().await?;

        if let Some(raw_cpf) = cpf
            && !raw_cpf.trim().is_empty()
        {
            let normalized_cpf = validate_and_normalize_cpf(raw_cpf)?;
            if let Some(user) = users
                .into_iter()
                .find(|u| u.cpf.as_deref() == Some(normalized_cpf.as_str()))
            {
                return Ok(user);
            }
            return Err(AppError::NotFound(format!(
                "Nenhum usuário encontrado para o CPF informado ({normalized_cpf})"
            )));
        }

        if let Some(raw_email) = email
            && !raw_email.trim().is_empty()
        {
            let normalized_email = validate_and_normalize_email(raw_email)?;
            if let Some(user) = users.into_iter().find(|u| u.email == normalized_email) {
                return Ok(user);
            }
            return Err(AppError::NotFound(format!(
                "Nenhum usuário encontrado para o e-mail '{normalized_email}'"
            )));
        }

        Err(AppError::BadRequest(
            "Informe 'cpf' ou 'email' para localizar o usuário".to_string(),
        ))
    }

    /// Vincula ou retorna uma conta social OAuth2 (Google/Gmail, Microsoft Office ou Instagram)
    pub async fn link_oauth_account(
        &self,
        user_id: Uuid,
        provider: OAuthProvider,
        provider_user_id: String,
        provider_email: String,
    ) -> Result<UserOAuthAccount> {
        let _user = self.get_user_by_id(user_id).await?;
        let mut db = self.inner.clone();

        let existing = UserOAuthRecord::all().exec(&mut db).await?;
        if let Some(found) = existing
            .into_iter()
            .find(|r| r.provider == provider && r.provider_user_id == provider_user_id)
        {
            return Ok(UserOAuthAccount::from(found));
        }

        let rec = toasty::create!(UserOAuthRecord {
            id: Uuid::now_v7(),
            user_id: user_id,
            provider: provider,
            provider_user_id: provider_user_id,
            provider_email: provider_email.trim().to_lowercase(),
        })
        .exec(&mut db)
        .await?;

        Ok(UserOAuthAccount::from(rec))
    }

    /// Autentica um usuário via provedor OAuth2 (`Google`, `Microsoft`, `Instagram`) e emite
    /// uma sessão única de 12 horas com hash `Argon2i`:
    /// 1. Busca vínculo existente em `user_oauth_accounts` por `(provider, provider_user_id)`.
    /// 2. Caso não exista vínculo prévio, localiza o usuário cadastrado pelo `provider_email` normalizado
    ///    e cria automaticamente o vínculo em `user_oauth_accounts`.
    /// 3. Respeita eventual bloqueio ativo de força bruta (`locked_until_epoch > now`).
    /// 4. Zera contadores de falha e emite sessão única de 12 horas (`create_or_replace_session`).
    pub async fn authenticate_with_oauth(
        &self,
        provider: OAuthProvider,
        provider_user_id: String,
        provider_email: String,
    ) -> Result<(User, Option<Player>, UserOAuthAccount, String, UserSession)> {
        let normalized_email = validate_and_normalize_email(&provider_email)?;
        let clean_provider_uid = provider_user_id.trim().to_string();
        if clean_provider_uid.is_empty() {
            return Err(AppError::BadRequest(
                "O identificador do usuário no provedor OAuth2 (provider_user_id) não pode ser vazio"
                    .to_string(),
            ));
        }

        let mut db = self.inner.clone();
        let existing_links = UserOAuthRecord::all().exec(&mut db).await?;
        let matched_link = existing_links
            .into_iter()
            .find(|r| r.provider == provider && r.provider_user_id == clean_provider_uid);

        let (mut user_rec, oauth_account) = if let Some(link_rec) = matched_link {
            let u_rec = UserRecord::get_by_id(&mut db, &link_rec.user_id).await?;
            (u_rec, UserOAuthAccount::from(link_rec))
        } else {
            let all_users = UserRecord::all().exec(&mut db).await?;
            let Some(u_rec) = all_users
                .into_iter()
                .find(|u| u.email.eq_ignore_ascii_case(&normalized_email))
            else {
                return Err(AppError::NotFound(format!(
                    "Nenhuma conta encontrada para o e-mail '{}'. Conclua o cadastro obrigatório (CPF, WhatsApp e endereço) antes de entrar com {}.",
                    normalized_email,
                    provider.display_name()
                )));
            };

            let created_link = toasty::create!(UserOAuthRecord {
                id: Uuid::now_v7(),
                user_id: u_rec.id,
                provider: provider,
                provider_user_id: clean_provider_uid,
                provider_email: normalized_email,
            })
            .exec(&mut db)
            .await?;

            (u_rec, UserOAuthAccount::from(created_link))
        };

        // Verifica se a conta está desativada (Soft Delete)
        if !user_rec.is_active {
            return Err(AppError::Forbidden(
                "Conta de usuário desativada. Entre em contato com o suporte ou administrador."
                    .to_string(),
            ));
        }

        // Verifica se a conta está temporariamente bloqueada por força bruta
        let now_epoch = Utc::now().timestamp();
        let locked_until = user_rec.locked_until_epoch.unwrap_or(0);
        if locked_until > now_epoch {
            let remaining_secs = locked_until - now_epoch;
            let remaining_mins = (remaining_secs + 59) / 60;
            return Err(AppError::TooManyRequests(format!(
                "Conta temporariamente bloqueada devido a múltiplas tentativas falhas de login. Tente novamente em {remaining_mins} minuto(s) ({remaining_secs}s)."
            )));
        }

        if user_rec.failed_login_attempts > 0 || user_rec.locked_until_epoch.is_some() {
            user_rec
                .update()
                .failed_login_attempts(0)
                .locked_until_epoch(None)
                .exec(&mut db)
                .await?;
        }

        let user = User::from(user_rec);
        let player_profile = if user.role == UserRole::Player {
            let all_players = PlayerRecord::all().exec(&mut db).await?;
            all_players
                .into_iter()
                .find(|p| p.user_id == Some(user.id))
                .map(Player::from)
        } else {
            None
        };

        let (raw_token, session) = self.create_or_replace_session(user.id).await?;

        Ok((user, player_profile, oauth_account, raw_token, session))
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<User> {
        let mut db = self.inner.clone();
        let rec = UserRecord::get_by_id(&mut db, &user_id).await?;
        Ok(User::from(rec))
    }

    pub async fn list_users(&self) -> Result<Vec<User>> {
        let mut db = self.inner.clone();
        let records = UserRecord::all()
            .order_by(UserRecord::fields().id().asc())
            .exec(&mut db)
            .await?;

        Ok(records.into_iter().map(User::from).collect())
    }

    /// Desativa logicamente a conta de um usuário (Soft Delete):
    /// - Marca `is_active = false` sem excluir dados físicos, partidas ou estatísticas
    /// - Revoga imediatamente qualquer sessão ativa do usuário
    pub async fn deactivate_user(&self, user_id: Uuid) -> Result<User> {
        let mut db = self.inner.clone();
        let mut user_rec = UserRecord::get_by_id(&mut db, &user_id)
            .await
            .map_err(|_| {
                AppError::NotFound(format!("Usuário com ID '{user_id}' não encontrado"))
            })?;

        user_rec.update().is_active(false).exec(&mut db).await?;

        // Revoga sessões ativas do usuário imediatamente
        let _ = self.revoke_session_by_user_id(user_id).await;

        Ok(User::from(user_rec))
    }

    /// Reativa logicamente a conta de um usuário desativado:
    /// - Marca `is_active = true`, permitindo novos logins
    pub async fn reactivate_user(&self, user_id: Uuid) -> Result<User> {
        let mut db = self.inner.clone();
        let mut user_rec = UserRecord::get_by_id(&mut db, &user_id)
            .await
            .map_err(|_| {
                AppError::NotFound(format!("Usuário com ID '{user_id}' não encontrado"))
            })?;

        user_rec.update().is_active(true).exec(&mut db).await?;

        Ok(User::from(user_rec))
    }

    // =========================================================================
    // Jogadores & Relacionamento N:N (Coach <-> Player) + Controle de Acesso
    // =========================================================================
}
