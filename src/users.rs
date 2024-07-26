use std::collections::VecDeque;
use std::ops::Range;

use crate::utils::{DeleteModal, EditModal, Modal};
use crate::utils::{SliderRenderer, THeadCellRenderer, TailwindClassesPreset, TimediffRenderer};

use chrono::{DateTime, Utc};
use leptos::html::Dialog;
use leptos::{ev::MouseEvent, logging::error, *};
use leptos_icons::Icon;
use leptos_struct_table::*;
use leptos_use::{use_debounce_fn_with_arg, use_timeout_fn};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use sqlx::QueryBuilder;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TableRow)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[table(sortable, classes_provider = TailwindClassesPreset, thead_cell_renderer = THeadCellRenderer)]
pub struct User {
    pub username: String,
    #[table(skip)]
    pub password_hash: String,
    #[table(class = "w-1", renderer = "SliderRenderer")]
    pub admin: bool,
    #[table(class = "w-1", renderer = "SliderRenderer")]
    pub active: bool,
    #[table(class = "w-1", title = "Created", renderer = "TimediffRenderer")]
    pub created_at: DateTime<Utc>,
}

pub(crate) fn is_valid_pw(password: &str) -> bool {
    (12..=1024).contains(&password.len())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserQuery {
    #[serde(default)]
    sort: VecDeque<(usize, ColumnSort)>,
    range: Range<usize>,
    search: String,
}

#[server]
pub async fn list_users(query: UserQuery) -> Result<Vec<User>, ServerFnError> {
    let _user = crate::auth::auth_admin().await?;
    let UserQuery { sort, range, search } = query;

    let mut query = QueryBuilder::new("SELECT * FROM users");
    if !search.is_empty() {
        query.push(" WHERE username LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%')");
    }

    if let Some(order) = User::sorting_to_sql(&sort) {
        query.push(" ");
        query.push(order);
    }

    query.push(" LIMIT ");
    query.push_bind(range.len() as i64);
    query.push(" OFFSET ");
    query.push_bind(range.start as i64);

    let pool = crate::database::ssr::pool()?;
    Ok(query.build_query_as::<User>().fetch_all(&pool).await?)
}

#[server]
pub async fn regenerate_api_key() -> Result<String, ServerFnError> {
    let user = crate::auth::auth_any().await?;
    if user.mailbox_owner.is_none() {
        return Err(ServerFnError::new("Must be a mailbox user."));
    }

    let mut buf = [0u8; 24];
    getrandom::getrandom(&mut buf)?;
    let api_token = hex::encode(buf);

    let mut query = QueryBuilder::new("UPDATE mailboxes SET api_token = ");
    query.push_bind(&api_token);
    query.push(" WHERE address = ");
    query.push_bind(&user.username);

    let pool = crate::database::ssr::pool()?;
    query.build().execute(&pool).await.map(|_| ())?;

    Ok(api_token)
}

#[server]
pub async fn admin_count() -> Result<usize, ServerFnError> {
    let _user = crate::auth::auth_admin().await?;
    let mut query = QueryBuilder::new("SELECT COUNT(*) FROM users WHERE admin = TRUE");

    let pool = crate::database::ssr::pool()?;
    let count = query.build_query_scalar::<i64>().fetch_one(&pool).await?;

    Ok(count as usize)
}

#[server]
pub async fn user_count() -> Result<usize, ServerFnError> {
    let _user = crate::auth::auth_admin().await?;
    let mut query = QueryBuilder::new("SELECT COUNT(*) FROM users");

    let pool = crate::database::ssr::pool()?;
    let count = query.build_query_scalar::<i64>().fetch_one(&pool).await?;

    Ok(count as usize)
}

#[server]
pub async fn delete_user(username: String) -> Result<(), ServerFnError> {
    let _user = crate::auth::auth_admin().await?;

    // Force user reload on next request
    let auth = crate::database::ssr::auth()?;
    auth.cache_clear_user(username.clone());

    let mut query = QueryBuilder::new("DELETE FROM users WHERE username = ");
    query.push_bind(username);

    let pool = crate::database::ssr::pool()?;
    query.build().execute(&pool).await.map(|_| ())?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub fn mk_password_hash(password: &str) -> Result<String, ServerFnError> {
    if !is_valid_pw(password) {
        return Err(ServerFnError::new("Password is invalid."));
    }

    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();
    // Hash password to PHC string ($argon2id$v=19$...)
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .to_string();

    Ok(password_hash)
}

#[server]
pub async fn change_password(current_password: String, new_password: String) -> Result<(), ServerFnError> {
    let user = crate::auth::auth_any().await?;

    // Reauthenticate
    let _ = crate::auth::authenticate_user(user.username.clone(), current_password.clone()).await?;
    let password_hash = mk_password_hash(&new_password)?;

    // Force user reload on next request
    let auth = crate::database::ssr::auth()?;
    auth.cache_clear_user(user.username.clone());

    let mut query = QueryBuilder::new("UPDATE users SET password_hash = ");
    query.push_bind(password_hash);
    query.push(" WHERE username = ");
    query.push_bind(&user.username);

    let pool = crate::database::ssr::pool()?;
    query.build().execute(&pool).await.map(|_| ())?;

    Ok(())
}

#[server]
pub async fn create_or_update_user(
    old_username: Option<String>,
    username: String,
    password: String,
    admin: bool,
    active: bool,
) -> Result<(), ServerFnError> {
    let _user = crate::auth::auth_admin().await?;
    let pool = crate::database::ssr::pool()?;

    if let Some(old_username) = old_username {
        // Force user reload on next request
        let auth = crate::database::ssr::auth()?;
        auth.cache_clear_user(username.clone());

        let mut query = QueryBuilder::new("UPDATE users SET admin = ");
        query.push_bind(admin);
        if !password.is_empty() {
            let password_hash = mk_password_hash(&password)?;
            query.push(", password_hash = ");
            query.push_bind(password_hash);
        }
        query.push(", active = ");
        query.push_bind(active);
        query.push(" WHERE username = ");
        query.push_bind(old_username);

        query.build().execute(&pool).await.map(|_| ())?;
    } else {
        let password_hash = mk_password_hash(&password)?;
        sqlx::query("INSERT INTO users (username, password_hash, admin, active) VALUES (?, ?, ?, ?)")
            .bind(username)
            .bind(password_hash)
            .bind(admin)
            .bind(active)
            .execute(&pool)
            .await
            .map(|_| ())?;
    }

    Ok(())
}

#[server]
pub async fn update_user_admin_or_active(username: String, admin: bool, active: bool) -> Result<(), ServerFnError> {
    let _user = crate::auth::auth_admin().await?;
    let mut query = QueryBuilder::new("UPDATE users SET admin = ");
    query.push_bind(admin);
    query.push(", active = ");
    query.push_bind(active);
    query.push(" WHERE username = ");
    query.push_bind(username);

    let pool = crate::database::ssr::pool()?;
    query.build().execute(&pool).await.map(|_| ())?;
    Ok(())
}

#[derive(Default)]
pub struct UserTableDataProvider {
    sort: VecDeque<(usize, ColumnSort)>,
    pub search: RwSignal<String>,
}

impl TableDataProvider<User> for UserTableDataProvider {
    async fn get_rows(&self, range: Range<usize>) -> Result<(Vec<User>, Range<usize>), String> {
        list_users(UserQuery {
            search: self.search.get_untracked().trim().to_string(),
            sort: self.sort.clone(),
            range: range.clone(),
        })
        .await
        .map(|rows| {
            let len = rows.len();
            (rows, range.start..range.start + len)
        })
        .map_err(|e| format!("{e:?}"))
    }

    async fn row_count(&self) -> Option<usize> {
        user_count().await.ok()
    }

    fn set_sorting(&mut self, sorting: &VecDeque<(usize, ColumnSort)>) {
        self.sort = sorting.clone();
    }

    fn track(&self) {
        self.search.track();
    }
}

#[component]
pub fn Users() -> impl IntoView {
    let mut rows = UserTableDataProvider::default();
    let default_sorting = VecDeque::from([(3, ColumnSort::Descending)]);
    rows.set_sorting(&default_sorting);
    let sorting = create_rw_signal(default_sorting);

    let reload_controller = ReloadController::default();
    let on_input = use_debounce_fn_with_arg(move |value| rows.search.set(value), 300.0);
    let (count, set_count) = create_signal(0);

    let delete_modal_user = create_rw_signal(None);
    let edit_modal_user = create_rw_signal(None);

    let (edit_modal_input_username, set_edit_modal_input_username) = create_signal("".to_string());
    let (edit_modal_input_password, set_edit_modal_input_password) = create_signal("".to_string());
    let (edit_modal_input_password_repeat, set_edit_modal_input_password_repeat) = create_signal("".to_string());
    let (edit_modal_input_admin, set_edit_modal_input_admin) = create_signal(false);
    let (edit_modal_input_active, set_edit_modal_input_active) = create_signal(true);
    let edit_modal_open_with = Callback::new(move |edit_user: Option<User>| {
        edit_modal_user.set(Some(edit_user.clone()));
        set_edit_modal_input_password("".to_string());
        set_edit_modal_input_password_repeat("".to_string());

        if let Some(edit_user) = edit_user {
            set_edit_modal_input_username(edit_user.username.clone());
            set_edit_modal_input_admin(edit_user.admin);
            set_edit_modal_input_active(edit_user.active);
        } else {
            set_edit_modal_input_username("".to_string());
            set_edit_modal_input_admin(false);
            set_edit_modal_input_active(true);
        }
    });

    let on_edit = move |(data, on_error): (Option<User>, Callback<String>)| {
        spawn_local(async move {
            if let Err(e) = create_or_update_user(
                data.map(|x| x.username),
                edit_modal_input_username.get_untracked(),
                edit_modal_input_password.get_untracked(),
                edit_modal_input_admin.get_untracked(),
                edit_modal_input_active.get_untracked(),
            )
            .await
            {
                on_error(e.to_string())
            } else {
                reload_controller.reload();
                edit_modal_user.set(None);
            }
        });
    };

    let on_row_change = move |ev: ChangeEvent<User>| {
        spawn_local(async move {
            if let Err(e) = update_user_admin_or_active(
                ev.changed_row.username.clone(),
                ev.changed_row.admin,
                ev.changed_row.active,
            )
            .await
            {
                error!(
                    "Failed to update admin/active status of {}: {}",
                    ev.changed_row.username, e
                );
            }
            reload_controller.reload();
        });
    };

    #[allow(unused_variables, non_snake_case)]
    let user_row_renderer = move |class: Signal<String>,
                                  row: User,
                                  index: usize,
                                  selected: Signal<bool>,
                                  on_select: EventHandler<MouseEvent>,
                                  on_change: EventHandler<ChangeEvent<User>>| {
        let delete_username = row.username.clone();
        let edit_user = row.clone();
        view! {
            <tr class=class on:click=move |mouse_event| on_select.run(mouse_event)>
                {row.render_row(index, on_change)}
                <td class="w-1 px-4 py-2 whitespace-nowrap text-ellipsis">
                    <div class="inline-flex items-center rounded-md">
                        <button
                            class="text-gray-800 dark:text-zinc-100 hover:text-white dark:hover:text-black bg-white dark:bg-black hover:bg-blue-600 dark:hover:bg-blue-500 transition-all border-[1.5px] border-gray-200 dark:border-zinc-800 rounded-l-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| edit_modal_open_with(Some(edit_user.clone()))
                        >
                            <Icon icon=icondata::FiEdit class="w-5 h-5"/>
                        </button>
                        <button
                            class="text-gray-800 dark:text-zinc-100 hover:text-white dark:hover:text-black bg-white dark:bg-black hover:bg-red-600 dark:hover:bg-red-500 transition-all border-l-0 border-[1.5px] border-gray-200 dark:border-zinc-800 rounded-r-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| {
                                delete_modal_user.set(Some(delete_username.clone()));
                            }
                        >

                            <Icon icon=icondata::FiTrash2 class="w-5 h-5"/>
                        </button>
                    </div>
                </td>
            </tr>
        }
    };

    let has_password_mismatch = move || edit_modal_input_password() != edit_modal_input_password_repeat();
    let has_invalid_password = create_memo(move |_| {
        // Either we edit an existing user (in which case an empty password means no change)
        // or the password is of correct length.
        let is_new = matches!(edit_modal_user.get(), Some(None));
        let is_valid_pw = is_valid_pw(&edit_modal_input_password());
        let valid = is_valid_pw || (!is_new && edit_modal_input_password().is_empty());
        !valid
    });
    let errors = create_memo(move |_| {
        let mut errors = Vec::new();
        if has_password_mismatch() {
            errors.push("Passwords don't match".to_string());
        }
        if has_invalid_password() {
            errors.push("Password must be between 12 and 512 characters".to_string());
        }
        errors
    });

    view! {
        <div class="h-full flex-1 flex-col mt-12">
            <div class="flex items-center justify-between space-y-2 mb-4">
                <h2 class="text-4xl font-bold">Users</h2>
            </div>
            <div class="space-y-4">
                <div class="flex flex-wrap items-center justify-between">
                    <input
                        class="flex flex-none rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-base p-2.5 me-2 mb-2 w-full md:w-[360px] lg:w-[520px] transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                        type="search"
                        placeholder="Search"
                        value=rows.search
                        on:input=move |e| {
                            on_input(event_target_value(&e));
                        }
                    />

                    <button
                        type="button"
                        class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-base text-white dark:text-zinc-100 py-2.5 px-4 me-2 mb-2 transition-all rounded-lg focus:ring-4 bg-blue-600 dark:bg-blue-700 hover:bg-blue-500 dark:hover:bg-blue-600 focus:ring-blue-300 dark:focus:ring-blue-900"
                        on:click=move |_| edit_modal_open_with(None)
                    >
                        <Icon icon=icondata::FiPlus class="w-6 h-6 me-2"/>
                        New
                    </button>
                    <div class="flex flex-1"></div>
                    <div class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-base text-right px-4">
                        {count} " results"
                    </div>
                </div>

                <div class="rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 text-base flex flex-col overflow-hidden">
                    <div class="overflow-auto grow min-h-0">
                        <table class="table-auto text-left w-full">
                            <TableContent
                                rows
                                sorting=sorting
                                sorting_mode=SortingMode::SingleColumn
                                row_renderer=user_row_renderer
                                reload_controller=reload_controller
                                loading_row_display_limit=0
                                on_row_count=set_count
                                on_change=on_row_change
                            />
                        </table>
                    </div>
                </div>
            </div>
        </div>

        <DeleteModal
            data=delete_modal_user
            text="Are you sure you want to delete this user? This action cannot be undone.".into_view()
            on_confirm=move |data| {
                spawn_local(async move {
                    if let Err(e) = delete_user(data).await {
                        error!("Failed to delete user: {}", e);
                    } else {
                        reload_controller.reload();
                    }
                    delete_modal_user.set(None);
                });
            }
        />

        <EditModal
            data=edit_modal_user
            what="User".to_string()
            get_title=move |x| { &x.username }
            on_confirm=on_edit
            errors
        >
            <div class="flex flex-col gap-2">
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="username"
                >
                    Username
                </label>
                <input
                    class="flex flex-none w-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    type="text"
                    placeholder="username"
                    required="required"
                    on:input=move |ev| set_edit_modal_input_username(event_target_value(&ev))
                    prop:value=edit_modal_input_username
                    disabled=move || !matches!(edit_modal_user.get(), Some(None))
                />
            </div>
            <div class="flex flex-col gap-2">
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="password"
                >
                    {move || {
                        if matches!(edit_modal_user.get(), Some(None)) {
                            "Password"
                        } else {
                            "Password (leave empty to keep current)"
                        }
                    }}

                </label>
                <input
                    class="flex flex-none w-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    class=("!ring-4", has_invalid_password)
                    class=("!ring-red-500", has_invalid_password)
                    type="password"
                    required="required"
                    maxlength="1024"
                    on:input=move |ev| set_edit_modal_input_password(event_target_value(&ev))
                    prop:value=edit_modal_input_password
                />
            </div>
            <div class="flex flex-col gap-2">
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="password_2"
                >
                    Repeat Password
                </label>
                <input
                    class="flex flex-none w-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    class=("!ring-4", has_password_mismatch)
                    class=("!ring-red-500", has_password_mismatch)
                    type="password"
                    required="required"
                    maxlength="1024"
                    on:input=move |ev| set_edit_modal_input_password_repeat(event_target_value(&ev))
                    prop:value=edit_modal_input_password_repeat
                />
            </div>
            <div class="flex flex-row gap-2 mt-2 items-center">
                <input
                    id="users_admin"
                    class="w-4 h-4 bg-transparent dark:bg-transparent text-blue-600 border-[1.5px] border-gray-200 dark:border-zinc-800 rounded checked:bg-blue-600 dark:checked:bg-blue-600 dark:bg-blue-600 focus:ring-ring focus:ring-4 transition-all"
                    type="checkbox"
                    on:change=move |ev| set_edit_modal_input_admin(event_target_checked(&ev))
                    prop:checked=edit_modal_input_admin
                />
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="users_admin"
                >
                    Admin
                </label>
            </div>
            <div class="flex flex-row gap-2 mt-2 items-center">
                <input
                    id="users_active"
                    class="w-4 h-4 bg-transparent dark:bg-transparent text-blue-600 border-[1.5px] border-gray-200 dark:border-zinc-800 rounded checked:bg-blue-600 dark:checked:bg-blue-600 dark:bg-blue-600 focus:ring-ring focus:ring-4 transition-all"
                    type="checkbox"
                    on:change=move |ev| set_edit_modal_input_active(event_target_checked(&ev))
                    prop:checked=edit_modal_input_active
                />
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="users_active"
                >
                    Active
                </label>
            </div>
        </EditModal>
    }
}

#[component]
pub fn AccountSettings(user: crate::auth::User) -> impl IntoView {
    let edit_modal_password = create_rw_signal(None);

    let (edit_modal_input_current_password, set_edit_modal_input_current_password) = create_signal("".to_string());
    let (edit_modal_input_password, set_edit_modal_input_password) = create_signal("".to_string());
    let (edit_modal_input_password_repeat, set_edit_modal_input_password_repeat) = create_signal("".to_string());
    let edit_modal_open = move || {
        edit_modal_password.set(Some(Some(())));
        set_edit_modal_input_current_password("".to_string());
        set_edit_modal_input_password("".to_string());
        set_edit_modal_input_password_repeat("".to_string());
    };

    let on_edit = move |(_data, on_error): (Option<()>, Callback<String>)| {
        spawn_local(async move {
            if let Err(e) = change_password(
                edit_modal_input_current_password.get_untracked(),
                edit_modal_input_password.get_untracked(),
            )
            .await
            {
                on_error(e.to_string())
            } else {
                edit_modal_password.set(None);
            }
        });
    };

    let has_password_mismatch = move || edit_modal_input_password() != edit_modal_input_password_repeat();
    let has_invalid_password = create_memo(move |_| !is_valid_pw(&edit_modal_input_password()));
    let errors = create_memo(move |_| {
        let mut errors = Vec::new();
        if has_password_mismatch() {
            errors.push("Passwords don't match".to_string());
        }
        if has_invalid_password() {
            errors.push("Password must be between 12 and 512 characters".to_string());
        }
        errors
    });

    let api_token_modal = create_node_ref::<Dialog>();
    let api_token_modal_open = create_rw_signal(false);
    let api_token_modal_token = create_rw_signal("".to_string());
    let api_token_modal_copied_timer = use_timeout_fn(|_: ()| (), 3000.0);
    let (api_token_modal_server_error, api_token_modal_set_server_error) = create_signal(None);
    create_effect(move |_| {
        // Clear API token when dialog closes in any way
        if !api_token_modal_open() {
            api_token_modal_token.set("".to_string());
            (api_token_modal_copied_timer.stop)();
            api_token_modal_set_server_error(None);
        }
    });

    view! {
        <div class="h-full flex-1 flex-col mt-12">
            <div class="flex items-center justify-between space-y-2 mb-4">
                <h2 class="text-4xl font-bold">Account Settings</h2>
            </div>
            <div class="grid gap-4 grid-cols-1 sm:max-w-sm">
                <button
                    type="button"
                    class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-base text-white dark:text-zinc-100 py-2.5 px-4 transition-all rounded-lg focus:ring-4 bg-blue-600 dark:bg-blue-600 hover:bg-blue-500 dark:hover:bg-blue-500 focus:ring-blue-300 dark:focus:ring-blue-900"
                    on:click=move |_| edit_modal_open()
                >
                    "Change password"
                </button>
                <button
                    type="button"
                    class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-base text-white dark:text-zinc-100 py-2.5 px-4 transition-all rounded-lg focus:ring-4 bg-blue-600 dark:bg-blue-600 hover:bg-blue-500 dark:hover:bg-blue-500 focus:ring-blue-300 dark:focus:ring-blue-900 disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50"
                    on:click=move |_| {
                        spawn_local(async move {
                            match regenerate_api_key().await {
                                Err(e) => api_token_modal_set_server_error(Some(e.to_string())),
                                Ok(api_token) => api_token_modal_token.set(api_token),
                            }
                            api_token_modal_open.set(true)
                        });
                    }

                    disabled=user.mailbox_owner.is_none()
                >
                    {
                        if user.mailbox_owner.is_none() {
                            "Regenerate API Token (login as a mailbox first)"
                        } else {
                            "Regenerate API Token"
                        }
                    }
                </button>
            </div>
        </div>

        <EditModal
            data=edit_modal_password
            what="Password".to_string()
            get_title=move |_| { "password" }
            on_confirm=on_edit
            errors
        >
            <div class="flex flex-col gap-2">
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="password"
                >
                    "Current Password"
                </label>
                <input
                    class="flex flex-none w-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    type="password"
                    required="required"
                    maxlength="1024"
                    on:input=move |ev| set_edit_modal_input_current_password(event_target_value(&ev))
                    prop:value=edit_modal_input_current_password
                />
            </div>
            <div class="flex flex-col gap-2">
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="password"
                >
                    "Password"
                </label>
                <input
                    class="flex flex-none w-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    class=("!ring-4", has_invalid_password)
                    class=("!ring-red-500", has_invalid_password)
                    type="password"
                    required="required"
                    maxlength="1024"
                    on:input=move |ev| set_edit_modal_input_password(event_target_value(&ev))
                    prop:value=edit_modal_input_password
                />
            </div>
            <div class="flex flex-col gap-2">
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="password_2"
                >
                    Repeat Password
                </label>
                <input
                    class="flex flex-none w-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    class=("!ring-4", has_password_mismatch)
                    class=("!ring-red-500", has_password_mismatch)
                    type="password"
                    required="required"
                    maxlength="1024"
                    on:input=move |ev| set_edit_modal_input_password_repeat(event_target_value(&ev))
                    prop:value=edit_modal_input_password_repeat
                />
            </div>
        </EditModal>

        <Modal open=api_token_modal_open dialog_el=api_token_modal>
            <div class="relative p-4 transform overflow-hidden rounded-lg bg-white dark:bg-black text-left transition-all sm:w-full sm:max-w-xl">
                <h3 class="text-2xl tracking-tight mt-2 mb-2 font-semibold text-gray-900 dark:text-gray-200">
                    "API Token"
                </h3>
                <div class="pb-3 space-y-3">
                    <p class="text-sm text-gray-500 dark:text-gray-400">
                        "Your new API Token is displayed below. Make sure to save it now, as it will not be displayed again."
                    </p>
                    <div class="w-full relative">
                        <input
                            type="text"
                            class="col-span-6 bg-gray-50 dark:bg-gray-900 dark:bg-black border border-gray-300 text-gray-500 dark:text-gray-400 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full px-2.5 py-4"
                            value=move || api_token_modal_token
                            disabled
                            readonly
                        />
                        <button
                            class="absolute end-2.5 top-1/2 -translate-y-1/2 text-gray-900 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg py-2 px-2.5 inline-flex items-center justify-center bg-white dark:bg-black border-gray-200 border"
                            on:click=move |_ev| {
                                (api_token_modal_copied_timer.start)(());
                                if let Some(clipboard) = window().navigator().clipboard() {
                                    let _ = clipboard.write_text(&api_token_modal_token.get());
                                }
                            }
                        >

                            <span
                                class="inline-flex items-center"
                                class=("hidden", api_token_modal_copied_timer.is_pending)
                            >
                                <Icon icon=icondata::RiFileCopy2DocumentFill class="w-3 h-3 me-1.5"/>
                                <span class="text-xs font-semibold">Copy</span>
                            </span>
                            <span
                                class="hidden items-center"
                                class=("!inline-flex", api_token_modal_copied_timer.is_pending)
                            >
                                <Icon
                                    icon=icondata::BiCheckRegular
                                    class="w-3 h-3 me-1.5 text-blue-700 dark:text-blue-300"
                                />
                                <span class="text-xs font-semibold text-blue-700 dark:text-blue-300">Copied</span>
                            </span>
                        </button>
                    </div>
                    <Show when=move || api_token_modal_server_error().is_some()>
                        <div class="rounded-lg p-4 flex bg-red-100 dark:bg-red-900 mt-2">
                            <div>
                                <Icon icon=icondata::BiXCircleSolid class="w-5 h-5 text-red-400 dark:text-red-200"/>
                            </div>
                            <div class="ml-3 text-red-700 dark:text-red-200">
                                {move || {
                                    match api_token_modal_server_error() {
                                        None => view! {}.into_view(),
                                        Some(error) => view! { <p>{error}</p> }.into_view(),
                                    }
                                }}

                            </div>
                        </div>
                    </Show>
                </div>
                <div class="flex flex-col gap-3 sm:flex-row-reverse">
                    <button
                        type="button"
                        class="inline-flex w-full min-w-20 justify-center rounded-lg transition-all bg-white dark:bg-black px-3 py-2 font-semibold text-gray-900 dark:text-gray-200 focus:ring-4 dark:focus:ring-zinc-800 border-[1.5px] border-gray-300 dark:border-zinc-800 hover:bg-gray-100 dark:hover:bg-zinc-900 sm:w-auto"
                        on:click=move |_ev| {
                            api_token_modal_open.set(false);
                        }
                    >

                        Dismiss
                    </button>
                </div>
            </div>
        </Modal>
    }
}
