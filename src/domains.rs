use std::collections::VecDeque;
use std::ops::Range;

use crate::utils::{Modal, Select};
use crate::utils::{SliderRenderer, THeadCellRenderer, TailwindClassesPreset, TimediffRenderer};

use chrono::{DateTime, Utc};
use leptos::{ev::MouseEvent, html::Dialog, logging::error, *};
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

                            <span>
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                    class="w-6 h-6"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0115.75 21H5.25A2.25 2.25 0 013 18.75V8.25A2.25 2.25 0 015.25 6H10"
                                    ></path>
                                </svg>
                            </span>
                        </button>
                        <button
                            class="text-gray-800 hover:text-white bg-white hover:bg-red-600 transition-all border-l-0 border-[1.5px] border-gray-200 rounded-r-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| {
                                set_delete_modal_domain(Some(delete_domain.clone()));
                                set_delete_modal_waiting(false);
                                set_delete_modal_open(true);
                            }
                        >

                            <span>
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                    class="w-6 h-6"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0"
                                    ></path>
                                </svg>
                            </span>
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

                            <svg
                                class="w-6 h-6 me-2"
                                xmlns="http://www.w3.org/2000/svg"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <line x1="12" y1="5" x2="12" y2="19"></line>
                                <line x1="5" y1="12" x2="19" y2="12"></line>
                            </svg>
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
                            <svg
                                class="h-6 w-6 text-red-600"
                                fill="none"
                                viewBox="0 0 24 24"
                                stroke-width="1.5"
                                stroke="currentColor"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z"
                                ></path>
                            </svg>
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
                            <svg
                                aria-hidden="true"
                                role="status"
                                class="inline w-4 h-4 me-2 text-red-900 animate-spin"
                                viewBox="0 0 100 101"
                                fill="none"
                                xmlns="http://www.w3.org/2000/svg"
                            >
                                <path
                                    d="M100 50.5908C100 78.2051 77.6142 100.591 50 100.591C22.3858 100.591 0 78.2051 0 50.5908C0 22.9766 22.3858 0.59082 50 0.59082C77.6142 0.59082 100 22.9766 100 50.5908ZM9.08144 50.5908C9.08144 73.1895 27.4013 91.5094 50 91.5094C72.5987 91.5094 90.9186 73.1895 90.9186 50.5908C90.9186 27.9921 72.5987 9.67226 50 9.67226C27.4013 9.67226 9.08144 27.9921 9.08144 50.5908Z"
                                    fill="currentColor"
                                ></path>
                                <path
                                    d="M93.9676 39.0409C96.393 38.4038 97.8624 35.9116 97.0079 33.5539C95.2932 28.8227 92.871 24.3692 89.8167 20.348C85.8452 15.1192 80.8826 10.7238 75.2124 7.41289C69.5422 4.10194 63.2754 1.94025 56.7698 1.05124C51.7666 0.367541 46.6976 0.446843 41.7345 1.27873C39.2613 1.69328 37.813 4.19778 38.4501 6.62326C39.0873 9.04874 41.5694 10.4717 44.0505 10.1071C47.8511 9.54855 51.7191 9.52689 55.5402 10.0491C60.8642 10.7766 65.9928 12.5457 70.6331 15.2552C75.2735 17.9648 79.3347 21.5619 82.5849 25.841C84.9175 28.9121 86.7997 32.2913 88.1811 35.8758C89.083 38.2158 91.5421 39.6781 93.9676 39.0409Z"
                                    fill="#ffffff"
                                ></path>
                            </svg>
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
                                <svg
                                    aria-hidden="true"
                                    role="status"
                                    class="inline w-4 h-4 me-2 text-blue-900 animate-spin"
                                    viewBox="0 0 100 101"
                                    fill="none"
                                    xmlns="http://www.w3.org/2000/svg"
                                >
                                    <path
                                        d="M100 50.5908C100 78.2051 77.6142 100.591 50 100.591C22.3858 100.591 0 78.2051 0 50.5908C0 22.9766 22.3858 0.59082 50 0.59082C77.6142 0.59082 100 22.9766 100 50.5908ZM9.08144 50.5908C9.08144 73.1895 27.4013 91.5094 50 91.5094C72.5987 91.5094 90.9186 73.1895 90.9186 50.5908C90.9186 27.9921 72.5987 9.67226 50 9.67226C27.4013 9.67226 9.08144 27.9921 9.08144 50.5908Z"
                                        fill="currentColor"
                                    ></path>
                                    <path
                                        d="M93.9676 39.0409C96.393 38.4038 97.8624 35.9116 97.0079 33.5539C95.2932 28.8227 92.871 24.3692 89.8167 20.348C85.8452 15.1192 80.8826 10.7238 75.2124 7.41289C69.5422 4.10194 63.2754 1.94025 56.7698 1.05124C51.7666 0.367541 46.6976 0.446843 41.7345 1.27873C39.2613 1.69328 37.813 4.19778 38.4501 6.62326C39.0873 9.04874 41.5694 10.4717 44.0505 10.1071C47.8511 9.54855 51.7191 9.52689 55.5402 10.0491C60.8642 10.7766 65.9928 12.5457 70.6331 15.2552C75.2735 17.9648 79.3347 21.5619 82.5849 25.841C84.9175 28.9121 86.7997 32.2913 88.1811 35.8758C89.083 38.2158 91.5421 39.6781 93.9676 39.0409Z"
                                        fill="#ffffff"
                                    ></path>
                                </svg>
                            </Show>
                            Save
                        </button>
                    </div>
                </div>
            </div>
        </Modal>
    }
}
