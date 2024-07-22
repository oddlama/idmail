use std::collections::VecDeque;
use std::ops::Range;
use std::str::FromStr;

use crate::auth::User;
use crate::utils::{DeleteModal, EditModal, Select};
use crate::utils::{SliderRenderer, THeadCellRenderer, TailwindClassesPreset, TimediffRenderer};

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
pub struct Alias {
    #[table(class = "w-40")]
    pub address: String,
    #[table(class = "w-40")]
    pub target: String,
    pub comment: String,
    #[table(class = "w-1", title = "Received")]
    pub n_recv: i64,
    #[table(class = "w-1", title = "Sent")]
    pub n_sent: i64,
    #[table(class = "w-1", renderer = "SliderRenderer")]
    pub active: bool,
    #[table(class = "w-1")]
    pub owner: String,
    #[table(class = "w-1", title = "Created", renderer = "TimediffRenderer")]
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AliasQuery {
    #[serde(default)]
    sort: VecDeque<(usize, ColumnSort)>,
    range: Range<usize>,
    search: String,
}

pub(crate) fn validate_email(localpart: &str, domain: &str) -> Result<String, email_address::Error> {
    let address = format!("{localpart}@{domain}");
    email_address::EmailAddress::from_str(&address).map(|x| x.to_string())
}

#[cfg(feature = "ssr")]
pub(crate) fn push_and_check_aliases_owner(query: &mut QueryBuilder<'_, sqlx::Sqlite>, username: String) {
    query.push(" AND ( owner = ");
    query.push_bind(username.clone());
    query.push(
        " OR owner IN ( \
        SELECT address from mailboxes WHERE owner = ",
    );
    query.push_bind(username.clone());
    query.push(" ) )");
}

#[server]
pub async fn list_aliases(query: AliasQuery) -> Result<Vec<Alias>, ServerFnError> {
    let user = crate::auth::auth_any().await?;

    let AliasQuery { sort, range, search } = query;

    let mut query = QueryBuilder::new("SELECT * FROM aliases WHERE 1=1");
    if !user.admin {
        push_and_check_aliases_owner(&mut query, user.username.clone());
    }
    if !search.is_empty() {
        query.push(" AND ( address LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%') OR comment LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%') OR owner LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%') )");
    }

    if let Some(order) = Alias::sorting_to_sql(&sort) {
        query.push(" ");
        query.push(order);
    }

    query.push(" LIMIT ");
    query.push_bind(range.len() as i64);
    query.push(" OFFSET ");
    query.push_bind(range.start as i64);

    let pool = crate::database::ssr::pool()?;
    Ok(query.build_query_as::<Alias>().fetch_all(&pool).await?)
}

/// Count all aliases, or just active/inactive ones if specified.
#[server]
pub async fn alias_count(active: Option<bool>, since: Option<DateTime<Utc>>) -> Result<usize, ServerFnError> {
    let user = crate::auth::auth_any().await?;

    let mut query = QueryBuilder::new("SELECT COUNT(*) FROM aliases WHERE 1=1");
    if !user.admin {
        push_and_check_aliases_owner(&mut query, user.username.clone());
    }
    if let Some(active) = active {
        query.push(" AND active = ");
        query.push_bind(active);
    }
    if let Some(since) = since {
        query.push(" AND created_at >=");
        query.push_bind(since);
    }

    let pool = crate::database::ssr::pool()?;
    let count = query.build_query_scalar::<i64>().fetch_one(&pool).await?;

    Ok(count as usize)
}

#[server]
pub async fn delete_alias(address: String) -> Result<(), ServerFnError> {
    let user = crate::auth::auth_any().await?;

    let mut query = QueryBuilder::new("DELETE FROM aliases WHERE address = ");
    query.push_bind(address);

    // Non-admins can only delete their own aliases
    if !user.admin {
        push_and_check_aliases_owner(&mut query, user.username.clone());
    }

    let pool = crate::database::ssr::pool()?;
    query.build().execute(&pool).await.map(|_| ())?;
    Ok(())
}

#[server]
pub async fn create_or_update_alias(
    old_address: Option<String>,
    alias: String,
    domain: String,
    target: String,
    comment: String,
    active: bool,
    owner: String,
) -> Result<(), ServerFnError> {
    use crate::mailboxes::allowed_targets;

    let user = crate::auth::auth_any().await?;
    let pool = crate::database::ssr::pool()?;

    let target = if target.is_empty() || !user.admin {
        if user.mailbox_owner.is_some() {
            &user.username
        } else {
            if !allowed_targets().await?.contains(&target) {
                return Err(ServerFnError::new("target must be set to a valid email address"));
            }
            &target
        }
    } else {
        &target
    };

    let owner = if user.admin {
        // Only admins can assign other owners
        owner.trim()
    } else if user.mailbox_owner.is_some() {
        // Mailbox users cannot change the owner
        &user.username
    } else {
        // Normal users must use the target as an owner
        target
    };

    // Empty owner -> self owned
    let owner = if owner.is_empty() { &user.username } else { owner };

    // Check if address is valid
    let address = validate_email(&alias, &domain)?;

    if let Some(old_address) = old_address {
        let mut query = QueryBuilder::new("UPDATE aliases SET address = ");
        query.push_bind(address);
        query.push(", domain = ");
        query.push_bind(domain);
        query.push(", target = ");
        query.push_bind(target);
        query.push(", comment = ");
        query.push_bind(comment);
        query.push(", active = ");
        query.push_bind(active);
        query.push(", owner = ");
        query.push_bind(owner);
        query.push(" WHERE address = ");
        query.push_bind(old_address);
        if !user.admin {
            push_and_check_aliases_owner(&mut query, user.username.clone());
        }

        query.build().execute(&pool).await.map(|_| ())?;
    } else {
        sqlx::query("INSERT INTO aliases (address, domain, target, comment, active, owner) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(address)
            .bind(domain)
            .bind(target)
            .bind(comment)
            .bind(active)
            .bind(owner)
            .execute(&pool)
            .await
            .map(|_| ())?;
    }

    Ok(())
}

#[server]
pub async fn update_alias_active(address: String, active: bool) -> Result<(), ServerFnError> {
    let user = crate::auth::auth_any().await?;
    let mut query = QueryBuilder::new("UPDATE aliases SET active = ");
    query.push_bind(active);
    query.push(" WHERE address = ");
    query.push_bind(address);

    // Non-admins can only change their own aliases
    if !user.admin {
        push_and_check_aliases_owner(&mut query, user.username.clone());
    }

    let pool = crate::database::ssr::pool()?;
    query.build().execute(&pool).await.map(|_| ())?;
    Ok(())
}

#[derive(Default)]
pub struct AliasTableDataProvider {
    sort: VecDeque<(usize, ColumnSort)>,
    pub search: RwSignal<String>,
}

impl TableDataProvider<Alias> for AliasTableDataProvider {
    async fn get_rows(&self, range: Range<usize>) -> Result<(Vec<Alias>, Range<usize>), String> {
        list_aliases(AliasQuery {
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
        alias_count(None, None).await.ok()
    }

    fn set_sorting(&mut self, sorting: &VecDeque<(usize, ColumnSort)>) {
        self.sort.clone_from(sorting);
    }

    fn track(&self) {
        self.search.track();
    }
}

#[component]
pub fn Aliases(user: User, reload_stats: Callback<()>) -> impl IntoView {
    let mut rows = AliasTableDataProvider::default();
    let default_sorting = VecDeque::from([(7, ColumnSort::Descending)]);
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
                Ok(domains) => set_allowed_domains(domains),
            }
        });
    };

