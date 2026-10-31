use crate::database::audit::{AuditAction, AuditLogger};
use crate::database::connection::DbConnection;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rusqlite::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub pengguna_id: i64,
    pub nama_lengkap: String,
    pub nama_pengguna: String,
    pub peran: String,
    pub harus_ganti_password: bool,
    pub terakhir_login: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Nama pengguna atau kata sandi tidak valid")]
    InvalidCredentials,
    #[error("Akun pengguna non-aktif")]
    AccountDisabled,
    #[error("Kata sandi lama tidak cocok")]
    OldPasswordMismatch,
    #[error("Kata sandi baru minimal 6 karakter")]
    PasswordTooShort,
    #[error("Gagal melakukan hashing kata sandi")]
    HashError,
}

pub struct AuthService;

impl AuthService {
    /// Hashes a plaintext password using Argon2id
    pub fn hash_password(password: &str) -> std::result::Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| AuthError::HashError)?
            .to_string();
        Ok(hash)
    }

    /// Verifies a password against an Argon2id hash
    pub fn verify_password(password: &str, hash_str: &str) -> bool {
        let parsed_hash = match PasswordHash::new(hash_str) {
            Ok(h) => h,
            Err(_) => return false,
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }

    /// Bootstraps initial developer account securely if no active users exist
    pub fn bootstrap_developer_if_needed(db: &DbConnection) -> Result<()> {
        let mut conn = db.lock().unwrap();
        let user_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM mst_pengguna WHERE deleted_st = 0;",
            [],
            |r| r.get(0),
        )?;

        if user_count == 0 {
            log::info!("Bootstrapping initial developer/admin account");
            let default_password = "developer123";
            let hash = Self::hash_password(default_password)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;

            let now = chrono::Utc::now().to_rfc3339();
            let tx = conn.transaction()?;

            tx.execute(
                "INSERT INTO mst_pengguna (
                    created_at, created_by, nama_lengkap, nama_pengguna, 
                    password_hash, peran, harus_ganti_password, active_st
                ) VALUES (?1, 1, 'Developer Admin', 'developer', ?2, 'ADMIN', 1, 1);",
                rusqlite::params![now, hash],
            )?;

            let new_id = tx.last_insert_rowid().to_string();

            AuditLogger::record(
                &tx,
                "mst_pengguna",
                &new_id,
                AuditAction::Insert,
                &json!({
                    "action": "BOOTSTRAP_DEVELOPER_ACCOUNT",
                    "nama_pengguna": "developer",
                    "peran": "ADMIN",
                    "harus_ganti_password": true
                }),
                Some(1),
            )?;

            tx.commit()?;
            log::info!("Initial developer account created with harus_ganti_password = 1");
        }

        Ok(())
    }

    /// Authenticates a user, updates last_login, records audit log
    pub fn login(
        db: &DbConnection,
        username: &str,
        password: &str,
    ) -> std::result::Result<UserSession, AuthError> {
        let mut conn = db.lock().unwrap();

        let query = "SELECT pengguna_id, nama_lengkap, nama_pengguna, password_hash, peran, harus_ganti_password, terakhir_login, active_st 
                     FROM mst_pengguna 
                     WHERE nama_pengguna = ?1 AND deleted_st = 0;";

        let row = conn.query_row(query, [username], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, i64>(5)? != 0,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, i64>(7)? != 0,
            ))
        });

        let (id, full_name, user_name, hash, role, must_change_pw, last_login, is_active) = match row {
            Ok(data) => data,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Err(AuthError::InvalidCredentials),
            Err(e) => return Err(AuthError::Db(e)),
        };

        if !is_active {
            return Err(AuthError::AccountDisabled);
        }

        if !Self::verify_password(password, &hash) {
            return Err(AuthError::InvalidCredentials);
        }

        // Update last login timestamp in atomic transaction with audit log
        let now = chrono::Utc::now().to_rfc3339();
        let tx = conn.transaction()?;

        tx.execute(
            "UPDATE mst_pengguna SET terakhir_login = ?1, updated_at = ?1 WHERE pengguna_id = ?2;",
            rusqlite::params![now, id],
        )?;

        AuditLogger::record(
            &tx,
            "mst_pengguna",
            &id.to_string(),
            AuditAction::Update,
            &json!({
                "action": "USER_LOGIN_SUCCESS",
                "nama_pengguna": username,
                "waktu": now
            }),
            Some(id),
        )?;

        tx.commit()?;

        Ok(UserSession {
            pengguna_id: id,
            nama_lengkap: full_name,
            nama_pengguna: user_name,
            peran: role,
            harus_ganti_password: must_change_pw,
            terakhir_login: last_login,
        })
    }

    /// Changes password, unsets harus_ganti_password, records audit log
    pub fn change_password(
        db: &DbConnection,
        user_id: i64,
        old_password: &str,
        new_password: &str,
    ) -> std::result::Result<(), AuthError> {
        if new_password.trim().len() < 6 {
            return Err(AuthError::PasswordTooShort);
        }

        let mut conn = db.lock().unwrap();

        let current_hash: String = conn.query_row(
            "SELECT password_hash FROM mst_pengguna WHERE pengguna_id = ?1 AND deleted_st = 0;",
            [user_id],
            |r| r.get(0),
        )?;

        if !Self::verify_password(old_password, &current_hash) {
            return Err(AuthError::OldPasswordMismatch);
        }

        let new_hash = Self::hash_password(new_password)?;
        let now = chrono::Utc::now().to_rfc3339();
        let tx = conn.transaction()?;

        tx.execute(
            "UPDATE mst_pengguna 
             SET password_hash = ?1, harus_ganti_password = 0, updated_at = ?2, updated_by = ?3 
             WHERE pengguna_id = ?3;",
            rusqlite::params![new_hash, now, user_id],
        )?;

        AuditLogger::record(
            &tx,
            "mst_pengguna",
            &user_id.to_string(),
            AuditAction::Update,
            &json!({
                "action": "PASSWORD_CHANGED",
                "user_id": user_id,
                "harus_ganti_password": false
            }),
            Some(user_id),
        )?;

        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::DbManager;
    use crate::database::migrator::Migrator;

    #[test]
    fn test_password_hash_and_verify() {
        let raw = "SuperSecret123!";
        let hash = AuthService::hash_password(raw).expect("Hash failed");
        assert!(AuthService::verify_password(raw, &hash));
        assert!(!AuthService::verify_password("WrongSecret", &hash));
    }

    #[test]
    fn test_developer_bootstrap_and_login() {
        let db_manager = DbManager::init_in_memory().unwrap();
        let conn_arc = db_manager.get_connection();

        // 1. Run migrations first to create mst_pengguna
        {
            let mut conn = conn_arc.lock().unwrap();
            Migrator::run(&mut conn).unwrap();
        }

        // 2. Bootstrap developer
        AuthService::bootstrap_developer_if_needed(&conn_arc).unwrap();

        // 3. Test wrong password
        let wrong_login = AuthService::login(&conn_arc, "developer", "wrong_pass");
        assert!(matches!(wrong_login, Err(AuthError::InvalidCredentials)));

        // 4. Test valid login
        let session = AuthService::login(&conn_arc, "developer", "developer123").unwrap();
        assert_eq!(session.nama_pengguna, "developer");
        assert_eq!(session.peran, "ADMIN");
        assert!(session.harus_ganti_password);

        // 5. Change password
        AuthService::change_password(&conn_arc, session.pengguna_id, "developer123", "NewSecret2026")
            .unwrap();

        // 6. Test login with new password
        let session_new = AuthService::login(&conn_arc, "developer", "NewSecret2026").unwrap();
        assert!(!session_new.harus_ganti_password);
    }
}
