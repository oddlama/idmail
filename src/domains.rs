use std::collections::VecDeque;
use std::ops::Range;

use crate::utils::{Modal, Select};
use crate::utils::{SliderRenderer, THeadCellRenderer, TailwindClassesPreset, TimediffRenderer};

use chrono::{DateTime, Utc};
use leptos::{ev::MouseEvent, html::Dialog, logging::error, *};
use leptos_icons::Icon;
use leptos_struct_table::*;
use leptos_use::use_debounce_fn_with_arg;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use sqlx::{QueryBuilder, Row};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TableRow)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[table(sortable, classes_provider = TailwindClassesPreset, thead_cell_renderer = THeadCellRenderer)]
pub struct Domain {
    #[table(class = "w-40")]
    pub domain: String,
    #[table(class = "w-40")]
    pub owner: String,
    pub catch_all: Option<String>,
    #[table(class = "w-1", renderer = "SliderRenderer")]
    pub public: bool,
    #[table(class = "w-1", renderer = "SliderRenderer")]
    pub active: bool,
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
pub async fn allowed_domains() -> Result<Vec<String>, ServerFnError> {
    let user = crate::auth::get_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Unauthorized"))?;

    let mut query = QueryBuilder::new("SELECT domain FROM domains");
    query.push(" WHERE public = TRUE OR owner = ?");
    query.push_bind(&user.username);

    let pool = crate::database::ssr::pool()?;
    Ok(query.build_query_scalar::<String>().fetch_all(&pool).await?)
}

#[server]
pub async fn list_domains(query: DomainQuery) -> Result<Vec<Domain>, ServerFnError> {
    let user = crate::auth::get_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Unauthorized"))?;

    let DomainQuery { sort, range, search } = query;

    let mut query = QueryBuilder::new("SELECT * FROM domains");
    if !search.is_empty() {
        query.push(" WHERE domain LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%') OR owner LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%') OR catch_all LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%')");
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
    let user = crate::auth::get_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Unauthorized"))?;

    let pool = crate::database::ssr::pool()?;
    let count: i64 = sqlx::query("SELECT COUNT(*) FROM domains")
        .fetch_one(&pool)
        .await?
        .get(0);

    Ok(count as usize)
}

#[server]
pub async fn create_or_update_domain(
    old_domain: Option<String>,
    domain: String,
    owner: String,
    catch_all: String,
    public: bool,
) -> Result<(), ServerFnError> {
    let user = crate::auth::get_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Unauthorized"))?;
    let pool = crate::database::ssr::pool()?;
    // TODO: FIXME: AAA auth
    // TODO: FIXME: duplicate detect
    // TODO: FIXME: invalid detect (empty, @@, ...)

    if let Some(old_domain) = old_domain {
        sqlx::query("UPDATE domains SET domain = ?, owner = ?, catch_all = ?, public = ? WHERE domain = ?")
            .bind(domain)
            .bind(owner)
            .bind(catch_all)
            .bind(public)
            .bind(old_domain)
            .execute(&pool)
            .await
            .map(|_| ())?;
    } else {
        sqlx::query("INSERT INTO domains (domain, owner, catch_all, public) VALUES (?, ?, ?, ?)")
            .bind(domain)
            .bind(owner)
            .bind(catch_all)
            .bind(public)
            .execute(&pool)
            .await
            .map(|_| ())?;
    }

    Ok(())
}

#[server]
pub async fn delete_domain(domain: String) -> Result<(), ServerFnError> {
    let pool = crate::database::ssr::pool()?;

    sqlx::query("DELETE FROM domains WHERE domain = $1")
        .bind(domain)
        .execute(&pool)
        .await
        .map(|_| ())?;
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

#[server]
pub async fn update_domain_public_and_active(domain: String, public: bool, active: bool) -> Result<(), ServerFnError> {
    let user = crate::auth::get_user().await?;
    let pool = crate::database::ssr::pool()?;
    // TODO: FIXME: AAA
    // TODO: FIXME: AAA auth

    sqlx::query("UPDATE domains SET public = ?, active = ? WHERE domain = ?")
        .bind(public)
        .bind(active)
        .bind(domain)
        .execute(&pool)
        .await
        .map(|_| ())?;

    Ok(())
}

#[component]
pub fn Domains() -> impl IntoView {
    let mut rows = DomainTableDataProvider::default();
    let default_sorting = VecDeque::from([(5, ColumnSort::Descending)]);
    rows.set_sorting(&default_sorting);
    let sorting = create_rw_signal(default_sorting);

    let reload_controller = ReloadController::default();
    let on_input = use_debounce_fn_with_arg(move |value| rows.search.set(value), 300.0);
    let (count, set_count) = create_signal(0);

    let (delete_modal_domain, set_delete_modal_domain) = create_signal(None);
    let (delete_modal_open, set_delete_modal_open) = create_signal(false);
    let (delete_modal_waiting, set_delete_modal_waiting) = create_signal(false);
    let delete_modal_elem = create_node_ref::<Dialog>();
    let delete_modal_close = Callback::new(move |()| {
        delete_modal_elem
            .get_untracked()
            .expect("delete dialog to have been created")
            .close();
    });

    let (edit_modal_domain, set_edit_modal_domain) = create_signal(None);
    let (edit_modal_open, set_edit_modal_open) = create_signal(false);
    let (edit_modal_waiting, set_edit_modal_waiting) = create_signal(false);
    let edit_modal_elem = create_node_ref::<Dialog>();
    let edit_modal_close = Callback::new(move |()| {
        edit_modal_elem
            .get_untracked()
            .expect("edit dialog to have been created")
            .close();
    });

    let (edit_modal_input_domain, set_edit_modal_input_domain) = create_signal("".to_string());
    let (edit_modal_input_owner, set_edit_modal_input_owner) = create_signal("".to_string());
    let (edit_modal_input_catchall, set_edit_modal_input_catchall) = create_signal("".to_string());
    let (edit_modal_input_public, set_edit_modal_input_public) = create_signal(true);
    let edit_modal_open_with = Callback::new(move |edit_domain: Option<Domain>| {
        set_edit_modal_domain(edit_domain.clone());

        if let Some(edit_domain) = edit_domain {
            set_edit_modal_domain(Some(edit_domain.clone()));
            set_edit_modal_input_domain(edit_domain.domain.clone());
            set_edit_modal_input_owner(edit_domain.owner.clone());
            set_edit_modal_input_catchall(edit_domain.catch_all.unwrap_or("".to_string()).clone());
            set_edit_modal_input_public(edit_domain.public);
        } else {
            set_edit_modal_input_domain("".to_string());
            set_edit_modal_input_owner("".to_string());
            set_edit_modal_input_catchall("".to_string());
            set_edit_modal_input_public(true);
        }
        set_edit_modal_waiting(false);
        set_edit_modal_open(true);
    });

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
                            class="text-gray-800 hover:text-white bg-white hover:bg-blue-600 transition-all border-[1.5px] border-gray-200 rounded-l-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| edit_modal_open_with(Some(edit_domain.clone()))
                        >
                            <Icon icon=icondata::FiEdit class="w-5 h-5" />
                        </button>
                        <button
                            class="text-gray-800 hover:text-white bg-white hover:bg-red-600 transition-all border-l-0 border-[1.5px] border-gray-200 rounded-r-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| {
                                set_delete_modal_domain(Some(delete_domain.clone()));
                                set_delete_modal_waiting(false);
                                set_delete_modal_open(true);
                            }
                        >
                            <Icon icon=icondata::FiTrash2 class="w-5 h-5" />
                        </button>
                    </div>
                </td>
            </tr>
        }
    };

