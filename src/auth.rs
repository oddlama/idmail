use leptos::*;
use leptos_icons::Icon;
use leptos_router::{ActionForm, Redirect};
use leptos_use::ColorMode;
use serde::{Deserialize, Serialize};

use crate::utils::ColorModeToggle;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct User {
    /// The username / mailbox address
    pub username: String,
    /// The associated password hash
    pub password_hash: String,
    /// The owner of this mailbox, or None if this is not a mailbox account
    pub mailbox_owner: Option<String>,
    /// Whether the user is an admin
    pub admin: bool,
    /// Whether the user is active
    pub active: bool,
}

#[cfg(feature = "ssr")]
pub mod ssr {
    pub use super::User;
    use anyhow::{anyhow, Context};
    pub use axum_session_auth::{Authentication, HasPermission};
    pub use axum_session_sqlx::SessionSqlitePool;
    pub use sqlx::SqlitePool;
    pub use std::collections::HashSet;
    pub type AuthSession = axum_session_auth::AuthSession<User, String, SessionSqlitePool, SqlitePool>;
    pub use async_trait::async_trait;

    impl User {
        pub async fn get(username: &str, pool: &SqlitePool) -> Option<Self> {
            let user = sqlx::query_as::<_, User>(
                "SELECT username, password_hash, NULL AS mailbox_owner, admin, active \
                FROM users WHERE username = $1 \
                UNION SELECT address AS username, password_hash, owner AS mailbox_owner, FALSE AS admin, active \
                FROM mailboxes WHERE address = $1",
            )
            .bind(username)
            .fetch_one(pool)
            .await
            .ok()?;

            Some(user)
        }

