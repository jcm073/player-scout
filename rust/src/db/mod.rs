pub mod models;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    db::models::{
        CoachPlayerRecord, MatchRecord, PlayerRecord, ShotEventRecord, UserOAuthRecord, UserRecord,
        UserSessionRecord,
    },
    domain::{
        ACCOUNT_LOCKOUT_SECONDS, AnalyticsComparisonReport, AnalyticsFilter, AuthenticatedUser,
        CoachPlayer, DecisionQuality, DomainError, DominantHand, MAX_FAILED_LOGIN_ATTEMPTS,
        MatchSession, OAuthProvider, Player, PlayerAnalyticsReport, PreferredSide,
        RegistrationInput, SESSION_TTL_SECONDS, ShotEvent, ShotOutcome, ShotType, SpecialTag, User,
        UserOAuthAccount, UserRole, UserSession, generate_and_hash_session_token_argon2i,
        hash_password_argon2i, parse_session_token_id, validate_and_normalize_cpf,
        validate_and_normalize_email, verify_dummy_password_argon2i, verify_password_argon2i,
        verify_session_token_argon2i,
    },
    error::{AppError, Result},
};

/// Repositório central de persistência sobre Toasty ORM + PostgreSQL 18
#[derive(Clone)]
pub struct Database {
    inner: toasty::Db,
}

impl Database {
    /// Inicializa o pool de conexões do Toasty ORM registrando todos os modelos e enums
    pub async fn connect(database_url: &str) -> Result<Self> {
        let db = toasty::Db::builder()
            .models(toasty::models!(
                UserRecord,
                UserSessionRecord,
                UserOAuthRecord,
                PlayerRecord,
                CoachPlayerRecord,
                MatchRecord,
                ShotEventRecord,
            ))
            .connect(database_url)
            .await?;

        Ok(Self { inner: db })
    }

    pub fn raw(&self) -> toasty::Db {
        self.inner.clone()
    }

    // =========================================================================
    // Usuários (Admin, Coach, Player), Cadastro Completo & Login Argon2i
    // =========================================================================

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

    // =========================================================================
    // Jogadores & Relacionamento N:N (Coach <-> Player) + Controle de Acesso
    // =========================================================================

    /// Verifica se o ator autenticado tem permissão de escrita (Admin ou Coach)
    pub async fn require_write_permission(&self, actor_id: Uuid) -> Result<User> {
        let actor = self.get_user_by_id(actor_id).await?;
        if !actor.role.can_write_shots() {
            return Err(AppError::Forbidden(
                "O perfil de jogador (Player) possui acesso estritamente somente leitura"
                    .to_string(),
            ));
        }
        Ok(actor)
    }

    pub async fn create_player(
        &self,
        actor_id: Uuid,
        user_id: Option<Uuid>,
        name: String,
        dominant_hand: DominantHand,
        preferred_side: PreferredSide,
        category: Option<String>,
    ) -> Result<Player> {
        let actor = self.require_write_permission(actor_id).await?;

        let primary_coach_id = if actor.role.is_coach() {
            Some(actor.id)
        } else {
            None
        };

        // Valida regras de domínio (ex: nome não vazio)
        let validated = Player::new(
            primary_coach_id,
            name,
            dominant_hand,
            preferred_side,
            category,
        )?;

        let mut db = self.inner.clone();
        let rec = toasty::create!(PlayerRecord {
            id: validated.id,
            user_id: user_id,
            coach_id: validated.coach_id,
            name: validated.name,
            dominant_hand: validated.dominant_hand,
            preferred_side: validated.preferred_side,
            category: validated.category,
        })
        .exec(&mut db)
        .await?;

        // Se criado por um Coach, registra automaticamente o vínculo N:N em coach_players
        if let Some(coach_id) = rec.coach_id {
            let _ = toasty::create!(CoachPlayerRecord {
                coach_id: coach_id,
                player_id: rec.id,
            })
            .exec(&mut db)
            .await?;
        }

        Ok(Player::from(rec))
    }

