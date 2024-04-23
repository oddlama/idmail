use leptos::logging::log;
use leptos::*;
use leptos_struct_table::*;
use leptos_struct_table::{ColumnSort, TableClassesProvider};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use sqlx::{QueryBuilder, Row};
use std::collections::VecDeque;
use std::ops::Range;

#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::auth::ssr::AuthSession;
    use leptos::*;
    use sqlx::SqlitePool;

    use super::Alias;

    pub fn pool() -> Result<SqlitePool, ServerFnError> {
        use_context::<SqlitePool>().ok_or_else(|| ServerFnError::ServerError("Pool missing.".into()))
    }

    pub fn auth() -> Result<AuthSession, ServerFnError> {
        use_context::<AuthSession>().ok_or_else(|| ServerFnError::ServerError("Auth session missing.".into()))
    }

    #[derive(sqlx::FromRow, Clone)]
    pub struct SqlAlias {
        address: String,
        target: String,
        comment: String,
        created_at: String,
        active: bool,
    }

    impl SqlAlias {
        pub async fn into_alias(self, _pool: &SqlitePool) -> Alias {
            Alias {
                address: self.address,
                target: self.target,
                comment: self.comment,
                created_at: self.created_at,
                active: self.active,
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct ClassesPreset;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TableRow)]
#[table(sortable, classes_provider = ClassesPreset, thead_cell_renderer = THeadCellRenderer)]
pub struct Alias {
    pub address: String,
    pub target: String,
    pub comment: String,
    pub created_at: String,
    pub active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AliasQuery {
    #[serde(default)]
    sort: VecDeque<(usize, ColumnSort)>,
    range: Range<usize>,
    name: String,
}

#[server]
pub async fn list_aliases(query: AliasQuery) -> Result<Vec<Alias>, ServerFnError> {
    use self::ssr::{pool, SqlAlias};
    use futures::future::join_all;
    let AliasQuery { sort, range, name } = query;

    let mut query = QueryBuilder::new("SELECT * FROM aliases");
    if !name.is_empty() {
        query.push(" WHERE address LIKE concat('%', ");
        query.push_bind(&name);
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

    let pool = pool()?;
    Ok(join_all(
        query
            .build_query_as::<SqlAlias>()
            .fetch_all(&pool)
            .await?
            .iter()
            .map(|x: &SqlAlias| x.clone().into_alias(&pool)),
    )
    .await)
}

#[server]
pub async fn alias_count() -> Result<usize, ServerFnError> {
    use self::ssr::pool;
    let pool = pool()?;
    let count: i64 = sqlx::query("SELECT COUNT(*) FROM aliases")
        .fetch_one(&pool)
        .await?
        .get(0);

    Ok(count as usize)
}

#[derive(Default)]
pub struct AliasTableDataProvider {
    sort: VecDeque<(usize, ColumnSort)>,
    pub search: RwSignal<String>,
}

impl TableDataProvider<Alias> for AliasTableDataProvider {
    async fn get_rows(&self, range: Range<usize>) -> Result<(Vec<Alias>, Range<usize>), String> {
        list_aliases(AliasQuery {
            name: self.search.get_untracked().trim().to_string(),
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
        log!("sorting: {:#?}", sorting);
        self.sort = sorting.clone();
    }

    fn track(&self) {
        self.search.track();
    }
}

impl TableClassesProvider for ClassesPreset {
    fn new() -> Self {
        Self
    }

    fn thead_row(&self, template_classes: &str) -> String {
        format!("{} {}", "text-xs", template_classes)
    }

    fn thead_cell(&self, _sort: ColumnSort, template_classes: &str) -> String {
        format!("h-10 px-2 text-left align-middle font-medium {}", template_classes)
    }

    fn thead_cell_inner(&self) -> String {
        "flex items-center".to_string()
    }

    fn row(&self, row_index: usize, _selected: bool, template_classes: &str) -> String {
        let bg_color = if row_index % 2 == 0 {
            "bg-white hover:bg-gray-100"
        } else {
            "bg-gray-50 hover:bg-gray-100"
        };

        format!("border-t last:border-0 {} {}", bg_color, template_classes)
    }

    fn loading_cell(&self, _row_index: usize, _col_index: usize, prop_class: &str) -> String {
        format!("{} {}", "p-2", prop_class)
    }

    fn loading_cell_inner(&self, _row_index: usize, _col_index: usize, prop_class: &str) -> String {
        format!(
            "animate-pulse h-2 bg-gray-400 rounded-full inline-block align-middle w-[calc(60%-2.5rem)] {}",
            prop_class
        )
    }

    fn cell(&self, template_classes: &str) -> String {
        format!(
            "{} {}",
            "p-2 whitespace-nowrap overflow-hidden text-ellipsis", template_classes
        )
    }
}

#[component]
pub fn THeadCellRenderer<F>(
    /// The class attribute for the head element. Generated by the classes provider.
    #[prop(into)]
    class: Signal<String>,
    /// The class attribute for the inner element. Generated by the classes provider.
    #[prop(into)]
    inner_class: String,
    /// The index of the column. Starts at 0 for the first column. The order of the columns is the same as the order of the fields in the struct.
    index: usize,
    /// The sort priority of the column. `None` if the column is not sorted. `0` means the column is the primary sort column.
    #[prop(into)]
    sort_priority: Signal<Option<usize>>,
    /// The sort direction of the column. See [`ColumnSort`].
    #[prop(into)]
    sort_direction: Signal<ColumnSort>,
    /// The event handler for the click event. Has to be called with [`TableHeadEvent`].
    on_click: F,
    children: Children,
) -> impl IntoView
where
    F: Fn(TableHeadEvent) + 'static,
{
    view! {
        <th class=class
            on:click=move |mouse_event| on_click(TableHeadEvent { index, mouse_event, })
        >
            <button type="button" class="inline-flex items-center justify-center whitespace-nowrap font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground rounded-md px-3 text-xs -ml-3 h-8">
                <span class=inner_class>
                    {children()}
                </span>
                {move || {
                    match (sort_priority(), sort_direction()) {
                        (Some(_prio), ColumnSort::Ascending) => view! {
                            <svg class="ml-2 h-4 w-4" width="24" height="24" viewBox="0 0 15 15" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                                <path fill-rule="evenodd" clip-rule="evenodd"
                                    d="M7.14645 2.14645C7.34171 1.95118 7.65829 1.95118 7.85355 2.14645L11.8536 6.14645C12.0488 6.34171 12.0488 6.65829 11.8536 6.85355C11.6583 7.04882 11.3417 7.04882 11.1464 6.85355L8 3.70711L8 12.5C8 12.7761 7.77614 13 7.5 13C7.22386 13 7 12.7761 7 12.5L7 3.70711L3.85355 6.85355C3.65829 7.04882 3.34171 7.04882 3.14645 6.85355C2.95118 6.65829 2.95118 6.34171 3.14645 6.14645L7.14645 2.14645Z"
                                    fill="currentColor">
                                </path>
                            </svg>
                        },
                        (Some(_prio), ColumnSort::Descending) => view! {
                            <svg class="ml-2 h-4 w-4" width="24" height="24" viewBox="0 0 15 15" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                                <path fill-rule="evenodd" clip-rule="evenodd"
                                    d="M7.5 2C7.77614 2 8 2.22386 8 2.5L8 11.2929L11.1464 8.14645C11.3417 7.95118 11.6583 7.95118 11.8536 8.14645C12.0488 8.34171 12.0488 8.65829 11.8536 8.85355L7.85355 12.8536C7.75979 12.9473 7.63261 13 7.5 13C7.36739 13 7.24021 12.9473 7.14645 12.8536L3.14645 8.85355C2.95118 8.65829 2.95118 8.34171 3.14645 8.14645C3.34171 7.95118 3.65829 7.95118 3.85355 8.14645L7 11.2929L7 2.5C7 2.22386 7.22386 2 7.5 2Z"
                                    fill="currentColor">
                                </path>
                            </svg>
                        },
                        _ => view! {
                            <svg class="ml-2 h-4 w-4 text-grey-500" width="24" height="24" viewBox="0 0 15 15" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                                <path fill-rule="evenodd" clip-rule="evenodd"
                                    d="M4.93179 5.43179C4.75605 5.60753 4.75605 5.89245 4.93179 6.06819C5.10753 6.24392 5.39245 6.24392 5.56819 6.06819L7.49999 4.13638L9.43179 6.06819C9.60753 6.24392 9.89245 6.24392 10.0682 6.06819C10.2439 5.89245 10.2439 5.60753 10.0682 5.43179L7.81819 3.18179C7.73379 3.0974 7.61933 3.04999 7.49999 3.04999C7.38064 3.04999 7.26618 3.0974 7.18179 3.18179L4.93179 5.43179ZM10.0682 9.56819C10.2439 9.39245 10.2439 9.10753 10.0682 8.93179C9.89245 8.75606 9.60753 8.75606 9.43179 8.93179L7.49999 10.8636L5.56819 8.93179C5.39245 8.75606 5.10753 8.75606 4.93179 8.93179C4.75605 9.10753 4.75605 9.39245 4.93179 9.56819L7.18179 11.8182C7.35753 11.9939 7.64245 11.9939 7.81819 11.8182L10.0682 9.56819Z"
                                    fill="currentColor">
                                </path>
                            </svg>
                        },
                    }
                }}
            </button>
        </th>
    }
}