    view! {
        <div class="overflow-hidden bg-background">
            <div class="h-full flex-1 flex-col space-y-12 p-4 md:p-12">
                <div class="flex items-center justify-between space-y-2">
                    <div>
                        <h2 class="text-4xl font-bold">Domains</h2>
                        <p class="text-xl text-muted-foreground">coolmailbox@somemail.com</p>
                    </div>
                </div>
                <div class="space-y-4">
                    <div class="flex flex-wrap items-center justify-between">
                        <input
                            class="flex flex-none rounded-lg border-[1.5px] border-input bg-transparent text-base p-2.5 me-2 mb-2 w-full md:w-[360px] lg:w-[520px] transition-all placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                            type="search"
                            placeholder="Search"
                            value=rows.search
                            on:input=move |e| {
                                on_input(event_target_value(&e));
                            }
                        />

                        <button
                            type="button"
                            class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-base text-white py-2.5 px-4 me-2 mb-2 transition-all rounded-lg focus:ring-4 bg-blue-600 hover:bg-blue-500 focus:ring-blue-300"
                            on:click=move |_| edit_modal_open_with(None)
                        >
                            <Icon icon=icondata::FiPlus class="w-6 h-6 me-2" />
                            New
                        </button>
                        <div class="flex flex-1"></div>
                        <div class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-base text-right px-4">
                            {count} " results"
                        </div>
                    </div>

                    <div class="rounded-lg border-[1.5px] text-base flex flex-col overflow-hidden">
                        <div class="overflow-auto grow min-h-0">
                            <table class="table-auto text-left w-full">
                                <TableContent
                                    rows
                                    sorting=sorting
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
        </div>
        <Modal open=delete_modal_open dialog_el=delete_modal_elem>
            <div class="relative p-4 transform overflow-hidden rounded-lg bg-white text-left transition-all sm:w-full sm:max-w-lg">
                <div class="bg-white py-3">
                    <div class="sm:flex sm:items-start">
                        <div class="mx-auto flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-full bg-red-100 sm:mx-0 sm:h-10 sm:w-10">
                            <Icon icon=icondata::AiWarningFilled class="w-6 h-6 text-red-600" />
                        </div>
                        <div class="mt-3 text-center sm:ml-4 sm:mt-0 sm:text-left">
                            <h3 class="text-xl font-semibold leading-6 text-gray-900">
                                "Delete " {delete_modal_domain}
                            </h3>
                            <div class="mt-2">
                                <p class="text-base text-gray-500">
                                    "Are you sure you want to delete this domain? This action cannot be undone."
                                </p>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="flex flex-col-reverse gap-3 sm:flex-row-reverse">
                    <button
                        type="button"
                        class="inline-flex w-full min-w-20 justify-center rounded-lg transition-all bg-white px-3 py-2 font-semibold text-gray-900 focus:ring-4 focus:ring-gray-300 border-[1.5px] border-gray-300 hover:bg-gray-100 sm:w-auto"
                        on:click=move |_ev| {
                            delete_modal_close(());
                        }
                    >

                        Cancel
                    </button>
                    <button
                        type="button"
                        disabled=delete_modal_waiting
                        class="inline-flex w-full min-w-20 justify-center items-center rounded-lg transition-all px-3 py-2 bg-red-600 hover:bg-red-500 font-semibold text-white focus:ring-4 focus:ring-red-300 sm:w-auto"
                        class=("!bg-red-500", delete_modal_waiting)
                        on:click=move |_ev| {
                            if !delete_modal_waiting() {
                                let addr = delete_modal_domain().expect("no domain to delete");
                                let delete_modal_close = delete_modal_close.clone();
                                set_delete_modal_waiting(true);
                                spawn_local(async move {
                                    if let Err(e) = delete_domain(addr).await {
                                        error!("Failed to delete: {}", e);
                                    } else {
                                        reload_controller.reload();
                                    }
                                    delete_modal_close(());
                                });
                            }
                        }
                    >

                        <Show when=delete_modal_waiting>
                            <Icon icon=icondata::CgSpinner class="inline w-4 h-4 me-2 text-red-900 animate-spin" />
                        </Show>
                        Delete
                    </button>
                </div>
            </div>
        </Modal>
        <Modal open=edit_modal_open dialog_el=edit_modal_elem>
            <div class="relative p-4 transform overflow-hidden rounded-lg bg-white text-left transition-all w-full sm:min-w-[512px]">
                <h3 class="text-xl mt-2 mb-4 font-semibold leading-6 text-gray-900">
                    {move || {
                        if let Some(domain) = edit_modal_domain() {
                            format!("Edit {}", domain.domain)
                        } else {
                            "New domain".to_string()
                        }
                    }}

                </h3>
                <div class="flex flex-col gap-3">
                    <div class="flex flex-col gap-2">
                        <label class="font-medium" for="domain">
                            Domain
                        </label>
                        <input
                            class="flex flex-none w-full rounded-lg border-[1.5px] border-input bg-transparent text-sm p-2.5 transition-all placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                            type="text"
                            placeholder="example.com"
                            on:input=move |ev| set_edit_modal_input_domain(event_target_value(&ev))
                            prop:value=edit_modal_input_domain
                        />
                    </div>
                    <div class="flex flex-col gap-2">
                        <label class="font-medium" for="owner">
                            Owner
                        </label>
                        <input
                            class="flex flex-none w-full rounded-lg border-[1.5px] border-input bg-transparent text-sm p-2.5 transition-all placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                            type="text"
                            placeholder="admin"
                            on:input=move |ev| set_edit_modal_input_owner(event_target_value(&ev))
                            prop:value=edit_modal_input_owner
                            disabled
                        />
                    </div>
                    <div class="flex flex-col gap-2">
                        <label class="font-medium" for="catchall">
                            Catch All
                        </label>
                        <input
                            class="flex flex-none w-full rounded-lg border-[1.5px] border-input bg-transparent text-sm p-2.5 transition-all placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                            type="text"
                            placeholder="catch-all@example.com"
                            on:input=move |ev| set_edit_modal_input_catchall(event_target_value(&ev))
                            prop:value=edit_modal_input_catchall
                        />
                    </div>
                    // TODO: public
                    // TODO: active
                    <div class="flex flex-col-reverse gap-3 sm:flex-row-reverse">
                        <button
                            type="button"
                            class="inline-flex w-full min-w-20 justify-center rounded-lg transition-all bg-white px-3 py-2 font-semibold text-gray-900 focus:ring-4 focus:ring-gray-300 border-[1.5px] border-gray-300 hover:bg-gray-100 sm:w-auto"
                            on:click=move |_ev| {
                                edit_modal_close(());
                            }
                        >

                            Cancel
                        </button>
                        <button
                            type="button"
                            disabled=edit_modal_waiting
                            class="inline-flex w-full min-w-20 justify-center items-center rounded-lg transition-all px-3 py-2 bg-blue-600 hover:bg-blue-500 font-semibold text-white focus:ring-4 focus:ring-blue-300 sm:w-auto"
                            class=("!bg-blue-500", edit_modal_waiting)
                            on:click=move |_ev| {
                                if !edit_modal_waiting() {
                                    let domain = edit_modal_domain();
                                    let edit_modal_close = edit_modal_close.clone();
                                    set_edit_modal_waiting(true);
                                    spawn_local(async move {
                                        if let Err(e) = create_or_update_domain(
                                                domain.map(|x| x.domain),
                                                edit_modal_input_domain(),
                                                edit_modal_input_owner(),
                                                edit_modal_input_catchall(),
                                                edit_modal_input_public(),
                                            )
                                            .await
                                        {
                                            error!("Failed to create/update: {}", e);
                                        } else {
                                            reload_controller.reload();
                                        }
                                        edit_modal_close(());
                                    });
                                }
                            }
                        >

                            <Show when=edit_modal_waiting>
                                <Icon icon=icondata::CgSpinner class="inline w-4 h-4 me-2 text-blue-900 animate-spin" />
                            </Show>
                            Save
                        </button>
                    </div>
                </div>
            </div>
        </Modal>
    }
}
