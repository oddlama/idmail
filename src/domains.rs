use std::collections::VecDeque;
use std::ops::Range;

use crate::auth::User;
use crate::utils::{DeleteModal, EditModal};
use crate::utils::{SliderRenderer, THeadCellRenderer, TailwindClassesPreset, TimediffRenderer};

use chrono::{DateTime, Utc};
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
pub struct Domain {
    #[table(class = "w-40")]
    pub domain: String,
    pub catch_all: Option<String>,
    #[table(class = "w-1", renderer = "SliderRenderer")]
    pub public: bool,
    #[table(class = "w-1", renderer = "SliderRenderer")]
    pub active: bool,
    #[table(class = "w-1")]
    pub owner: String,
    #[table(class = "w-1", title = "Created", renderer = "TimediffRenderer")]
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainQuery {
    #[serde(default)]
    sort: VecDeque<(usize, ColumnSort)>,
    range: Range<usize>,
    search: String,
}

#[server]
pub async fn allowed_domains() -> Result<Vec<(String, String)>, ServerFnError> {
    let user = crate::auth::auth_any().await?;

    let mut query = QueryBuilder::new("SELECT domain, owner FROM domains");
    query.push(" WHERE active = TRUE AND (public = TRUE OR owner = ");
    query.push_bind(&user.username);
    if let Some(mailbox_owner) = user.mailbox_owner {
        query.push(" OR owner = ");
        query.push_bind(mailbox_owner.clone());
    }
    query.push(")");

    let pool = crate::database::ssr::pool()?;
    Ok(query.build_query_as::<(String, String)>().fetch_all(&pool).await?)
}

#[server]
pub async fn list_domains(query: DomainQuery) -> Result<Vec<Domain>, ServerFnError> {
    let user = crate::auth::auth_user().await?;

    let DomainQuery { sort, range, search } = query;

    let mut query = QueryBuilder::new("SELECT * FROM domains WHERE 1=1");
    if !user.admin {
        query.push(" AND owner = ");
        query.push_bind(&user.username);
    }
    if !search.is_empty() {
        query.push(" AND ( domain LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%') OR catch_all LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%') OR owner LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%') )");
    }

    if let Some(order) = Domain::sorting_to_sql(&sort) {
        query.push(" ");
        query.push(order);
    }

    query.push(" LIMIT ");
    query.push_bind(range.len() as i64);
    query.push(" OFFSET ");
    query.push_bind(range.start as i64);

    let pool = crate::database::ssr::pool()?;
    Ok(query.build_query_as::<Domain>().fetch_all(&pool).await?)
}

#[server]
pub async fn domain_count() -> Result<usize, ServerFnError> {
    let user = crate::auth::auth_user().await?;

    let mut query = QueryBuilder::new("SELECT COUNT(*) FROM domains");
    if !user.admin {
        query.push(" WHERE owner = ");
        query.push_bind(&user.username);
    }

    let pool = crate::database::ssr::pool()?;
    let count = query.build_query_scalar::<i64>().fetch_one(&pool).await?;

    Ok(count as usize)
}

#[server]
pub async fn delete_domain(domain: String) -> Result<(), ServerFnError> {
    // Creating/Deleting only as admin!
    let user = crate::auth::auth_admin().await?;

    let mut query = QueryBuilder::new("DELETE FROM domains WHERE domain = ");
    query.push_bind(domain);

    // (Hypothetical) Non-admins can only delete their own domains
    if !user.admin {
        query.push(" AND owner = ");
        query.push_bind(&user.username);
    }

    let pool = crate::database::ssr::pool()?;
    query.build().execute(&pool).await.map(|_| ())?;
    Ok(())
}

#[server]
pub async fn create_or_update_domain(
    old_domain: Option<String>,
    domain: String,
    catch_all: String,
    public: bool,
    active: bool,
    owner: String,
) -> Result<(), ServerFnError> {
    let user = if old_domain.is_some() {
        // Editing is allowed for some users
        crate::auth::auth_user().await?
    } else {
        // Creation only as admin.
        crate::auth::auth_admin().await?
    };
    let pool = crate::database::ssr::pool()?;

    // Only admins can assign other owners
    let owner = if user.admin { owner.trim() } else { &user.username };
    // Empty owner -> self owned
    let owner = if owner.is_empty() { &user.username } else { owner };
    // Only admins may create public domains
    let public = public && user.admin;
    // TODO: FIXME: invalid detect (empty, @@, ...)

    if let Some(old_domain) = old_domain {
        let mut query = QueryBuilder::new("UPDATE domains SET catch_all = ");
        query.push(", catch_all = ");
        query.push_bind(catch_all);
        if user.admin {
            // Only admins can edit the domain itself
            query.push(", domain = ");
            query.push_bind(domain);
        }
        query.push(", public = ");
        query.push_bind(public);
        query.push(", active = ");
        query.push_bind(active);
        query.push(", owner = ");
        query.push_bind(owner);
        query.push(" WHERE domain = ");
        query.push_bind(old_domain);
        if !user.admin {
            query.push(" AND owner = ");
            query.push_bind(&user.username);
        }

        query.build().execute(&pool).await.map(|_| ())?;
    } else {
        sqlx::query("INSERT INTO domains (domain, catch_all, public, active, owner) VALUES (?, ?, ?, ?, ?)")
            .bind(domain)
            .bind(catch_all)
            .bind(public)
            .bind(active)
            .bind(owner)
            .execute(&pool)
            .await
            .map(|_| ())?;
    }

    Ok(())
}

#[server]
pub async fn update_domain_public_and_active(domain: String, public: bool, active: bool) -> Result<(), ServerFnError> {
    let user = crate::auth::auth_user().await?;

    // Only admins may create public domains
    let public = public && user.admin;

    let mut query = QueryBuilder::new("UPDATE domains SET public = ");
    query.push_bind(public);
    query.push(", active = ");
    query.push_bind(active);
    query.push(" WHERE domain = ");
    query.push_bind(domain);

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
pub struct DomainTableDataProvider {
    sort: VecDeque<(usize, ColumnSort)>,
    pub search: RwSignal<String>,
}

impl TableDataProvider<Domain> for DomainTableDataProvider {
    async fn get_rows(&self, range: Range<usize>) -> Result<(Vec<Domain>, Range<usize>), String> {
        list_domains(DomainQuery {
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
        domain_count().await.ok()
    }

    fn set_sorting(&mut self, sorting: &VecDeque<(usize, ColumnSort)>) {
        self.sort = sorting.clone();
    }

    fn track(&self) {
        self.search.track();
    }
}

#[component]
pub fn Domains(user: User) -> impl IntoView {
    let mut rows = DomainTableDataProvider::default();
    let default_sorting = VecDeque::from([(5, ColumnSort::Descending)]);
    rows.set_sorting(&default_sorting);
    let sorting = create_rw_signal(default_sorting);

    let reload_controller = ReloadController::default();
    let on_input = use_debounce_fn_with_arg(move |value| rows.search.set(value), 300.0);
    let (count, set_count) = create_signal(0);

    let delete_modal_domain = create_rw_signal(None);
    let edit_modal_domain = create_rw_signal(None);

    let (edit_modal_input_domain, set_edit_modal_input_domain) = create_signal("".to_string());
    let (edit_modal_input_catchall, set_edit_modal_input_catchall) = create_signal("".to_string());
    let (edit_modal_input_public, set_edit_modal_input_public) = create_signal(true);
    let (edit_modal_input_active, set_edit_modal_input_active) = create_signal(true);
    let (edit_modal_input_owner, set_edit_modal_input_owner) = create_signal("".to_string());
    let edit_modal_open_with = Callback::new(move |edit_domain: Option<Domain>| {
        edit_modal_domain.set(Some(edit_domain.clone()));

        if let Some(edit_domain) = edit_domain {
            set_edit_modal_input_domain(edit_domain.domain.clone());
            set_edit_modal_input_catchall(edit_domain.catch_all.unwrap_or("".to_string()).clone());
            set_edit_modal_input_public(edit_domain.public);
            set_edit_modal_input_active(edit_domain.active);
            set_edit_modal_input_owner(edit_domain.owner.clone());
        } else {
            set_edit_modal_input_domain("".to_string());
            set_edit_modal_input_catchall("".to_string());
            set_edit_modal_input_public(user.admin);
            set_edit_modal_input_active(true);
            set_edit_modal_input_owner("".to_string());
        }
    });

    let errors = Vec::new;

    let on_edit = move |(data, on_error): (Option<Domain>, Callback<String>)| {
        spawn_local(async move {
            if let Err(e) = create_or_update_domain(
                data.map(|x| x.domain),
                edit_modal_input_domain.get_untracked(),
                edit_modal_input_catchall.get_untracked(),
                edit_modal_input_public.get_untracked(),
                edit_modal_input_active.get_untracked(),
                edit_modal_input_owner.get_untracked(),
            )
            .await
            {
                on_error(e.to_string())
            } else {
                reload_controller.reload();
                edit_modal_domain.set(None);
            }
        });
    };

    let on_row_change = move |ev: ChangeEvent<Domain>| {
        spawn_local(async move {
            if let Err(e) = update_domain_public_and_active(
                ev.changed_row.domain.clone(),
                ev.changed_row.public,
                ev.changed_row.active,
            )
            .await
            {
                error!(
                    "Failed to update public or active status of {}: {}",
                    ev.changed_row.domain, e
                );
            }
            reload_controller.reload();
        });
    };

    #[allow(unused_variables, non_snake_case)]
    let domain_row_renderer = move |class: Signal<String>,
                                    row: Domain,
                                    index: usize,
                                    selected: Signal<bool>,
                                    on_select: EventHandler<MouseEvent>,
                                    on_change: EventHandler<ChangeEvent<Domain>>| {
        let delete_domain = row.domain.clone();
        let edit_domain = row.clone();
        view! {
            <tr class=class on:click=move |mouse_event| on_select.run(mouse_event)>
                {row.render_row(index, on_change)}
                <td class="w-1 px-4 py-2 whitespace-nowrap text-ellipsis">
                    <div class="inline-flex items-center rounded-md">
                        <button
                            class="text-gray-800 dark:text-zinc-100 hover:text-white dark:hover:text-black bg-white dark:bg-black hover:bg-blue-600 dark:hover:bg-blue-500 transition-all border-[1.5px] border-gray-200 dark:border-zinc-800 rounded-l-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| edit_modal_open_with(Some(edit_domain.clone()))
                        >
                            <Icon icon=icondata::FiEdit class="w-5 h-5"/>
                        </button>
                        <button
                            class="text-gray-800 dark:text-zinc-100 hover:text-white dark:hover:text-black bg-white dark:bg-black hover:bg-red-600 dark:hover:bg-red-500 transition-all border-l-0 border-[1.5px] border-gray-200 dark:border-zinc-800 rounded-r-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| {
                                delete_modal_domain.set(Some(delete_domain.clone()));
                            }
                            disabled=move || !user.admin
                        >

                            <Icon icon=icondata::FiTrash2 class="w-5 h-5"/>
                        </button>
                    </div>
                </td>
            </tr>
        }
    };

    view! {
        <div class="h-full flex-1 flex-col mt-12">
            <div class="flex items-center justify-between space-y-2 mb-4">
                <h2 class="text-4xl font-bold">Domains</h2>
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
                                row_renderer=domain_row_renderer
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
            data=delete_modal_domain
            text="Are you sure you want to delete this domain? This action cannot be undone.".into_view()
            on_confirm=move |data| {
                spawn_local(async move {
                    if let Err(e) = delete_domain(data).await {
                        error!("Failed to delete domain: {}", e);
                    } else {
                        reload_controller.reload();
                    }
                    delete_modal_domain.set(None);
                });
            }
        />

        <EditModal
            data=edit_modal_domain
            what="Domain".to_string()
            get_title=move |x| { &x.domain }
            on_confirm=on_edit
            errors
        >
            <div class="flex flex-col gap-2">
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="domain"
                >
                    Domain
                </label>
                <input
                    class="flex flex-none w-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    type="text"
                    placeholder="example.com"
                    on:input=move |ev| set_edit_modal_input_domain(event_target_value(&ev))
                    prop:value=edit_modal_input_domain
                    disabled=move || !user.admin
                />
            </div>
            <div class="flex flex-col gap-2">
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="catchall"
                >
                    Catch All
                </label>
                <input
                    class="flex flex-none w-full rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 bg-transparent dark:bg-transparent text-sm p-2.5 transition-all placeholder:text-gray-500 dark:placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    type="text"
                    placeholder="catch-all@example.com"
                    on:input=move |ev| set_edit_modal_input_catchall(event_target_value(&ev))
                    prop:value=edit_modal_input_catchall
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
                    placeholder=move || user.username.clone()
                    on:input=move |ev| set_edit_modal_input_owner(event_target_value(&ev))
                    prop:value=edit_modal_input_owner
                    disabled=move || !user.admin
                />
            </div>
            <Show when=move || user.admin>
                <div class="flex flex-row gap-2 mt-2 items-center">
                    <input
                        id="public"
                        class="w-4 h-4 bg-transparent dark:bg-transparent text-blue-600 border-[1.5px] border-gray-200 dark:border-zinc-800 rounded checked:bg-blue-600 dark:checked:bg-blue-600 dark:bg-blue-600 focus:ring-ring focus:ring-4 transition-all"
                        type="checkbox"
                        on:change=move |ev| set_edit_modal_input_public(event_target_checked(&ev))
                        prop:checked=edit_modal_input_public
                    />
                    <label
                        class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                        for="public"
                    >
                        Public
                    </label>
                </div>
            </Show>
            <div class="flex flex-row gap-2 mt-2 items-center">
                <input
                    id="domains_active"
                    class="w-4 h-4 bg-transparent dark:bg-transparent text-blue-600 border-[1.5px] border-gray-200 dark:border-zinc-800 rounded checked:bg-blue-600 dark:checked:bg-blue-600 dark:bg-blue-600 focus:ring-ring focus:ring-4 transition-all"
                    type="checkbox"
                    on:change=move |ev| set_edit_modal_input_active(event_target_checked(&ev))
                    prop:checked=edit_modal_input_active
                />
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="domains_active"
                >
                    Active
                </label>
            </div>
        </EditModal>
    }
}
