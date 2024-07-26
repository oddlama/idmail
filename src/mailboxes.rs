use std::collections::VecDeque;
use std::ops::Range;

use crate::aliases::validate_address;
use crate::users::is_valid_pw;
use crate::utils::{DeleteModal, EditModal, Select};
use crate::utils::{SliderRenderer, THeadCellRenderer, TailwindClassesPreset, TimediffRenderer};

use crate::auth::User;
use chrono::{DateTime, Utc};
use leptos::leptos_dom::is_browser;
use leptos::{ev::MouseEvent, logging::error, *};
use leptos_icons::Icon;
use leptos_struct_table::*;
use leptos_use::use_debounce_fn_with_arg;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use sqlx::QueryBuilder;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TableRow)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[table(sortable, classes_provider = TailwindClassesPreset, thead_cell_renderer = THeadCellRenderer)]
pub struct Mailbox {
    pub address: String,
    #[table(skip)]
    pub password_hash: String,
    #[table(class = "w-1", renderer = "SliderRenderer")]
    pub active: bool,
    #[table(class = "w-1")]
    pub owner: String,
    #[table(class = "w-1", title = "Created", renderer = "TimediffRenderer")]
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MailboxQuery {
    #[serde(default)]
    sort: VecDeque<(usize, ColumnSort)>,
    range: Range<usize>,
    search: String,
}

#[server]
pub async fn allowed_targets() -> Result<Vec<String>, ServerFnError> {
    let user = crate::auth::auth_any().await?;

    // Mailbox users can only target themselves
    if user.mailbox_owner.is_some() {
        return Ok(vec![user.username]);
    }

    let mut query = QueryBuilder::new("SELECT address FROM mailboxes");
    query.push(" WHERE owner = ");
    query.push_bind(&user.username);

    let pool = crate::database::ssr::pool()?;
    Ok(query.build_query_scalar::<String>().fetch_all(&pool).await?)
}

#[server]
pub async fn list_mailboxes(query: MailboxQuery) -> Result<Vec<Mailbox>, ServerFnError> {
    let user = crate::auth::auth_user().await?;

    let MailboxQuery { sort, range, search } = query;

    let mut query = QueryBuilder::new("SELECT * FROM mailboxes WHERE 1=1");
    if !user.admin {
        query.push(" AND owner = ");
        query.push_bind(&user.username);
    }
    if !search.is_empty() {
        query.push(" AND ( address LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%') OR owner LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%') )");
    }

    if let Some(order) = Mailbox::sorting_to_sql(&sort) {
        query.push(" ");
        query.push(order);
    }

    query.push(" LIMIT ");
    query.push_bind(range.len() as i64);
    query.push(" OFFSET ");
    query.push_bind(range.start as i64);

    let pool = crate::database::ssr::pool()?;
    Ok(query.build_query_as::<Mailbox>().fetch_all(&pool).await?)
}

#[server]
pub async fn mailbox_count() -> Result<usize, ServerFnError> {
    let user = crate::auth::auth_user().await?;

    let mut query = QueryBuilder::new("SELECT COUNT(*) FROM mailboxes");
    if !user.admin {
        query.push(" WHERE owner = ");
        query.push_bind(&user.username);
    }

    let pool = crate::database::ssr::pool()?;
    let count = query.build_query_scalar::<i64>().fetch_one(&pool).await?;

    Ok(count as usize)
}

#[server]
pub async fn delete_mailbox(address: String) -> Result<(), ServerFnError> {
    let user = crate::auth::auth_user().await?;

    let mut query = QueryBuilder::new("DELETE FROM mailboxes WHERE address = ");
    query.push_bind(address);

    // Non-admins can only delete their own mailboxes
    if !user.admin {
        query.push(" AND owner = ");
        query.push_bind(&user.username);
    }

    let pool = crate::database::ssr::pool()?;
    query.build().execute(&pool).await.map(|_| ())?;
    Ok(())
}

#[server]
pub async fn create_or_update_mailbox(
    old_address: Option<String>,
    localpart: String,
    domain: String,
    password: String,
    active: bool,
    owner: String,
) -> Result<(), ServerFnError> {
    use crate::users::mk_password_hash;
    use crate::domains::allowed_domains;

    let user = crate::auth::auth_user().await?;
    let pool = crate::database::ssr::pool()?;

    // Only admins can assign other owners
    let owner = if user.admin { owner.trim() } else { &user.username };
    // Empty owner -> self owned
    let owner = if owner.is_empty() { &user.username } else { owner };

    // Check if address is valid
    let allowed_domains = allowed_domains().await?;
    let Some(db_domain) = allowed_domains.iter().find(|x| x.0 == domain) else {
        return Err(ServerFnError::new("domain must be set to a valid domain"));
    };

    let address = validate_address(&localpart, &domain, user.admin || db_domain.1 == user.username)
        .map_err(ServerFnError::new)?;

    if let Some(old_address) = old_address {
        let mut query = QueryBuilder::new("UPDATE mailboxes SET address = ");
        query.push_bind(address);
        query.push(", domain = ");
        query.push_bind(domain);
        if !password.is_empty() {
            let password_hash = mk_password_hash(&password)?;
            query.push(", password_hash = ");
            query.push_bind(password_hash);
        }
        query.push(", active = ");
        query.push_bind(active);
        query.push(", owner = ");
        query.push_bind(owner);
        query.push(" WHERE address = ");
        query.push_bind(old_address);
        if !user.admin {
            query.push(" AND owner = ");
            query.push_bind(&user.username);
        }

        query.build().execute(&pool).await.map(|_| ())?;
    } else {
        let password_hash = mk_password_hash(&password)?;
        sqlx::query("INSERT INTO mailboxes (address, domain, password_hash, active, owner) VALUES (?, ?, ?, ?, ?)")
            .bind(address)
            .bind(domain)
            .bind(password_hash)
            .bind(active)
            .bind(owner)
            .execute(&pool)
            .await
            .map(|_| ())?;
    }

    Ok(())
}

#[server]
pub async fn update_mailbox_active(address: String, active: bool) -> Result<(), ServerFnError> {
    let user = crate::auth::auth_user().await?;
    let mut query = QueryBuilder::new("UPDATE mailboxes SET active = ");
    query.push_bind(active);
    query.push(" WHERE address = ");
    query.push_bind(address);

    // Non-admins can only change their own domains
    if !user.admin {
        query.push(" AND owner = ");
        query.push_bind(&user.username);
    }

    let pool = crate::database::ssr::pool()?;
    query.build().execute(&pool).await.map(|_| ())?;
    Ok(())
}

#[derive(Default)]
pub struct MailboxTableDataProvider {
    sort: VecDeque<(usize, ColumnSort)>,
    pub search: RwSignal<String>,
}

impl TableDataProvider<Mailbox> for MailboxTableDataProvider {
    async fn get_rows(&self, range: Range<usize>) -> Result<(Vec<Mailbox>, Range<usize>), String> {
        list_mailboxes(MailboxQuery {
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
        mailbox_count().await.ok()
    }

    fn set_sorting(&mut self, sorting: &VecDeque<(usize, ColumnSort)>) {
        self.sort = sorting.clone();
    }

    fn track(&self) {
        self.search.track();
    }
}

#[component]
pub fn Mailboxes(user: User, reload_stats: Callback<()>) -> impl IntoView {
    let mut rows = MailboxTableDataProvider::default();
    let default_sorting = VecDeque::from([(3, ColumnSort::Descending)]);
    rows.set_sorting(&default_sorting);
    let sorting = create_rw_signal(default_sorting);

    let reload = create_trigger();
    let reload_controller = ReloadController::default();
    create_effect(move |_| {
        reload.track();
        reload_controller.reload();
        reload_stats(());
    });

    let on_input = use_debounce_fn_with_arg(move |value| rows.search.set(value), 300.0);
    let (count, set_count) = create_signal(0);

    let (allowed_domains, set_allowed_domains) = create_signal(vec![]);
    let refresh_domains = move || {
        spawn_local(async move {
            use crate::domains::allowed_domains;
            match allowed_domains().await {
                Err(e) => error!("Failed to load allowed domains: {}", e),
                Ok(domains) => set_allowed_domains(domains.into_iter().map(|x| x.0).collect()),
            }
        });
    };

    if is_browser() {
        refresh_domains();
    }

    let delete_modal_mailbox = create_rw_signal(None);
    let edit_modal_mailbox = create_rw_signal(None);

    let (edit_modal_input_localpart, set_edit_modal_input_localpart) = create_signal("".to_string());
    let (edit_modal_input_domain, set_edit_modal_input_domain) = create_signal("".to_string());
    let (edit_modal_input_password, set_edit_modal_input_password) = create_signal("".to_string());
    let (edit_modal_input_password_repeat, set_edit_modal_input_password_repeat) = create_signal("".to_string());
    let (edit_modal_input_active, set_edit_modal_input_active) = create_signal(true);
    let (edit_modal_input_owner, set_edit_modal_input_owner) = create_signal("".to_string());
    let edit_modal_open_with = Callback::new(move |edit_mailbox: Option<Mailbox>| {
        refresh_domains();
        edit_modal_mailbox.set(Some(edit_mailbox.clone()));
        set_edit_modal_input_password("".to_string());
        set_edit_modal_input_password_repeat("".to_string());

        let allowed_domains = allowed_domains.get();
        if let Some(edit_mailbox) = edit_mailbox {
            let (localpart, domain) = match edit_mailbox.address.split_once('@') {
                Some((localpart, domain)) => (localpart.to_string(), domain.to_string()),
                None => (edit_mailbox.address.clone(), "".to_string()),
            };
            set_edit_modal_input_localpart(localpart.to_string());
            if !allowed_domains.contains(&domain) {
                set_edit_modal_input_domain(allowed_domains.first().cloned().unwrap_or("".to_string()));
            } else {
                set_edit_modal_input_domain(domain);
            }
            set_edit_modal_input_active(edit_mailbox.active);
            set_edit_modal_input_owner(edit_mailbox.owner.clone());
        } else {
            // Only set the input domain if the current one is not in the list
            // of allowed domains. This allows users to keep the old value
            // between mailbox creations, making it easier to create multiple
            // mailboxes on the same domain.
            set_edit_modal_input_localpart("".to_string());
            if !allowed_domains.contains(&edit_modal_input_domain()) {
                set_edit_modal_input_domain(allowed_domains.first().cloned().unwrap_or("".to_string()));
            }
            set_edit_modal_input_active(true);
            set_edit_modal_input_owner("".to_string());
        }
    });

    let on_edit = move |(data, on_error): (Option<Mailbox>, Callback<String>)| {
        spawn_local(async move {
            if let Err(e) = create_or_update_mailbox(
                data.map(|x| x.address),
                edit_modal_input_localpart.get_untracked(),
                edit_modal_input_domain.get_untracked(),
                edit_modal_input_password.get_untracked(),
                edit_modal_input_active.get_untracked(),
                edit_modal_input_owner.get_untracked(),
            )
            .await
            {
                on_error(e.to_string())
            } else {
                reload.notify();
                edit_modal_mailbox.set(None);
            }
        });
    };

    let on_row_change = move |ev: ChangeEvent<Mailbox>| {
        spawn_local(async move {
            if let Err(e) = update_mailbox_active(ev.changed_row.address.clone(), ev.changed_row.active).await {
                error!("Failed to update active status of {}: {}", ev.changed_row.address, e);
            }
            reload.notify();
        });
    };

    #[allow(unused_variables, non_snake_case)]
    let mailbox_row_renderer = move |class: Signal<String>,
                                     row: Mailbox,
                                     index: usize,
                                     selected: Signal<bool>,
                                     on_select: EventHandler<MouseEvent>,
                                     on_change: EventHandler<ChangeEvent<Mailbox>>| {
        let delete_address = row.address.clone();
        let edit_mailbox = row.clone();
        view! {
            <tr class=class on:click=move |mouse_event| on_select.run(mouse_event)>
                {row.render_row(index, on_change)}
                <td class="w-1 px-4 py-2 whitespace-nowrap text-ellipsis">
                    <div class="inline-flex items-center rounded-md">
                        <button
                            class="text-gray-800 dark:text-zinc-100 hover:text-white dark:hover:text-black bg-white dark:bg-black hover:bg-blue-600 dark:hover:bg-blue-500 transition-all border-[1.5px] border-gray-200 dark:border-zinc-800 rounded-l-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| edit_modal_open_with(Some(edit_mailbox.clone()))
                        >
                            <Icon icon=icondata::FiEdit class="w-5 h-5"/>
                        </button>
                        <button
                            class="text-gray-800 dark:text-zinc-100 hover:text-white dark:hover:text-black bg-white dark:bg-black hover:bg-red-600 dark:hover:bg-red-500 transition-all border-l-0 border-[1.5px] border-gray-200 dark:border-zinc-800 rounded-r-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| {
                                delete_modal_mailbox.set(Some(delete_address.clone()));
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
        // Either we edit an existing mailbox (in which case an empty password means no change)
        // or the password is of correct length.
        let is_new = matches!(edit_modal_mailbox.get(), Some(None));
        let is_valid_pw = is_valid_pw(&edit_modal_input_password());
        let valid = is_valid_pw || (!is_new && edit_modal_input_password().is_empty());
        !valid
    });
    let has_invalid_address = create_memo(move |_| {
        validate_address(
            &edit_modal_input_localpart(),
            &edit_modal_input_domain(),
            true, /* error on create to save resources */
        )
        .is_err()
    });
    let errors = create_memo(move |_| {
        let mut errors = Vec::new();
        if let Err(e) = validate_address(
            &edit_modal_input_localpart(),
            &edit_modal_input_domain(),
            true, /* error on create to save resources */
        ) {
            errors.push(format!("invalid address: {}", e));
        }
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
                <h2 class="text-4xl font-bold">Mailboxes</h2>
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
                                row_renderer=mailbox_row_renderer
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
            data=delete_modal_mailbox
            text="Are you sure you want to delete this mailbox? This action cannot be undone.".into_view()
            on_confirm=move |data| {
                spawn_local(async move {
                    if let Err(e) = delete_mailbox(data).await {
                        error!("Failed to delete mailbox: {}", e);
                    } else {
                        reload.notify();
                    }
                    delete_modal_mailbox.set(None);
                });
            }
        />

        <EditModal
            data=edit_modal_mailbox
            what="Mailbox".to_string()
            get_title=move |x| { &x.address }
            on_confirm=on_edit
            errors
        >
            <div class="flex flex-col sm:flex-row">
                <div class="flex flex-1 flex-col gap-2">
                    <label
                        class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                        for="mailbox"
                    >
                        Mailbox
                    </label>
                    <div class="flex flex-row">
                        <input
                            class="flex sm:min-w-32 flex-1 rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                            class=("!ring-4", has_invalid_address)
                            class=("!ring-red-500", has_invalid_address)
                            type="email"
                            placeholder="mailbox"
                            on:input=move |ev| set_edit_modal_input_localpart(event_target_value(&ev))
                            prop:value=edit_modal_input_localpart
                        />
                        <span class="inline-flex flex-none text-base items-center mx-2">@</span>
                    </div>
                </div>
                <div class="flex flex-col gap-2">
                    <label
                        class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 mt-3 sm:mt-0"
                        for="domain"
                    >
                        Domain
                    </label>
                    <Select
                        class="w-full h-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all focus:ring-4 focus:ring-blue-300 dark:focus:ring-blue-900"
                        choices=allowed_domains
                        value=edit_modal_input_domain
                        set_value=set_edit_modal_input_domain
                    />
                </div>
            </div>
            <div class="flex flex-col gap-2">
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="password"
                >
                    {move || {
                        if matches!(edit_modal_mailbox.get(), Some(None)) {
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
            <div class="flex flex-col gap-2">
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="owner"
                >
                    Owner
                </label>
                <input
                    class="flex flex-none w-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    type="text"
                    placeholder=user.username.clone()
                    on:input=move |ev| set_edit_modal_input_owner(event_target_value(&ev))
                    prop:value=edit_modal_input_owner
                    disabled=!user.admin
                />
            </div>
            <div class="flex flex-row gap-2 mt-2 items-center">
                <input
                    id="mailboxes_active"
                    class="w-4 h-4 bg-transparent dark:bg-transparent text-blue-600 border-[1.5px] border-gray-200 dark:border-zinc-800 rounded checked:bg-blue-600 dark:checked:bg-blue-600 dark:bg-blue-600 focus:ring-ring focus:ring-4 transition-all"
                    type="checkbox"
                    on:change=move |ev| set_edit_modal_input_active(event_target_checked(&ev))
                    prop:checked=edit_modal_input_active
                />
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="mailboxes_active"
                >
                    Active
                </label>
            </div>
        </EditModal>
    }
}