    let (allowed_targets, set_allowed_targets) = create_signal(vec![]);
    let refresh_targets = move || {
        spawn_local(async move {
            use crate::mailboxes::allowed_targets;
            match allowed_targets().await {
                Err(e) => error!("Failed to load allowed targets: {}", e),
                Ok(targets) => set_allowed_targets(targets),
            }
        });
    };

    if is_browser() {
        refresh_domains();
        refresh_targets();
    }

    let delete_modal_alias = create_rw_signal(None);
    let edit_modal_alias = create_rw_signal(None);

    let (edit_modal_input_alias, set_edit_modal_input_alias) = create_signal("".to_string());
    let (edit_modal_input_domain, set_edit_modal_input_domain) = create_signal("".to_string());
    let (edit_modal_input_target, set_edit_modal_input_target) = create_signal("".to_string());
    let (edit_modal_input_comment, set_edit_modal_input_comment) = create_signal("".to_string());
    let (edit_modal_input_active, set_edit_modal_input_active) = create_signal(true);
    let (edit_modal_input_owner, set_edit_modal_input_owner) = create_signal("".to_string());

    if !user.admin && user.mailbox_owner.is_none() {
        // Non-admin users that are not mailboxes always need to use the
        // target as the owner
        create_effect(move |_| {
            set_edit_modal_input_owner(edit_modal_input_target());
        });
    }