    pub async fn get_player_by_id(&self, player_id: Uuid) -> Result<Player> {
        let mut db = self.inner.clone();
        let rec = PlayerRecord::get_by_id(&mut db, &player_id).await?;
        Ok(Player::from(rec))
    }

    /// Vincula um treinador adicional a um jogador na tabela N:N `coach_players`
    pub async fn assign_coach_to_player(
        &self,
        actor_id: Uuid,
        coach_id: Uuid,
        player_id: Uuid,
    ) -> Result<CoachPlayer> {
        let _actor = self.require_write_permission(actor_id).await?;
        let coach = self.get_user_by_id(coach_id).await?;
        if !coach.role.is_coach() && !coach.role.is_admin() {
            return Err(AppError::BadRequest(
                "O usuário atribuído como treinador deve possuir perfil Coach ou Admin".to_string(),
            ));
        }
        let _player = self.get_player_by_id(player_id).await?;

        let mut db = self.inner.clone();
        let existing = CoachPlayerRecord::filter(
            CoachPlayerRecord::fields()
                .coach_id()
                .eq(coach_id)
                .and(CoachPlayerRecord::fields().player_id().eq(player_id)),
        )
        .first()
        .exec(&mut db)
        .await?;

        if let Some(rec) = existing {
            return Ok(CoachPlayer::from(rec));
        }

        let rec = toasty::create!(CoachPlayerRecord {
            coach_id: coach_id,
            player_id: player_id,
        })
        .exec(&mut db)
        .await?;

        Ok(CoachPlayer::from(rec))
    }

    /// Lista os treinadores associados a um jogador
    pub async fn list_coaches_for_player(&self, player_id: Uuid) -> Result<Vec<User>> {
        let mut db = self.inner.clone();
        let links =
            CoachPlayerRecord::filter(CoachPlayerRecord::fields().player_id().eq(player_id))
                .exec(&mut db)
                .await?;

        if links.is_empty() {
            return Ok(Vec::new());
        }

        let coach_ids: Vec<Uuid> = links.into_iter().map(|l| l.coach_id).collect();
        let coaches = UserRecord::filter(UserRecord::fields().id().in_list(coach_ids))
            .exec(&mut db)
            .await?;

        Ok(coaches.into_iter().map(User::from).collect())
    }

    ///Verifica se um ator (`Admin`, `Coach` ou `Player`) pode visualizar os dados de um jogador
    pub async fn verify_player_read_access(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
    ) -> Result<Player> {
        let actor = self.get_user_by_id(actor_id).await?;
        let player = self.get_player_by_id(player_id).await?;

        match actor.role {
            UserRole::Admin => Ok(player),
            UserRole::Player => {
                if player.user_id == Some(actor.id) {
                    Ok(player)
                } else {
                    Err(AppError::Forbidden(
                        "Um jogador só pode visualizar o seu próprio histórico e estatísticas"
                            .to_string(),
                    ))
                }
            }
            UserRole::Coach => {
                if player.coach_id == Some(actor.id) {
                    return Ok(player);
                }
                let mut db = self.inner.clone();
                let link = CoachPlayerRecord::filter(
                    CoachPlayerRecord::fields()
                        .coach_id()
                        .eq(actor.id)
                        .and(CoachPlayerRecord::fields().player_id().eq(player_id)),
                )
                .first()
                .exec(&mut db)
                .await?;

                if link.is_some() {
                    Ok(player)
                } else {
                    Err(AppError::Forbidden(
                        "Treinador não possui vínculo ativo com este jogador".to_string(),
                    ))
                }
            }
        }
    }

