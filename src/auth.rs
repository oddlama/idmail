use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct User {
    /// The username / mailbox address
    pub username: String,
    /// The associated password hash
    pub password: String,
    /// Whether the user is a mailbox
    pub mailbox: bool,
    /// Whether the user is an admin
    pub admin: bool,
    /// Whether the user is active
    pub active: bool,
}

#[cfg(feature = "ssr")]
pub mod ssr {
    pub use super::User;
    use anyhow::{anyhow, Context};
    pub use axum_session_auth::{Authentication, HasPermission, SessionSqlitePool};
    pub use sqlx::SqlitePool;
    pub use std::collections::HashSet;
    pub type AuthSession = axum_session_auth::AuthSession<User, String, SessionSqlitePool, SqlitePool>;
    pub use async_trait::async_trait;
    pub use bcrypt::{hash, verify, DEFAULT_COST};

    impl User {
        pub async fn get(username: &str, pool: &SqlitePool) -> Option<Self> {
            let user = sqlx::query_as::<_, User>(
                "SELECT username, password, FALSE AS is_mailbox, admin, active \
                FROM users WHERE username = $1 \
                UNION SELECT address AS username, password, TRUE AS is_mailbox, FALSE AS admin, active \
                FROM mailboxes WHERE address = $1",
            )
            .bind(username)
            .fetch_one(pool)
            .await
            .ok()?;

            Some(user)
        }
    }

    #[async_trait]
    impl Authentication<User, String, SqlitePool> for User {
        async fn load_user(username: String, pool: Option<&SqlitePool>) -> Result<User, anyhow::Error> {
            let pool = pool.context("Missing sql pool")?;

            User::get(&username, pool)
                .await
                .ok_or_else(|| anyhow!("Cannot get user"))
        }

        fn is_authenticated(&self) -> bool {
            true
        }

        fn is_active(&self) -> bool {
            self.active
        }

        fn is_anonymous(&self) -> bool {
            false
        }
    }

    #[async_trait]
    impl HasPermission<SqlitePool> for User {
        async fn has(&self, _perm: &str, _pool: &Option<&SqlitePool>) -> bool {
            false
        }
    }
}

#[server]
pub async fn get_user() -> Result<Option<User>, ServerFnError> {
    let auth = crate::database::ssr::auth()?;
    Ok(auth.current_user)
}

#[server]
pub async fn login(username: String, password: String, remember: Option<String>) -> Result<(), ServerFnError> {
    let pool = crate::database::ssr::pool()?;
    let auth = crate::database::ssr::auth()?;

    let user = User::get(&username, &pool)
        .await
        .ok_or_else(|| ServerFnError::new("User does not exist."))?;

    match bcrypt::verify(password, &user.password)? {
        true => {
            auth.login_user(user.username);
            auth.remember_user(remember.is_some());
            leptos_axum::redirect("/");
            Ok(())
        }
        false => Err(ServerFnError::ServerError("Password does not match.".to_string())),
    }
}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    let auth = crate::database::ssr::auth()?;
    auth.logout_user();
    leptos_axum::redirect("/");
    Ok(())
}
