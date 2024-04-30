use std::collections::VecDeque;
use std::ops::Range;

use crate::utils::{Modal, Select};
use crate::utils::{SliderRenderer, THeadCellRenderer, TailwindClassesPreset, TimediffRenderer};

use chrono::{DateTime, Utc};
use leptos::leptos_dom::is_browser;
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

#[server]
pub async fn list_aliases(query: AliasQuery) -> Result<Vec<Alias>, ServerFnError> {
    let user = crate::auth::get_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Unauthorized"))?;
    let AliasQuery { sort, range, search } = query;

    let mut query = QueryBuilder::new("SELECT * FROM aliases");
    if !search.is_empty() {
        query.push(" WHERE address LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%') OR comment LIKE concat('%', ");
        query.push_bind(&search);
        query.push(", '%')");
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

#[server]
pub async fn alias_count() -> Result<usize, ServerFnError> {
    let user = crate::auth::get_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Unauthorized"))?;
    let pool = crate::database::ssr::pool()?;
    let count: i64 = sqlx::query("SELECT COUNT(*) FROM aliases")
        .fetch_one(&pool)
        .await?
        .get(0);

    Ok(count as usize)
}

#[server]
pub async fn delete_alias(address: String) -> Result<(), ServerFnError> {
    let user = crate::auth::get_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Unauthorized"))?;
    let pool = crate::database::ssr::pool()?;

    sqlx::query("DELETE FROM aliases WHERE address = $1")
        .bind(address)
        .execute(&pool)
        .await
        .map(|_| ())?;
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
        alias_count().await.ok()
    }

    fn set_sorting(&mut self, sorting: &VecDeque<(usize, ColumnSort)>) {
        self.sort = sorting.clone();
    }

    fn track(&self) {
        self.search.track();
    }
}

#[server]
pub async fn update_alias_active(address: String, active: bool) -> Result<(), ServerFnError> {
    let user = crate::auth::get_user().await?;
    let pool = crate::database::ssr::pool()?;
    // TODO: FIXME: AAA
    // TODO: FIXME: AAA auth

    sqlx::query("UPDATE aliases SET active = ? WHERE address = ?")
        .bind(active)
        .bind(address)
        .execute(&pool)
        .await
        .map(|_| ())?;

    Ok(())
}

#[server]
pub async fn create_or_update_alias(
    old_address: Option<String>,
    alias: String,
    domain: String,
    target: String,
    comment: String,
) -> Result<(), ServerFnError> {
    let user = crate::auth::get_user().await?;
    let pool = crate::database::ssr::pool()?;
    // TODO: FIXME: AAA auth
    // TODO: FIXME: duplicate detect
    // TODO: FIXME: invalid detect (empty, @@, ...)

    let address = format!("{alias}@{domain}");
    if let Some(old_address) = old_address {
        sqlx::query("UPDATE aliases SET address = ?, target = ?, comment = ? WHERE address = ?")
            .bind(address)
            .bind(target)
            .bind(comment)
            .bind(old_address)
            .execute(&pool)
            .await
            .map(|_| ())?;
    } else {
        sqlx::query("INSERT INTO aliases (address, target, comment) VALUES (?, ?, ?)")
            .bind(address)
            .bind(target)
            .bind(comment)
            .execute(&pool)
            .await
            .map(|_| ())?;
    }

    Ok(())
}

#[component]
pub fn Aliases() -> impl IntoView {
    let mut rows = AliasTableDataProvider::default();
    let default_sorting = VecDeque::from([(6, ColumnSort::Descending)]);
    rows.set_sorting(&default_sorting);
    let sorting = create_rw_signal(default_sorting);

    let reload_controller = ReloadController::default();
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

    if is_browser() {
        refresh_domains();
    }

    let (delete_modal_alias, set_delete_modal_alias) = create_signal(None);
    let (delete_modal_open, set_delete_modal_open) = create_signal(false);
    let (delete_modal_waiting, set_delete_modal_waiting) = create_signal(false);
    let delete_modal_elem = create_node_ref::<Dialog>();
    let delete_modal_close = Callback::new(move |()| {
        delete_modal_elem
            .get_untracked()
            .expect("delete dialog to have been created")
            .close();
    });

    let (edit_modal_alias, set_edit_modal_alias) = create_signal(None);
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
    let (edit_modal_input_alias, set_edit_modal_input_alias) = create_signal("".to_string());
    let (edit_modal_input_target, set_edit_modal_input_target) = create_signal("".to_string());
    let (edit_modal_input_comment, set_edit_modal_input_comment) = create_signal("".to_string());
    let edit_modal_open_with = Callback::new(move |edit_alias: Option<Alias>| {
        refresh_domains();
        set_edit_modal_alias(edit_alias.clone());

        let allowed_domains = allowed_domains.get();
        if let Some(edit_alias) = edit_alias {
            let (alias, domain) = match edit_alias.address.split_once('@') {
                Some((alias, domain)) => (alias.to_string(), domain.to_string()),
                None => (edit_alias.address.clone(), "".to_string()),
            };
            set_edit_modal_alias(Some(edit_alias.clone()));
            set_edit_modal_input_alias(alias.to_string());
            if !allowed_domains.contains(&domain) {
                set_edit_modal_input_domain(allowed_domains.first().cloned().unwrap_or("".to_string()));
            } else {
                set_edit_modal_input_domain(domain);
            }
            set_edit_modal_input_target(edit_alias.target.clone());
            set_edit_modal_input_comment(edit_alias.comment.clone());
        } else {
            // Only set the input domain if the current one is not in the list
            // of allowed domains. This allows users to keep the old value
            // between alias creations, making it easier to create multiple
            // aliases on the same domain.
            set_edit_modal_input_alias("".to_string());
            if !allowed_domains.contains(&edit_modal_input_domain()) {
                set_edit_modal_input_domain(allowed_domains.first().cloned().unwrap_or("".to_string()));
            }
            // TODO set from user
            //set_edit_modal_input_target("".to_string());
            set_edit_modal_input_comment("".to_string());
        }
        set_edit_modal_waiting(false);
        set_edit_modal_open(true);
    });

    let on_row_change = move |ev: ChangeEvent<Alias>| {
        spawn_local(async move {
            if let Err(e) = update_alias_active(ev.changed_row.address.clone(), ev.changed_row.active).await {
                error!("Failed to update active status of {}: {}", ev.changed_row.address, e);
            }
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
                            class="text-gray-800 hover:text-white bg-white hover:bg-blue-600 transition-all border-[1.5px] border-gray-200 rounded-l-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| edit_modal_open_with(Some(edit_alias.clone()))
                        >
                            <Icon icon=icondata::FiEdit class="w-5 h-5"/>
                        </button>
                        <button
                            class="text-gray-800 hover:text-white bg-white hover:bg-red-600 transition-all border-l-0 border-[1.5px] border-gray-200 rounded-r-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| {
                                set_delete_modal_alias(Some(delete_address.clone()));
                                set_delete_modal_waiting(false);
                                set_delete_modal_open(true);
                            }
                        >

                            <Icon icon=icondata::FiTrash2 class="w-5 h-5"/>
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
                        <h2 class="text-4xl font-bold">Aliases</h2>
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
                            <Icon icon=icondata::FiPlus class="w-6 h-6 me-2"/>
                            New
                        </button>
                        <button
                            type="button"
                            class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-base text-white py-2.5 px-4 me-2 mb-2 transition-all rounded-lg focus:ring-4 bg-green-600 hover:bg-green-500 focus:ring-green-300"
                        >
                            <Icon icon=icondata::FaDiceSolid class="w-6 h-6 me-2"/>
                            New Random
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
        </div>
        <Modal open=delete_modal_open dialog_el=delete_modal_elem>
            <div class="relative p-4 transform overflow-hidden rounded-lg bg-white text-left transition-all sm:w-full sm:max-w-lg">
                <div class="bg-white py-3">
                    <div class="sm:flex sm:items-start">
                        <div class="mx-auto flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-full bg-red-100 sm:mx-0 sm:h-10 sm:w-10">
                            <Icon icon=icondata::AiWarningFilled class="w-6 h-6 text-red-600"/>
                        </div>
                        <div class="mt-3 text-center sm:ml-4 sm:mt-0 sm:text-left">
                            <h3 class="text-2xl tracking-tight font-semibold text-gray-900">
                                "Delete " {delete_modal_alias}
                            </h3>
                            <div class="mt-2">
                                <p class="text-base text-gray-500">
                                    "Are you sure you want to delete this alias? This action cannot be undone."
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
                                let addr = delete_modal_alias().expect("no alias to delete");
                                let delete_modal_close = delete_modal_close.clone();
                                set_delete_modal_waiting(true);
                                spawn_local(async move {
                                    if let Err(e) = delete_alias(addr).await {
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
                            <Icon icon=icondata::CgSpinner class="inline w-4 h-4 me-2 text-red-900 animate-spin"/>
                        </Show>
                        Delete
                    </button>
                </div>
            </div>
        </Modal>
        <Modal open=edit_modal_open dialog_el=edit_modal_elem>
            <div class="relative p-4 transform overflow-hidden rounded-lg bg-white text-left transition-all w-full sm:min-w-[512px]">
                <h3 class="text-2xl tracking-tight mt-2 mb-4 font-semibold text-gray-900">
                    {move || {
                        if let Some(alias) = edit_modal_alias() {
                            format!("Edit {}", alias.address)
                        } else {
                            "New alias".to_string()
                        }
                    }}

                </h3>
                <div class="flex flex-col gap-3">
                    <div class="flex flex-col sm:flex-row">
                        <div class="flex flex-1 flex-col gap-2">
                            <label class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70" for="alias">
                                Alias
                            </label>
                            <div class="flex flex-row">
                                <input
                                    class="flex sm:min-w-32 flex-1 rounded-lg border-[1.5px] border-input bg-transparent text-sm p-2.5 transition-all placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                                    type="email"
                                    placeholder="alias"
                                    on:input=move |ev| set_edit_modal_input_alias(event_target_value(&ev))
                                    prop:value=edit_modal_input_alias
                                />
                                <span class="inline-flex flex-none text-base items-center mx-2">@</span>
                            </div>
                        </div>
                        <div class="flex flex-col gap-2">
                            <label class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70" for="domain">
                                Domain
                            </label>
                            <Select
                                class="w-full rounded-lg border-[1.5px] border-input bg-transparent text-sm p-2.5 transition-all focus:ring-4 focus:ring-blue-300"
                                choices=allowed_domains
                                value=edit_modal_input_domain
                                set_value=set_edit_modal_input_domain
                            />
                        </div>
                    </div>
                    <div class="flex flex-col gap-2">
                        <label class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70" for="target">
                            Target
                        </label>
                        <input
                            class="flex flex-none w-full rounded-lg border-[1.5px] border-input bg-transparent text-sm p-2.5 transition-all placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                            type="email"
                            // TODO value from user
                            placeholder="target@example.com"
                            on:input=move |ev| set_edit_modal_input_target(event_target_value(&ev))
                            prop:value=edit_modal_input_target
                            disabled
                        />
                    </div>
                    <div class="flex flex-col gap-2">
                        <label class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70" for="comment">
                            Comment
                        </label>
                        <input
                            class="flex flex-none w-full rounded-lg border-[1.5px] border-input bg-transparent text-sm p-2.5 transition-all placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                            type="text"
                            placeholder="Comment"
                            on:input=move |ev| set_edit_modal_input_comment(event_target_value(&ev))
                            prop:value=edit_modal_input_comment
                        />
                    </div>
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
                                    let alias = edit_modal_alias();
                                    let edit_modal_close = edit_modal_close.clone();
                                    set_edit_modal_waiting(true);
                                    spawn_local(async move {
                                        if let Err(e) = create_or_update_alias(
                                                alias.map(|x| x.address),
                                                edit_modal_input_alias(),
                                                edit_modal_input_domain(),
                                                edit_modal_input_target(),
                                                edit_modal_input_comment(),
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
                                <Icon icon=icondata::CgSpinner class="inline w-4 h-4 me-2 text-blue-900 animate-spin"/>
                            </Show>
                            Save
                        </button>
                    </div>
                </div>
            </div>
        </Modal>
    }
}