        pub async fn get_by_api_token(api_token: &str, pool: &SqlitePool) -> Option<Self> {
            if api_token.len() < 16 {
                // Disregard insecure API tokens directly
                return None;
            }

            let user = sqlx::query_as::<_, User>(
                "SELECT address AS username, password_hash, owner AS mailbox_owner, FALSE AS admin, active \
                FROM mailboxes WHERE api_token = $1",
            )
            .bind(api_token)
            .fetch_one(pool)
            .await
            .ok()?;

            if !user.active {
                log::warn!(
                    "denying successful api token because user '{}' is inactive",
                    user.username
                );
                return None;
            }

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

/// Get the current user and ensure that it is an admin
#[server]
pub async fn auth_admin() -> Result<User, ServerFnError> {
    let user = get_user().await?.ok_or_else(|| ServerFnError::new("Unauthorized"))?;
    if !user.admin {
        return Err(ServerFnError::new("Unauthorized"));
    }

    Ok(user)
}

/// Get the current user and ensure that it is not a mailbox
#[server]
pub async fn auth_user() -> Result<User, ServerFnError> {
    let user = get_user().await?.ok_or_else(|| ServerFnError::new("Unauthorized"))?;
    if user.mailbox_owner.is_some() {
        return Err(ServerFnError::new("Unauthorized"));
    }

    Ok(user)
}

/// Get the current user, regardless of whether it is an admin, user or mailbox
#[server]
pub async fn auth_any() -> Result<User, ServerFnError> {
    get_user().await?.ok_or_else(|| ServerFnError::new("Unauthorized"))
}

#[server]
pub async fn authenticate_user(username: String, password: String) -> Result<User, ServerFnError> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    // A generic error message to not leak information to the clients
    let generic_err = || ServerFnError::new("Wrong password or invalid user.");

    let pool = crate::database::ssr::pool()?;
    let user = User::get(&username, &pool).await.ok_or_else(generic_err)?;

    let verify_result = PasswordHash::new(&user.password_hash)
        .and_then(|hash| Argon2::default().verify_password(password.as_bytes(), &hash));
    if verify_result.is_ok() {
        if !user.active {
            log::warn!("denying successful login attempt because user '{username}' is inactive");
            return Err(generic_err());
        }

        log::info!("login successful for user '{username}'");
        Ok(user)
    } else {
        log::warn!(
            "failed authentication of user '{username}': {}",
            verify_result.unwrap_err()
        );
        Err(generic_err())
    }
}

#[server]
pub async fn login(username: String, password: String) -> Result<(), ServerFnError> {
    let user = authenticate_user(username.clone(), password.clone()).await?;
    let auth = crate::database::ssr::auth()?;

    auth.login_user(user.username);
    auth.remember_user(false);
    leptos_axum::redirect("/");
    Ok(())
}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    let auth = crate::database::ssr::auth()?;
    auth.logout_user();
    leptos_axum::redirect("/");
    Ok(())
}

#[component]
pub fn Login(
    action: Action<Login, Result<(), ServerFnError>>,
    color_mode: Signal<ColorMode>,
    set_color_mode: WriteSignal<ColorMode>,
) -> impl IntoView {
    let action_value = Signal::derive(move || action.value().get().unwrap_or(Ok(())));

    view! {
        <div class="relative flex min-h-screen flex-col">
            <div class="absolute top-4 right-4">
                <ColorModeToggle color_mode set_color_mode/>
            </div>
            <div class="w-full h-screen flex items-center justify-center px-4">

                <div class="flex flex-col mx-auto">
                    <div class="mx-auto mb-4 flex flex-row items-center">
                        <img class="w-16 h-16 me-2" src="/logo.svg"/>
                        <h2 class="text-4xl leading-none font-bold inline-block">idmail</h2>
                    </div>
                    <ActionForm
                        action
                        class="rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 text-card-foreground max-w-sm"
                    >
                        <div class="flex flex-col space-y-1.5 p-6">
                            <h2 class="font-semibold tracking-tight text-2xl mb-2">Login</h2>
                            <p class="text-sm text-gray-500 dark:text-gray-400">
                                "Enter your mailbox address and password below to login"
                            </p>
                        </div>
                        <div class="p-6 pt-0">
                            <div class="grid gap-4">
                                <div class="grid gap-2">
                                    <label
                                        class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                                        for="username"
                                    >
                                        Email
                                    </label>
                                    <input
                                        class="flex flex-none w-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                                        type="text"
                                        name="username"
                                        placeholder="username@example.com"
                                        required="required"
                                    />
                                </div>
                                <div class="grid gap-2">
                                    <div class="flex items-center">
                                        <label
                                            class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                                            for="password"
                                        >
                                            Password
                                        </label>
                                    </div>
                                    <input
                                        class="flex flex-none w-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                                        type="password"
                                        name="password"
                                        required="required"
                                    />
                                </div>
                                <ErrorBoundary fallback=|errors| {
                                    view! {
                                        <div class="rounded-lg p-4 flex bg-red-100 dark:bg-red-800">
                                            <div>
                                                <Icon
                                                    icon=icondata::BiXCircleSolid
                                                    class="w-5 h-5 text-red-400 dark:text-red-300"
                                                />
                                            </div>
                                            <div class="ml-3 text-red-700 dark:text-red-300">
                                                <p>
                                                    {move || {
                                                        errors
                                                            .get()
                                                            .into_iter()
                                                            .map(|(_, e)| view! { {e.to_string()} })
                                                            .collect_view()
                                                    }}

                                                </p>
                                            </div>
                                        </div>
                                    }
                                }>

                                    {action_value}
                                </ErrorBoundary>
                                <button
                                    type="submit"
                                    tabindex="0"
                                    class="inline-flex w-full justify-center mt-3 items-center rounded-lg transition-all p-2.5 bg-blue-600 dark:bg-blue-600 hover:bg-blue-500 dark:hover:bg-blue-500 font-semibold text-white focus:ring-4 focus:ring-blue-300 dark:focus:ring-blue-900 sm:w-auto"
                                >
                                    Login
                                </button>
                            </div>
                        </div>
                    </ActionForm>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn LoginView(
    login: Action<Login, Result<(), ServerFnError>>,
    logout: Action<Logout, Result<(), ServerFnError>>,
    color_mode: Signal<ColorMode>,
    set_color_mode: WriteSignal<ColorMode>,
) -> impl IntoView {
    let user = create_resource(
        move || (login.version().get(), logout.version().get()),
        move |_| get_user(),
    );

    view! {
        <Transition fallback=move || {
            view! { <span class="text-gray-300 dark:text-gray-600">"Loading..."</span> }
        }>
            {move || {
                user.get()
                    .map(|user| match user {
                        Err(e) => {
                            view! {
                                <div class="absolute">
                                    <span>{format!("Login error: {}", e)}</span>
                                </div>
                                <Login action=login color_mode set_color_mode/>
                            }
                                .into_view()
                        }
                        Ok(None) => view! { <Login action=login color_mode set_color_mode/> }.into_view(),
                        Ok(Some(_)) => view! { <Redirect path="/aliases"/> }.into_view(),
                    })
            }}

        </Transition>
    }
}
