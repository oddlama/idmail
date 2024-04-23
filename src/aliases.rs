use leptos::*;
use leptos_struct_table::*;
use leptos_struct_table::{ColumnSort, TableClassesProvider};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use sqlx::{types::chrono, QueryBuilder, Row};
use std::collections::VecDeque;
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TableRow)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[table(sortable, classes_provider = ClassesPreset, thead_cell_renderer = THeadCellRenderer)]
pub struct Alias {
    #[table(class = "w-40")]
    pub address: String,
    #[table(class = "w-40")]
    pub target: String,
    pub comment: String,
    #[table(class = "w-1")]
    pub n_recv: i64,
    #[table(class = "w-1")]
    pub n_sent: i64,
    #[table(class = "w-1", renderer = "TimediffRenderer")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[table(class = "w-1")]
    pub active: bool,
}

#[component]
fn TimediffRenderer<F>(
    class: String,
    #[prop(into)] value: MaybeSignal<chrono::DateTime<chrono::Utc>>,
    #[allow(dead_code)]
    on_change: F,
    #[allow(dead_code)]
    index: usize,
) -> impl IntoView
where
    F: Fn(chrono::DateTime<chrono::Utc>) + 'static,
{
    let time = create_memo(move |_| {
        let time = value();
        let dt = time - chrono::Utc::now();
        let human_time = chrono_humanize::HumanTime::from(dt);
        human_time.to_string()
    });

    view! {
        <td class=class>
            {time}
        </td>
    }
}

#[derive(Clone, Copy)]
pub struct ClassesPreset;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AliasQuery {
    #[serde(default)]
    sort: VecDeque<(usize, ColumnSort)>,
    range: Range<usize>,
    search: String,
}

#[server]
pub async fn list_aliases(query: AliasQuery) -> Result<Vec<Alias>, ServerFnError> {
    use crate::database::ssr::pool;
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

    let pool = pool()?;
    Ok(query.build_query_as::<Alias>().fetch_all(&pool).await?)
}

#[server]
pub async fn alias_count() -> Result<usize, ServerFnError> {
    use crate::database::ssr::pool;
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
            <button type="button" class="inline-flex items-center justify-center whitespace-nowrap px-2 text-xs -ml-2 h-8 text-gray-900 bg-white focus:outline-none hover:bg-gray-100 focus-visible:ring-2 focus-visible:ring-ring rounded-lg">
                <span class=inner_class>
                    {children()}
                </span>
                <span class="ml-2 w-3">
                    {move || {
                        match (sort_priority(), sort_direction()) {
                            (_, ColumnSort::Ascending) => view! { "↑" },
                            (_, ColumnSort::Descending) => view! { "↓" },
                            _ => view! { "" },
                        }
                    }}
                </span>
            </button>
        </th>
    }
}