    let username = user.username.clone();
    let is_mailbox = user.mailbox_owner.is_some();
    let edit_modal_open_with = Callback::new(move |edit_alias: Option<Alias>| {
        refresh_domains();
        refresh_targets();
        edit_modal_alias.set(Some(edit_alias.clone()));

        let allowed_targets = allowed_targets.get();
        let allowed_domains = allowed_domains.get();
        if let Some(edit_alias) = edit_alias {
            let (alias, domain) = match edit_alias.address.split_once('@') {
                Some((alias, domain)) => (alias.to_string(), domain.to_string()),
                None => (edit_alias.address.clone(), "".to_string()),
            };
            set_edit_modal_input_alias(alias.to_string());
            if !allowed_domains.contains(&domain) {
                set_edit_modal_input_domain(allowed_domains.first().cloned().unwrap_or("".to_string()));
            } else {
                set_edit_modal_input_domain(domain);
            }
            set_edit_modal_input_target(edit_alias.target.clone());
            set_edit_modal_input_comment(edit_alias.comment.clone());
            set_edit_modal_input_active(edit_alias.active);
            set_edit_modal_input_owner(edit_alias.owner.clone());
        } else {
            // Only set the input domain if the current one is not in the list
            // of allowed domains. This allows users to keep the old value
            // between alias creations, making it easier to create multiple
            // aliases on the same domain.
            set_edit_modal_input_alias("".to_string());
            if !allowed_domains.contains(&edit_modal_input_domain()) {
                set_edit_modal_input_domain(allowed_domains.first().cloned().unwrap_or("".to_string()));
            }
            // set_edit_modal_input_owner is already set by settings target via an effect,
            // but we initialize it before in case we are an admin or mailbox
            set_edit_modal_input_owner(username.clone());
            if is_mailbox {
                set_edit_modal_input_target(username.clone());
            } else if !allowed_targets.contains(&edit_modal_input_target()) {
                set_edit_modal_input_target(allowed_targets.first().cloned().unwrap_or("".to_string()));
            }
            set_edit_modal_input_comment("".to_string());
            set_edit_modal_input_active(true);
        }
    });

    let on_edit = move |(data, on_error): (Option<Alias>, Callback<String>)| {
        spawn_local(async move {
            if let Err(e) = create_or_update_alias(
                data.map(|x| x.address),
                edit_modal_input_alias.get_untracked(),
                edit_modal_input_domain.get_untracked(),
                edit_modal_input_target.get_untracked(),
                edit_modal_input_comment.get_untracked(),
                edit_modal_input_active.get_untracked(),
                edit_modal_input_owner.get_untracked(),
            )
            .await
            {
                on_error(e.to_string())
            } else {
                reload.notify();
                edit_modal_alias.set(None);
            }
        });
    };

    let on_row_change = move |ev: ChangeEvent<Alias>| {
        spawn_local(async move {
            if let Err(e) = update_alias_active(ev.changed_row.address.clone(), ev.changed_row.active).await {
                error!("Failed to update active status of {}: {}", ev.changed_row.address, e);
            }
            reload.notify();
        });
    };