    /// Lista os jogadores visíveis para o usuário autenticado conforme seu perfil:
    /// - `Admin`: todos os jogadores do sistema
    /// - `Coach`: todos os jogadores vinculados em `coach_players`
    /// - `Player`: estritamente o seu próprio registro de atleta
    pub async fn list_players_for_actor(&self, actor_id: Uuid) -> Result<Vec<Player>> {
        let actor = self.get_user_by_id(actor_id).await?;
        let mut db = self.inner.clone();

        match actor.role {
            UserRole::Admin => {
                let records = PlayerRecord::all()
                    .order_by(PlayerRecord::fields().id().asc())
                    .exec(&mut db)
                    .await?;
                Ok(records.into_iter().map(Player::from).collect())
            }
            UserRole::Player => {
                let records = PlayerRecord::filter(PlayerRecord::fields().user_id().eq(actor.id))
                    .exec(&mut db)
                    .await?;
                Ok(records.into_iter().map(Player::from).collect())
            }
            UserRole::Coach => {
                let links =
                    CoachPlayerRecord::filter(CoachPlayerRecord::fields().coach_id().eq(actor.id))
                        .exec(&mut db)
                        .await?;

                if links.is_empty() {
                    return Ok(Vec::new());
                }

                let player_ids: Vec<Uuid> = links.into_iter().map(|l| l.player_id).collect();
                let records = PlayerRecord::filter(PlayerRecord::fields().id().in_list(player_ids))
                    .order_by(PlayerRecord::fields().id().asc())
                    .exec(&mut db)
                    .await?;
                Ok(records.into_iter().map(Player::from).collect())
            }
        }
    }

    // =========================================================================
    // Partidas / Sessões (Matches)
    // =========================================================================

    pub async fn create_match(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
        opponent_name: Option<String>,
        notes: Option<String>,
    ) -> Result<MatchSession> {
        let _actor = self.require_write_permission(actor_id).await?;
        let _player = self.verify_player_read_access(actor_id, player_id).await?;

        let session = MatchSession::new(actor_id, player_id, opponent_name);
        let mut db = self.inner.clone();
        let rec = toasty::create!(MatchRecord {
            id: session.id,
            coach_id: session.coach_id,
            player_id: session.player_id,
            tournament_id: session.tournament_id,
            opponent_name: session.opponent_name,
            status: session.status,
            score: session.score,
            notes: notes,
        })
        .exec(&mut db)
        .await?;

        Ok(MatchSession::from(rec))
    }

    pub async fn get_match_by_id(&self, match_id: Uuid) -> Result<MatchSession> {
        let mut db = self.inner.clone();
        let rec = MatchRecord::get_by_id(&mut db, &match_id).await?;
        Ok(MatchSession::from(rec))
    }
}

/// Parâmetros de entrada para o registro de uma jogada em quadra
#[derive(Debug, Clone)]
pub struct NewShotInput {
    pub player_id: Uuid,
    pub set_number: u8,
    pub shot_type: ShotType,
    pub outcome: ShotOutcome,
    pub decision: Option<DecisionQuality>,
    pub special_tag: Option<SpecialTag>,
}

impl Database {
    // =========================================================================
    // Jogadas em Quadra (Shot Events: Ingestão Rápida & Undo)
    // =========================================================================

    pub async fn record_shot(
        &self,
        actor_id: Uuid,
        match_id: Uuid,
        input: NewShotInput,
    ) -> Result<ShotEvent> {
        let _actor = self.require_write_permission(actor_id).await?;
        let match_session = self.get_match_by_id(match_id).await?;

        if match_session.player_id != input.player_id {
            return Err(AppError::BadRequest(
                "O jogador informado não pertence a esta partida".to_string(),
            ));
        }

        // Valida regras de negócio do Padel (Winner -> Hit, UnforcedError -> Miss, Set 1..=5)
        let validated = ShotEvent::new(
            match_id,
            input.player_id,
            input.set_number,
            input.shot_type,
            input.outcome,
            input.decision,
            input.special_tag,
        )?;

        let mut db = self.inner.clone();
        let rec = toasty::create!(ShotEventRecord {
            id: validated.id,
            match_id: validated.match_id,
            player_id: validated.player_id,
            set_number: validated.set_number,
            shot_type: validated.shot_type,
            outcome: validated.outcome,
            decision: validated.decision,
            special_tag: validated.special_tag,
        })
        .exec(&mut db)
        .await?;

        Ok(ShotEvent::from(rec))
    }