    #[allow(unused_variables, non_snake_case)]
    let alias_row_renderer = move |class: Signal<String>,
                                   row: Alias,
                                   index: usize,
                                   selected: Signal<bool>,
                                   on_select: EventHandler<MouseEvent>,
                                   on_change: EventHandler<ChangeEvent<Alias>>| {
        let delete_address = row.address.clone();
        let edit_alias = row.clone();
        view! {
            <tr class=class on:click=move |mouse_event| on_select.run(mouse_event)>
                {row.render_row(index, on_change)}
                <td class="w-1 px-4 py-2 whitespace-nowrap text-ellipsis">
                    <div class="inline-flex items-center rounded-md">
                        <button
                            class="text-gray-800 dark:text-zinc-100 hover:text-white dark:hover:text-black bg-white dark:bg-black hover:bg-blue-600 dark:hover:bg-blue-500 transition-all border-[1.5px] border-gray-200 dark:border-zinc-800 rounded-l-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| edit_modal_open_with(Some(edit_alias.clone()))
                        >
                            <Icon icon=icondata::FiEdit class="w-5 h-5"/>
                        </button>
                        <button
                            class="text-gray-800 dark:text-zinc-100 hover:text-white dark:hover:text-black bg-white dark:bg-black hover:bg-red-600 dark:hover:bg-red-500 transition-all border-l-0 border-[1.5px] border-gray-200 dark:border-zinc-800 rounded-r-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| {
                                delete_modal_alias.set(Some(delete_address.clone()));
                            }
                        >

                            <Icon icon=icondata::FiTrash2 class="w-5 h-5"/>
                        </button>
                    </div>
                </td>
            </tr>
        }
    };

    let has_invalid_email =
        create_memo(move |_| validate_email(&edit_modal_input_alias(), &edit_modal_input_domain()).is_err());
    let has_invalid_target = create_memo(move |_| {
        email_address::EmailAddress::from_str(&edit_modal_input_target()).is_err()
            || (!user.admin && edit_modal_input_target().is_empty())
    });

    let errors = create_memo(move |_| {
        let mut errors = Vec::<String>::new();
        if let Err(e) = validate_email(&edit_modal_input_alias(), &edit_modal_input_domain()) {
            errors.push(format!("invalid alias address: {}", e));
        }
        if let Err(e) = email_address::EmailAddress::from_str(&edit_modal_input_target()) {
            errors.push(format!("invalid target address: {}", e));
        }
        errors
    });

    view! {
        <div class="h-full flex-1 flex-col mt-12">
            <div class="flex items-center justify-between space-y-2 mb-4">
                <h2 class="text-4xl font-bold">Aliases</h2>
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
                        "New"
                    </button>
                    <button
                        type="button"
                        class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-base text-white dark:text-zinc-100 py-2.5 px-4 me-2 mb-2 transition-all rounded-lg focus:ring-4 bg-green-600 dark:bg-green-700 hover:bg-green-500 dark:hover:bg-green-600 focus:ring-green-300 dark:focus:ring-green-900"
                    >
                        <Icon icon=icondata::FaDiceSolid class="w-6 h-6 me-2"/>
                        "New Random"
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
                                row_renderer=alias_row_renderer
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
            data=delete_modal_alias
            text="Are you sure you want to delete this alias? This action cannot be undone.".into_view()
            on_confirm=move |data| {
                spawn_local(async move {
                    if let Err(e) = delete_alias(data).await {
                        error!("Failed to delete alias: {}", e);
                    } else {
                        reload.notify();
                    }
                    delete_modal_alias.set(None);
                });
            }
        />

        <EditModal
            data=edit_modal_alias
            what="Alias".to_string()
            get_title=move |x| { &x.address }
            on_confirm=on_edit
            errors
        >
            <div class="flex flex-col sm:flex-row">
                <div class="flex flex-1 flex-col gap-2">
                    <label
                        class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                        for="alias"
                    >
                        Alias
                    </label>
                    <div class="flex flex-row">
                        <input
                            class="flex sm:min-w-32 flex-1 rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                            class=("!ring-4", has_invalid_email)
                            class=("!ring-red-500", has_invalid_email)
                            type="email"
                            placeholder="alias"
                            on:input=move |ev| set_edit_modal_input_alias(event_target_value(&ev))
                            prop:value=edit_modal_input_alias
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
                    for="target"
                >
                    Target
                </label>

                {if user.admin || user.mailbox_owner.is_some() {
                    view! {
                        <input
                            class="flex flex-none w-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                            class=("!ring-4", has_invalid_target)
                            class=("!ring-red-500", has_invalid_target)
                            type="email"
                            placeholder=if user.mailbox_owner.is_some() {
                                user.username.clone()
                            } else {
                                "target@example.com".to_string()
                            }

                            on:input=move |ev| set_edit_modal_input_target(event_target_value(&ev))
                            prop:value=edit_modal_input_target
                            disabled=!user.admin
                        />
                    }
                        .into_view()
                } else {
                    view! {
                        <Select
                            class="w-full h-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all focus:ring-4 focus:ring-blue-300 dark:focus:ring-blue-900"
                            choices=allowed_targets
                            value=edit_modal_input_target
                            set_value=set_edit_modal_input_target
                        />
                    }
                        .into_view()
                }}

            </div>
            <div class="flex flex-col gap-2">
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="comment"
                >
                    Comment
                </label>
                <input
                    class="flex flex-none w-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    type="text"
                    placeholder="Comment"
                    on:input=move |ev| set_edit_modal_input_comment(event_target_value(&ev))
                    prop:value=edit_modal_input_comment
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
                    placeholder=edit_modal_input_owner
                    on:input=move |ev| set_edit_modal_input_owner(event_target_value(&ev))
                    prop:value=edit_modal_input_owner
                    disabled=!user.admin
                />
            </div>
            <div class="flex flex-row gap-2 mt-2 items-center">
                <input
                    id="alias_active"
                    class="w-4 h-4 bg-transparent dark:bg-transparent text-blue-600 border-[1.5px] border-gray-200 dark:border-zinc-800 rounded checked:bg-blue-600 dark:checked:bg-blue-600 dark:bg-blue-600 focus:ring-ring focus:ring-4 transition-all"
                    type="checkbox"
                    on:change=move |ev| set_edit_modal_input_active(event_target_checked(&ev))
                    prop:checked=edit_modal_input_active
                />
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="alias_active"
                >
                    Active
                </label>
            </div>
        </EditModal>
    }
}