    pub async fn list_shots_by_match(
        &self,
        actor_id: Uuid,
        match_id: Uuid,
    ) -> Result<Vec<ShotEvent>> {
        let match_session = self.get_match_by_id(match_id).await?;
        let _player = self
            .verify_player_read_access(actor_id, match_session.player_id)
            .await?;

        let mut db = self.inner.clone();
        let records = ShotEventRecord::filter(ShotEventRecord::fields().match_id().eq(match_id))
            .order_by(ShotEventRecord::fields().id().asc())
            .exec(&mut db)
            .await?;

        Ok(records.into_iter().map(ShotEvent::from).collect())
    }

    /// Desfaz (remove) a última jogada registrada em uma partida usando ordenação decrescente por UUIDv7
    pub async fn undo_last_shot(&self, actor_id: Uuid, match_id: Uuid) -> Result<ShotEvent> {
        let _actor = self.require_write_permission(actor_id).await?;
        let match_session = self.get_match_by_id(match_id).await?;
        let _player = self
            .verify_player_read_access(actor_id, match_session.player_id)
            .await?;

        let mut db = self.inner.clone();
        let last_shot = ShotEventRecord::filter(ShotEventRecord::fields().match_id().eq(match_id))
            .order_by(ShotEventRecord::fields().id().desc())
            .first()
            .exec(&mut db)
            .await?;

        let Some(rec) = last_shot else {
            return Err(AppError::NotFound(
                "Nenhuma jogada registrada nesta partida para desfazer".to_string(),
            ));
        };

        let undone = ShotEvent::from(rec.clone());
        rec.delete().exec(&mut db).await?;

        Ok(undone)
    }

    // =========================================================================
    // Motor Analítico Recursivo & Modo Comparativo (Fase 3)
    // =========================================================================

    pub async fn query_player_shots(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
    ) -> Result<Vec<ShotEvent>> {
        let _player = self.verify_player_read_access(actor_id, player_id).await?;
        let mut db = self.inner.clone();

        let records = ShotEventRecord::filter(ShotEventRecord::fields().player_id().eq(player_id))
            .order_by(ShotEventRecord::fields().id().asc())
            .exec(&mut db)
            .await?;

        Ok(records.into_iter().map(ShotEvent::from).collect())
    }

    pub async fn compute_player_analytics(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
        filter: AnalyticsFilter,
    ) -> Result<PlayerAnalyticsReport> {
        if let Some(set_num) = filter.set_number
            && !(1..=5).contains(&set_num)
        {
            return Err(AppError::Domain(DomainError::InvalidSetNumber(set_num)));
        }

        let events = self.query_player_shots(actor_id, player_id).await?;
        Ok(PlayerAnalyticsReport::from_events(
            player_id, filter, &events,
        ))
    }

    pub async fn compare_player_analytics(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
        scope_a: AnalyticsFilter,
        scope_b: AnalyticsFilter,
    ) -> Result<AnalyticsComparisonReport> {
        if let Some(set_num) = scope_a.set_number
            && !(1..=5).contains(&set_num)
        {
            return Err(AppError::Domain(DomainError::InvalidSetNumber(set_num)));
        }
        if let Some(set_num) = scope_b.set_number
            && !(1..=5).contains(&set_num)
        {
            return Err(AppError::Domain(DomainError::InvalidSetNumber(set_num)));
        }

        let events = self.query_player_shots(actor_id, player_id).await?;
        let report_a = PlayerAnalyticsReport::from_events(player_id, scope_a, &events);
        let report_b = PlayerAnalyticsReport::from_events(player_id, scope_b, &events);

        Ok(AnalyticsComparisonReport::compare(report_a, report_b))
    }
}
