use crate::{
    auth::{get_user, Login, Logout, Signup},
    database::{Alias, AliasTableDataProvider},
    error_template::ErrorTemplate,
};
use leptos::{
    component, create_node_ref, create_resource, create_server_action, create_server_multi_action, event_target_value,
    html::Div, server, view, Action, CollectView, ErrorBoundary, IntoView, ServerFnError, SignalGet, SignalSet,
    Transition,
};
use leptos_meta::{provide_meta_context, Link, Stylesheet};
use leptos_router::{ActionForm, MultiActionForm, Route, Router, Routes, A};
use leptos_struct_table::*;

#[server]
pub async fn get_todos() -> Result<Vec<Alias>, ServerFnError> {
    use crate::database::ssr::{pool, SqlAlias};
    use futures::future::join_all;

    let pool = pool()?;

    Ok(join_all(
        sqlx::query_as::<_, SqlAlias>("SELECT * FROM todos")
            .fetch_all(&pool)
            .await?
            .iter()
            .map(|todo: &SqlAlias| todo.clone().into_alias(&pool)),
    )
    .await)
}

#[server]
pub async fn add_todo(title: String) -> Result<(), ServerFnError> {
    use crate::database::ssr::pool;
    let user = get_user().await?;
    let pool = pool()?;

    let id = match user {
        Some(user) => user.id,
        None => -1,
    };

    let r = sqlx::query("INSERT INTO todos (title, user_id, completed) VALUES (?, ?, false)")
        .bind(title)
        .bind(id)
        .execute(&pool)
        .await
        .map(|_| ())?;
    Ok(r)
}

// The struct name and path prefix arguments are optional.
#[server]
pub async fn delete_todo(id: u16) -> Result<(), ServerFnError> {
    use crate::database::ssr::pool;
    let pool = pool()?;

    Ok(sqlx::query("DELETE FROM todos WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map(|_| ())?)
}

#[component]
pub fn App() -> impl IntoView {
    let login = create_server_action::<Login>();
    let logout = create_server_action::<Logout>();
    let signup = create_server_action::<Signup>();

    let user = create_resource(
        move || (login.version().get(), signup.version().get(), logout.version().get()),
        move |_| get_user(),
    );
    provide_meta_context();

    view! {
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Stylesheet id="leptos" href="/pkg/idmail.css"/>
        <Router>
            <header class="bg-gray-800 text-white py-4">
                <A href="/" class="text-xl font-bold">
                    "My Tasks"
                </A>
                <Transition fallback=move || {
                    view! { <span class="text-gray-300">"Loading..."</span> }
                }>
                    {move || {
                        user.get()
                            .map(|user| match user {
                                Err(e) => {
                                    view! {
                                        <div>
                                            <A href="/signup" class="text-blue-400">"Signup"</A>
                                            <span class="text-gray-300">", "</span>
                                            <A href="/login" class="text-blue-400">"Login"</A>
                                            <span class="text-gray-300">", "</span>
                                            <span>{format!("Login error: {}", e)}</span>
                                        </div>
                                    }.into_view()
                                }
                                Ok(None) => {
                                    view! {
                                        <div>
                                            <A href="/signup" class="text-blue-400">
                                                "Signup"
                                            </A>
                                            <span class="text-gray-300">", "</span>
                                            <A href="/login" class="text-blue-400">
                                                "Login"
                                            </A>
                                            <span class="text-gray-300">", "</span>
                                            <span>"Logged out."</span>
                                        </div>
                                    }.into_view()
                                }
                                Ok(Some(user)) => {
                                    view! {
                                        <div>
                                            <A href="/settings" class="text-blue-400">
                                                "Settings"
                                            </A>
                                            <span class="text-gray-300">", "</span>
                                            <span>
                                                {format!("Logged in as: {} ({})", user.username, user.id)}
                                            </span>
                                        </div>
                                    }.into_view()
                                }
                            })
                    }}

                </Transition>
            </header>
            <hr class="my-4"/>
            <main>
                <Routes>
                    // Route
                    <Route path="" view=HomePage/>
                    <Route path="signup" view=move || view! { <Signup action=signup/> }/>
                    <Route path="login" view=move || view! { <Login action=login/> }/>
                    <Route
                        path="settings"
                        view=move || {
                            view! {
                                <div>
                                    <h1 class="text-2xl font-bold">"Settings"</h1>
                                    <Logout action=logout/>
                                </div>
                            }
                        }
                    />

                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    let add_todo = create_server_multi_action::<AddTodo>();
    let delete_todo = create_server_action::<DeleteTodo>();
    let submissions = add_todo.submissions();

    // list of todos is loaded from the server in reaction to changes
    let todos = create_resource(
        move || (add_todo.version().get(), delete_todo.version().get()),
        move |_| get_todos(),
    );

    let scroll_container = create_node_ref::<Div>();
    let rows = AliasTableDataProvider::default();
    let name = rows.name;

    view! {
        <MultiActionForm action=add_todo class="mb-4">
            <label class="block mb-2">
                "Add a Todo" <input type="text" name="title" class="form-input"/>
            </label>
            <input type="submit" value="Add" class="button"/>
        </MultiActionForm>
        <div class="overflow-hidden bg-background">
        <div class="hidden h-full flex-1 flex-col space-y-8 p-8 md:flex">
            <div class="flex items-center justify-between space-y-2">
                <div>
                    <h2 class="text-2xl font-bold tracking-tight">Welcome back!</h2>
                    <p class="text-muted-foreground">Heres a list of your tasks for this month!</p>
                </div>
                <div class="flex items-center space-x-2">
                    <button type="button" tabindex="0" aria-controls="Dzj2NiMMX6" aria-expanded="false"
                        class="inline-flex items-center justify-center whitespace-nowrap text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground px-4 py-2 relative h-8 w-8 rounded-full">
                        <div class="relative flex shrink-0 overflow-hidden rounded-full h-9 w-9">
                            <img class="aspect-square h-full w-full" alt="TODO" src="/avatars/01.png" />
                        </div>
                    </button>
                </div>
            </div>
            <div class="space-y-4">
                <div class="flex items-center justify-between">
                    <div class="flex flex-1 items-center space-x-2">
                        <input
                            class="flex rounded-md border border-input bg-transparent px-3 py-1 text-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 h-8 w-[150px] lg:w-[250px]"
                            placeholder="Filter tasks..." type="search" />
                        <button type="button" tabindex="0" role="button" aria-haspopup="dialog" aria-expanded="true"
                            class="inline-flex items-center justify-center whitespace-nowrap font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground rounded-md px-3 text-xs h-8 border-dashed">
                            <svg width="24" height="24" role="img" aria-label="plus circled," viewBox="0 0 15 15"
                                fill="currentColor" xmlns="http://www.w3.org/2000/svg" class="mr-2 h-4 w-4">
                                <path fill-rule="evenodd" clip-rule="evenodd"
                                    d="M7.49991 0.876892C3.84222 0.876892 0.877075 3.84204 0.877075 7.49972C0.877075 11.1574 3.84222 14.1226 7.49991 14.1226C11.1576 14.1226 14.1227 11.1574 14.1227 7.49972C14.1227 3.84204 11.1576 0.876892 7.49991 0.876892ZM1.82707 7.49972C1.82707 4.36671 4.36689 1.82689 7.49991 1.82689C10.6329 1.82689 13.1727 4.36671 13.1727 7.49972C13.1727 10.6327 10.6329 13.1726 7.49991 13.1726C4.36689 13.1726 1.82707 10.6327 1.82707 7.49972ZM7.50003 4C7.77617 4 8.00003 4.22386 8.00003 4.5V7H10.5C10.7762 7 11 7.22386 11 7.5C11 7.77614 10.7762 8 10.5 8H8.00003V10.5C8.00003 10.7761 7.77617 11 7.50003 11C7.22389 11 7.00003 10.7761 7.00003 10.5V8H4.50003C4.22389 8 4.00003 7.77614 4.00003 7.5C4.00003 7.22386 4.22389 7 4.50003 7H7.00003V4.5C7.00003 4.22386 7.22389 4 7.50003 4Z"
                                    fill="currentColor">
                                </path>
                            </svg>
                            Status
                        </button>
                        <button type="button" tabindex="0" role="button" aria-haspopup="dialog" aria-expanded="true"
                            class="inline-flex items-center justify-center whitespace-nowrap font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground rounded-md px-3 text-xs h-8 border-dashed">
                            <svg width="24" height="24" role="img" aria-label="plus circled," viewBox="0 0 15 15"
                                fill="currentColor" xmlns="http://www.w3.org/2000/svg" class="mr-2 h-4 w-4">
                                <path fill-rule="evenodd" clip-rule="evenodd"
                                    d="M7.49991 0.876892C3.84222 0.876892 0.877075 3.84204 0.877075 7.49972C0.877075 11.1574 3.84222 14.1226 7.49991 14.1226C11.1576 14.1226 14.1227 11.1574 14.1227 7.49972C14.1227 3.84204 11.1576 0.876892 7.49991 0.876892ZM1.82707 7.49972C1.82707 4.36671 4.36689 1.82689 7.49991 1.82689C10.6329 1.82689 13.1727 4.36671 13.1727 7.49972C13.1727 10.6327 10.6329 13.1726 7.49991 13.1726C4.36689 13.1726 1.82707 10.6327 1.82707 7.49972ZM7.50003 4C7.77617 4 8.00003 4.22386 8.00003 4.5V7H10.5C10.7762 7 11 7.22386 11 7.5C11 7.77614 10.7762 8 10.5 8H8.00003V10.5C8.00003 10.7761 7.77617 11 7.50003 11C7.22389 11 7.00003 10.7761 7.00003 10.5V8H4.50003C4.22389 8 4.00003 7.77614 4.00003 7.5C4.00003 7.22386 4.22389 7 4.50003 7H7.00003V4.5C7.00003 4.22386 7.22389 4 7.50003 4Z"
                                    fill="currentColor">
                                </path>
                            </svg>
                            Priority
                        </button>
                    </div>
                    <button type="button" tabindex="0" aria-controls="R35lhT0ZGL" aria-expanded="false"
                        class="items-center justify-center whitespace-nowrap font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground rounded-md px-3 text-xs ml-auto hidden h-8 lg:flex">
                        <svg width="24" height="24" role="img" aria-label="mixer horizontal," viewBox="0 0 15 15"
                            fill="currentColor" xmlns="http://www.w3.org/2000/svg" class="mr-2 h-4 w-4">
                            <path fill-rule="evenodd" clip-rule="evenodd"
                                d="M5.5 3C4.67157 3 4 3.67157 4 4.5C4 5.32843 4.67157 6 5.5 6C6.32843 6 7 5.32843 7 4.5C7 3.67157 6.32843 3 5.5 3ZM3 5C3.01671 5 3.03323 4.99918 3.04952 4.99758C3.28022 6.1399 4.28967 7 5.5 7C6.71033 7 7.71978 6.1399 7.95048 4.99758C7.96677 4.99918 7.98329 5 8 5H13.5C13.7761 5 14 4.77614 14 4.5C14 4.22386 13.7761 4 13.5 4H8C7.98329 4 7.96677 4.00082 7.95048 4.00242C7.71978 2.86009 6.71033 2 5.5 2C4.28967 2 3.28022 2.86009 3.04952 4.00242C3.03323 4.00082 3.01671 4 3 4H1.5C1.22386 4 1 4.22386 1 4.5C1 4.77614 1.22386 5 1.5 5H3ZM11.9505 10.9976C11.7198 12.1399 10.7103 13 9.5 13C8.28967 13 7.28022 12.1399 7.04952 10.9976C7.03323 10.9992 7.01671 11 7 11H1.5C1.22386 11 1 10.7761 1 10.5C1 10.2239 1.22386 10 1.5 10H7C7.01671 10 7.03323 10.0008 7.04952 10.0024C7.28022 8.8601 8.28967 8 9.5 8C10.7103 8 11.7198 8.8601 11.9505 10.0024C11.9668 10.0008 11.9833 10 12 10H13.5C13.7761 10 14 10.2239 14 10.5C14 10.7761 13.7761 11 13.5 11H12C11.9833 11 11.9668 10.9992 11.9505 10.9976ZM8 10.5C8 9.67157 8.67157 9 9.5 9C10.3284 9 11 9.67157 11 10.5C11 11.3284 10.3284 12 9.5 12C8.67157 12 8 11.3284 8 10.5Z"
                                fill="currentColor">
                            </path>
                        </svg>
                        View
                    </button>
                </div>

                <div class="flex flex-col h-[100vh] bg-white">
                    <div class="border-b bg-slate-100 px-5 py-2">
                        <label class="relative block">
                            <span class="absolute inset-y-0 left-0 flex items-center pl-3">
                                <svg
                                    class="h-5 w-5 fill-black"
                                    xmlns="http://www.w3.org/2000/svg"
                                    x="0px"
                                    y="0px"
                                    width="30"
                                    height="30"
                                    viewBox="0 0 30 30"
                                >
                                    <path d="M 13 3 C 7.4889971 3 3 7.4889971 3 13 C 3 18.511003 7.4889971 23 13 23 C 15.396508 23 17.597385 22.148986 19.322266 20.736328 L 25.292969 26.707031 A 1.0001 1.0001 0 1 0 26.707031 25.292969 L 20.736328 19.322266 C 22.148986 17.597385 23 15.396508 23 13 C 23 7.4889971 18.511003 3 13 3 z M 13 5 C 17.430123 5 21 8.5698774 21 13 C 21 17.430123 17.430123 21 13 21 C 8.5698774 21 5 17.430123 5 13 C 5 8.5698774 8.5698774 5 13 5 z"></path>
                                </svg>
                            </span>
                            <input
                                class="w-full bg-white placeholder:font-italitc border border-slate-300 rounded-full py-2 pl-10 pr-4 focus:outline-none"
                                placeholder="Search by name or company"
                                type="text"
                                value=name
                                on:change=move |e| name.set(event_target_value(&e))
                            />
                        </label>
                    </div>
                    <div node_ref=scroll_container class="overflow-auto grow min-h-0">
                        <table class="table-fixed text-sm text-left text-gray-500 dark:text-gray-400 w-full">
                            <TableContent
                                rows
                                scroll_container
                            />
                        </table>
                    </div>
                </div>

                <div class="rounded-md border">
                    <div class="relative w-full overflow-auto">
                        <table class="w-full caption-bottom text-sm" role="table">
                            <thead class="[&amp;_tr]:border-b">
                                <tr class="border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted">
                                    <th class="h-10 px-2 text-left align-middle font-medium text-muted-foreground">
                                        <button type="button" data-state="unchecked" role="checkbox"
                                            aria-checked="false" aria-required="false" data-melt-checkbox=""
                                            data-checkbox-root=""
                                            class="peer box-content h-4 w-4 shrink-0 rounded-sm border border-primary focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 data-[disabled=true]:cursor-not-allowed data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground data-[disabled=true]:opacity-50"
                                            aria-label="Select all">
                                            <div class="flex h-4 w-4 items-center justify-center text-current"
                                                data-checkbox-indicator="" data-state="unchecked">
                                                <svg width="24" height="24" role="img" aria-label="check,"
                                                    viewBox="0 0 15 15" fill="currentColor"
                                                    xmlns="http://www.w3.org/2000/svg"
                                                    class="h-3.5 w-3.5 text-transparent">
                                                    <path fill-rule="evenodd" clip-rule="evenodd"
                                                        d="M11.4669 3.72684C11.7558 3.91574 11.8369 4.30308 11.648 4.59198L7.39799 11.092C7.29783 11.2452 7.13556 11.3467 6.95402 11.3699C6.77247 11.3931 6.58989 11.3355 6.45446 11.2124L3.70446 8.71241C3.44905 8.48022 3.43023 8.08494 3.66242 7.82953C3.89461 7.57412 4.28989 7.55529 4.5453 7.78749L6.75292 9.79441L10.6018 3.90792C10.7907 3.61902 11.178 3.53795 11.4669 3.72684Z"
                                                        fill="currentColor">
                                                    </path>
                                                </svg>
                                            </div>
                                        </button>
                                    </th>
                                    <th class="h-10 px-2 text-left align-middle font-medium text-muted-foreground">Alias</th>
                                    <th class="h-10 px-2 text-left align-middle font-medium text-muted-foreground">
                                        <div class="flex items-center">
                                            <button type="button" tabindex="0" aria-controls="fXGZVBRr8S"
                                                aria-expanded="false" data-state="closed" id="WLM4Pkgrjp"
                                                data-melt-dropdown-menu-trigger="" data-menu-trigger=""
                                                class="inline-flex items-center justify-center whitespace-nowrap font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground rounded-md px-3 text-xs -ml-3 h-8 data-[state=open]:bg-accent"
                                                data-button-root="">
                                                Title
                                                <svg width="24" height="24" role="img"
                                                    aria-label="caret sort," viewBox="0 0 15 15" fill="currentColor"
                                                    xmlns="http://www.w3.org/2000/svg" class="ml-2 h-4 w-4">
                                                    <path fill-rule="evenodd" clip-rule="evenodd"
                                                        d="M4.93179 5.43179C4.75605 5.60753 4.75605 5.89245 4.93179 6.06819C5.10753 6.24392 5.39245 6.24392 5.56819 6.06819L7.49999 4.13638L9.43179 6.06819C9.60753 6.24392 9.89245 6.24392 10.0682 6.06819C10.2439 5.89245 10.2439 5.60753 10.0682 5.43179L7.81819 3.18179C7.73379 3.0974 7.61933 3.04999 7.49999 3.04999C7.38064 3.04999 7.26618 3.0974 7.18179 3.18179L4.93179 5.43179ZM10.0682 9.56819C10.2439 9.39245 10.2439 9.10753 10.0682 8.93179C9.89245 8.75606 9.60753 8.75606 9.43179 8.93179L7.49999 10.8636L5.56819 8.93179C5.39245 8.75606 5.10753 8.75606 4.93179 8.93179C4.75605 9.10753 4.75605 9.39245 4.93179 9.56819L7.18179 11.8182C7.35753 11.9939 7.64245 11.9939 7.81819 11.8182L10.0682 9.56819Z"
                                                        fill="currentColor">
                                                    </path>
                                                </svg>
                                            </button>
                                        </div>
                                    </th>
                                    <th class="h-10 px-2 text-left align-middle font-medium text-muted-foreground">
                                        <div class="flex items-center">
                                            <button type="button" tabindex="0" aria-controls="UpcVz4I1iF"
                                                aria-expanded="false" data-state="closed" id="1X5trb05cl"
                                                data-melt-dropdown-menu-trigger="" data-menu-trigger=""
                                                class="inline-flex items-center justify-center whitespace-nowrap font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground rounded-md px-3 text-xs -ml-3 h-8 data-[state=open]:bg-accent"
                                                data-button-root="">Status <svg width="24" height="24" role="img"
                                                    aria-label="caret sort," viewBox="0 0 15 15" fill="currentColor"
                                                    xmlns="http://www.w3.org/2000/svg" class="ml-2 h-4 w-4">
                                                    <path fill-rule="evenodd" clip-rule="evenodd"
                                                        d="M4.93179 5.43179C4.75605 5.60753 4.75605 5.89245 4.93179 6.06819C5.10753 6.24392 5.39245 6.24392 5.56819 6.06819L7.49999 4.13638L9.43179 6.06819C9.60753 6.24392 9.89245 6.24392 10.0682 6.06819C10.2439 5.89245 10.2439 5.60753 10.0682 5.43179L7.81819 3.18179C7.73379 3.0974 7.61933 3.04999 7.49999 3.04999C7.38064 3.04999 7.26618 3.0974 7.18179 3.18179L4.93179 5.43179ZM10.0682 9.56819C10.2439 9.39245 10.2439 9.10753 10.0682 8.93179C9.89245 8.75606 9.60753 8.75606 9.43179 8.93179L7.49999 10.8636L5.56819 8.93179C5.39245 8.75606 5.10753 8.75606 4.93179 8.93179C4.75605 9.10753 4.75605 9.39245 4.93179 9.56819L7.18179 11.8182C7.35753 11.9939 7.64245 11.9939 7.81819 11.8182L10.0682 9.56819Z"
                                                        fill="currentColor">
                                                    </path>
                                                </svg>
                                            </button>
                                        </div>
                                    </th>
                                    <th class="h-10 px-2 text-left align-middle font-medium text-muted-foreground [&amp;:has([role=checkbox])]:pr-0 [&amp;>[role=checkbox]]:translate-y-[2px]"
                                        role="columnheader" colspan="1">
                                        <div class="flex items-center">
                                            <button type="button" tabindex="0" aria-controls="Arxlb_AWo-"
                                                aria-expanded="false" data-state="closed" id="P-yK0isBfB"
                                                data-melt-dropdown-menu-trigger="" data-menu-trigger=""
                                                class="inline-flex items-center justify-center whitespace-nowrap font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground rounded-md px-3 text-xs -ml-3 h-8 data-[state=open]:bg-accent"
                                                data-button-root="">Priority <svg width="24" height="24" role="img"
                                                    aria-label="caret sort," viewBox="0 0 15 15" fill="currentColor"
                                                    xmlns="http://www.w3.org/2000/svg" class="ml-2 h-4 w-4">
                                                    <path fill-rule="evenodd" clip-rule="evenodd"
                                                        d="M4.93179 5.43179C4.75605 5.60753 4.75605 5.89245 4.93179 6.06819C5.10753 6.24392 5.39245 6.24392 5.56819 6.06819L7.49999 4.13638L9.43179 6.06819C9.60753 6.24392 9.89245 6.24392 10.0682 6.06819C10.2439 5.89245 10.2439 5.60753 10.0682 5.43179L7.81819 3.18179C7.73379 3.0974 7.61933 3.04999 7.49999 3.04999C7.38064 3.04999 7.26618 3.0974 7.18179 3.18179L4.93179 5.43179ZM10.0682 9.56819C10.2439 9.39245 10.2439 9.10753 10.0682 8.93179C9.89245 8.75606 9.60753 8.75606 9.43179 8.93179L7.49999 10.8636L5.56819 8.93179C5.39245 8.75606 5.10753 8.75606 4.93179 8.93179C4.75605 9.10753 4.75605 9.39245 4.93179 9.56819L7.18179 11.8182C7.35753 11.9939 7.64245 11.9939 7.81819 11.8182L10.0682 9.56819Z"
                                                        fill="currentColor">
                                                    </path>
                                                </svg>
                                            </button>
                                        </div>
                                    </th>
                                    <th class="h-10 px-2 text-left align-middle font-medium text-muted-foreground [&amp;:has([role=checkbox])]:pr-0 [&amp;>[role=checkbox]]:translate-y-[2px]"
                                        role="columnheader" colspan="1">
                                    </th>
                                </tr>
                            </thead>
                            <tbody role="rowgroup">
            <Transition fallback=move || view! { <p>"Loading..."</p> }>
                <ErrorBoundary fallback=|errors| {
                    view! { <ErrorTemplate errors=errors/> }
                }>
                    {move || {
                        let existing_todos = {
                            move || {
                                todos
                                    .get()
                                    .map(move |todos| match todos {
                                        Err(e) => {
                                            view! {
                                                <pre class="error">"Server Error: " {e.to_string()}</pre>
                                            }
                                                .into_view()
                                        }
                                        Ok(todos) => {
                                            if todos.is_empty() {
                                                view! { <p>"No tasks were found."</p> }.into_view()
                                            } else {
                                                todos
                                                    .into_iter()
                                                    .map(move |todo| {
                                                        view! {
                                                            <tr class="border-b last:border-0 transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted" role="row">
                                                                <td class="p-2">
                                                                    <button type="button"
                                                                        class="h-4 w-4 shrink-0 rounded-sm border border-primary focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring translate-y-[2px]"
                                                                        aria-label="Select row">
                                                                        <div class="flex h-4 w-4 items-center justify-center text-current"
                                                                            data-checkbox-indicator="" data-state="unchecked">
                                                                            <svg width="24" height="24" role="img" aria-label="check,"
                                                                                viewBox="0 0 15 15" fill="currentColor"
                                                                                xmlns="http://www.w3.org/2000/svg"
                                                                                class="h-3.5 w-3.5 text-transparent">
                                                                                <path fill-rule="evenodd" clip-rule="evenodd"
                                                                                    d="M11.4669 3.72684C11.7558 3.91574 11.8369 4.30308 11.648 4.59198L7.39799 11.092C7.29783 11.2452 7.13556 11.3467 6.95402 11.3699C6.77247 11.3931 6.58989 11.3355 6.45446 11.2124L3.70446 8.71241C3.44905 8.48022 3.43023 8.08494 3.66242 7.82953C3.89461 7.57412 4.28989 7.55529 4.5453 7.78749L6.75292 9.79441L10.6018 3.90792C10.7907 3.61902 11.178 3.53795 11.4669 3.72684Z"
                                                                                    fill="currentColor">
                                                                                </path>
                                                                            </svg>
                                                                        </div>
                                                                    </button>
                                                                </td>
                                                                <td class="p-2">
                                                                    <div class="w-[80px]">{todo.title}</div>
                                                                </td>
                                                                <td class="p-2">
                                                                    <div class="flex space-x-2">
                                                                        <span
                                                                            class="inline-flex select-none items-center rounded-md border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 text-foreground">Documentation
                                                                        </span>
                                                                        <span class="max-w-[500px] truncate font-medium">Calculating the bus wont
                                                                            do anything, we need to navigate the back-end JSON protocol!
                                                                        </span>
                                                                    </div>
                                                                </td>
                                                                <td class="p-2">
                                                                    <div class="flex w-[100px] items-center">
                                                                        <svg width="24" height="24" role="img" aria-label="stopwatch,"
                                                                            viewBox="0 0 15 15" fill="currentColor"
                                                                            xmlns="http://www.w3.org/2000/svg"
                                                                            class="mr-2 h-4 w-4 text-muted-foreground">
                                                                            <path fill-rule="evenodd" clip-rule="evenodd"
                                                                                d="M5.49998 0.5C5.49998 0.223858 5.72383 0 5.99998 0H7.49998H8.99998C9.27612 0 9.49998 0.223858 9.49998 0.5C9.49998 0.776142 9.27612 1 8.99998 1H7.99998V2.11922C9.09832 2.20409 10.119 2.56622 10.992 3.13572C11.0116 3.10851 11.0336 3.08252 11.058 3.05806L11.858 2.25806C12.1021 2.01398 12.4978 2.01398 12.7419 2.25806C12.986 2.50214 12.986 2.89786 12.7419 3.14194L11.967 3.91682C13.1595 5.07925 13.9 6.70314 13.9 8.49998C13.9 12.0346 11.0346 14.9 7.49998 14.9C3.96535 14.9 1.09998 12.0346 1.09998 8.49998C1.09998 5.13362 3.69904 2.3743 6.99998 2.11922V1H5.99998C5.72383 1 5.49998 0.776142 5.49998 0.5ZM2.09998 8.49998C2.09998 5.51764 4.51764 3.09998 7.49998 3.09998C10.4823 3.09998 12.9 5.51764 12.9 8.49998C12.9 11.4823 10.4823 13.9 7.49998 13.9C4.51764 13.9 2.09998 11.4823 2.09998 8.49998ZM7.99998 4.5C7.99998 4.22386 7.77612 4 7.49998 4C7.22383 4 6.99998 4.22386 6.99998 4.5V9.5C6.99998 9.77614 7.22383 10 7.49998 10C7.77612 10 7.99998 9.77614 7.99998 9.5V4.5Z"
                                                                                fill="currentColor">
                                                                            </path>
                                                                        </svg>
                                                                        <span>{todo.created_at}</span>
                                                                    </div>
                                                                </td>
                                                                <td class="p-2">
                                                                    <div class="flex items-center">
                                                                        <svg width="24" height="24" role="img" aria-label="arrow up,"
                                                                            viewBox="0 0 15 15" fill="currentColor"
                                                                            xmlns="http://www.w3.org/2000/svg"
                                                                            class="mr-2 h-4 w-4 text-muted-foreground">
                                                                            <path fill-rule="evenodd" clip-rule="evenodd"
                                                                                d="M7.14645 2.14645C7.34171 1.95118 7.65829 1.95118 7.85355 2.14645L11.8536 6.14645C12.0488 6.34171 12.0488 6.65829 11.8536 6.85355C11.6583 7.04882 11.3417 7.04882 11.1464 6.85355L8 3.70711L8 12.5C8 12.7761 7.77614 13 7.5 13C7.22386 13 7 12.7761 7 12.5L7 3.70711L3.85355 6.85355C3.65829 7.04882 3.34171 7.04882 3.14645 6.85355C2.95118 6.65829 2.95118 6.34171 3.14645 6.14645L7.14645 2.14645Z"
                                                                                fill="currentColor">
                                                                            </path>
                                                                        </svg>
                                                                        <span>{todo.user.unwrap_or_default().username}</span>
                                                                    </div>
                                                                </td>
                                                                <td class="p-2">
                                                                    <button type="button" tabindex="0" aria-controls="TETDbpywlU"
                                                                        aria-expanded="false" data-state="closed" id="mnFbnaeVQI"
                                                                        data-melt-dropdown-menu-trigger="" data-menu-trigger=""
                                                                        class="items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground flex h-8 w-8 p-0 data-[state=open]:bg-muted"
                                                                        data-button-root="">
                                                                        <svg width="24" height="24" role="img" aria-label="dots horizontal,"
                                                                            viewBox="0 0 15 15" fill="currentColor"
                                                                            xmlns="http://www.w3.org/2000/svg" class="h-4 w-4">
                                                                            <path fill-rule="evenodd" clip-rule="evenodd"
                                                                                d="M3.625 7.5C3.625 8.12132 3.12132 8.625 2.5 8.625C1.87868 8.625 1.375 8.12132 1.375 7.5C1.375 6.87868 1.87868 6.375 2.5 6.375C3.12132 6.375 3.625 6.87868 3.625 7.5ZM8.625 7.5C8.625 8.12132 8.12132 8.625 7.5 8.625C6.87868 8.625 6.375 8.12132 6.375 7.5C6.375 6.87868 6.87868 6.375 7.5 6.375C8.12132 6.375 8.625 6.87868 8.625 7.5ZM12.5 8.625C13.1213 8.625 13.625 8.12132 13.625 7.5C13.625 6.87868 13.1213 6.375 12.5 6.375C11.8787 6.375 11.375 6.87868 11.375 7.5C11.375 8.12132 11.8787 8.625 12.5 8.625Z"
                                                                                fill="currentColor">
                                                                            </path>
                                                                        </svg>
                                                                        <span class="sr-only">Open
                                                                            Menu
                                                                        </span>
                                                                    </button>
                                                                </td>
                                                            </tr>
                                                        }
                                                    })
                                                    .collect_view()
                                            }
                                        }
                                    })
                                    .unwrap_or_default()
                            }
                        };
                        let pending_todos = move || {
                            submissions
                                .get()
                                .into_iter()
                                .filter(|submission| submission.pending().get())
                                .map(|submission| {
                                    view! {
                                        <li class="mb-2 p-4 bg-yellow-200 rounded-lg shadow-md">
                                            {move || submission.input.get().map(|data| data.title)}
                                        </li>
                                    }
                                })
                                .collect_view()
                        };
                        view! {
                            {existing_todos}
                            {pending_todos}
                        }
                    }}

                </ErrorBoundary>
            </Transition>
                            </tbody>
                        </table>
                    </div>
                </div>
                <div class="flex items-center justify-between px-2">
                    <div class="flex-1 text-sm text-muted-foreground">0 of 100 row(s) selected.
                    </div>
                    <div class="flex items-center space-x-6 lg:space-x-8">
                        <div class="flex items-center space-x-2">
                            <p class="text-sm font-medium">Rows per page
                            </p>
                            <button type="button" aria-autocomplete="list" aria-controls="jNj48H38UZ"
                                aria-expanded="false" aria-labelledby="vzi2ooIj6P" id="_OPxhqxg0i" role="combobox"
                                data-melt-select-trigger="" data-select-trigger=""
                                class="flex items-center justify-between whitespace-nowrap rounded-md border border-input bg-transparent px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50 [&amp;>span]:line-clamp-1 h-8 w-[70px]">
                                <span data-select-value="">10
                                </span>
                                <div>
                                    <svg width="24" height="24" role="img" aria-label="caret sort," viewBox="0 0 15 15"
                                        fill="currentColor" xmlns="http://www.w3.org/2000/svg"
                                        class="h-4 w-4 opacity-50">
                                        <path fill-rule="evenodd" clip-rule="evenodd"
                                            d="M4.93179 5.43179C4.75605 5.60753 4.75605 5.89245 4.93179 6.06819C5.10753 6.24392 5.39245 6.24392 5.56819 6.06819L7.49999 4.13638L9.43179 6.06819C9.60753 6.24392 9.89245 6.24392 10.0682 6.06819C10.2439 5.89245 10.2439 5.60753 10.0682 5.43179L7.81819 3.18179C7.73379 3.0974 7.61933 3.04999 7.49999 3.04999C7.38064 3.04999 7.26618 3.0974 7.18179 3.18179L4.93179 5.43179ZM10.0682 9.56819C10.2439 9.39245 10.2439 9.10753 10.0682 8.93179C9.89245 8.75606 9.60753 8.75606 9.43179 8.93179L7.49999 10.8636L5.56819 8.93179C5.39245 8.75606 5.10753 8.75606 4.93179 8.93179C4.75605 9.10753 4.75605 9.39245 4.93179 9.56819L7.18179 11.8182C7.35753 11.9939 7.64245 11.9939 7.81819 11.8182L10.0682 9.56819Z"
                                            fill="currentColor">
                                        </path>
                                    </svg>
                                </div>
                            </button>
                        </div>
                        <div class="flex w-[100px] items-center justify-center text-sm font-medium">Page 1 of 10
                        </div>
                        <div class="flex items-center space-x-2">
                            <button type="button" tabindex="0"
                                class="items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground hidden h-8 w-8 p-0 lg:flex"
                                disabled="" data-button-root="">
                                <span class="sr-only">Go
                                    to first page
                                </span>
                                <svg width="15" height="15" role="img" aria-label="double arrow left,"
                                    viewBox="0 0 15 15" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                                    <path fill-rule="evenodd" clip-rule="evenodd"
                                        d="M6.85355 3.85355C7.04882 3.65829 7.04882 3.34171 6.85355 3.14645C6.65829 2.95118 6.34171 2.95118 6.14645 3.14645L2.14645 7.14645C1.95118 7.34171 1.95118 7.65829 2.14645 7.85355L6.14645 11.8536C6.34171 12.0488 6.65829 12.0488 6.85355 11.8536C7.04882 11.6583 7.04882 11.3417 6.85355 11.1464L3.20711 7.5L6.85355 3.85355ZM12.8536 3.85355C13.0488 3.65829 13.0488 3.34171 12.8536 3.14645C12.6583 2.95118 12.3417 2.95118 12.1464 3.14645L8.14645 7.14645C7.95118 7.34171 7.95118 7.65829 8.14645 7.85355L12.1464 11.8536C12.3417 12.0488 12.6583 12.0488 12.8536 11.8536C13.0488 11.6583 13.0488 11.3417 12.8536 11.1464L9.20711 7.5L12.8536 3.85355Z"
                                        fill="currentColor">
                                    </path>
                                </svg>
                            </button>
                            <button type="button" tabindex="0"
                                class="inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground h-8 w-8 p-0"
                                disabled="" data-button-root="">
                                <span class="sr-only">Go
                                    to previous page
                                </span>
                                <svg width="15" height="15" role="img" aria-label="chevron left," viewBox="0 0 15 15"
                                    fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                                    <path fill-rule="evenodd" clip-rule="evenodd"
                                        d="M8.84182 3.13514C9.04327 3.32401 9.05348 3.64042 8.86462 3.84188L5.43521 7.49991L8.86462 11.1579C9.05348 11.3594 9.04327 11.6758 8.84182 11.8647C8.64036 12.0535 8.32394 12.0433 8.13508 11.8419L4.38508 7.84188C4.20477 7.64955 4.20477 7.35027 4.38508 7.15794L8.13508 3.15794C8.32394 2.95648 8.64036 2.94628 8.84182 3.13514Z"
                                        fill="currentColor">
                                    </path>
                                </svg>
                            </button>
                            <button type="button" tabindex="0"
                                class="inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground h-8 w-8 p-0"
                                data-button-root="">
                                <span class="sr-only">Go to next
                                    page
                                </span>
                                <svg width="15" height="15" role="img" aria-label="chevron right," viewBox="0 0 15 15"
                                    fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                                    <path fill-rule="evenodd" clip-rule="evenodd"
                                        d="M6.1584 3.13508C6.35985 2.94621 6.67627 2.95642 6.86514 3.15788L10.6151 7.15788C10.7954 7.3502 10.7954 7.64949 10.6151 7.84182L6.86514 11.8418C6.67627 12.0433 6.35985 12.0535 6.1584 11.8646C5.95694 11.6757 5.94673 11.3593 6.1356 11.1579L9.565 7.49985L6.1356 3.84182C5.94673 3.64036 5.95694 3.32394 6.1584 3.13508Z"
                                        fill="currentColor">
                                    </path>
                                </svg>
                            </button>
                            <button type="button" tabindex="0"
                                class="items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground hidden h-8 w-8 p-0 lg:flex"
                                data-button-root="">
                                <span class="sr-only">Go to last
                                    page
                                </span>
                                <svg width="15" height="15" role="img" aria-label="double arrow right,"
                                    viewBox="0 0 15 15" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                                    <path fill-rule="evenodd" clip-rule="evenodd"
                                        d="M2.14645 11.1464C1.95118 11.3417 1.95118 11.6583 2.14645 11.8536C2.34171 12.0488 2.65829 12.0488 2.85355 11.8536L6.85355 7.85355C7.04882 7.65829 7.04882 7.34171 6.85355 7.14645L2.85355 3.14645C2.65829 2.95118 2.34171 2.95118 2.14645 3.14645C1.95118 3.34171 1.95118 3.65829 2.14645 3.85355L5.79289 7.5L2.14645 11.1464ZM8.14645 11.1464C7.95118 11.3417 7.95118 11.6583 8.14645 11.8536C8.34171 12.0488 8.65829 12.0488 8.85355 11.8536L12.8536 7.85355C13.0488 7.65829 13.0488 7.34171 12.8536 7.14645L8.85355 3.14645C8.65829 2.95118 8.34171 2.95118 8.14645 3.14645C7.95118 3.34171 7.95118 3.65829 8.14645 3.85355L11.7929 7.5L8.14645 11.1464Z"
                                        fill="currentColor">
                                    </path>
                                </svg>
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        </div>
    }
}

#[component]
pub fn Login(action: Action<Login, Result<(), ServerFnError>>) -> impl IntoView {
    view! {
        <ActionForm action=action class="w-full max-w-xs">
            <h1 class="text-2xl font-bold mb-4">"Log In"</h1>
            <label class="block mb-4">
                <span class="text-gray-700">"User ID:"</span>
                <input
                    type="text"
                    placeholder="User ID"
                    maxlength="32"
                    name="username"
                    class="auth-input mt-1 block w-full"
                />
            </label>
            <label class="block mb-4">
                <span class="text-gray-700">"Password:"</span>
                <input
                    type="password"
                    placeholder="Password"
                    name="password"
                    class="auth-input mt-1 block w-full"
                />
            </label>
            <label class="inline-flex items-center mb-4">
                <input type="checkbox" name="remember" class="auth-input mr-2"/>
                <span class="text-sm">"Remember me?"</span>
            </label>
            <button type="submit" class="button w-full">
                "Log In"
            </button>
        </ActionForm>
    }
}

#[component]
pub fn Signup(action: Action<Signup, Result<(), ServerFnError>>) -> impl IntoView {
    view! {
        <ActionForm action=action>
            <h1>"Sign Up"</h1>
            <label>
                "User ID:"
                <input
                    type="text"
                    placeholder="User ID"
                    maxlength="32"
                    name="username"
                    class="auth-input"
                />
            </label>
            <br/>
            <label>
                "Password:"
                <input type="password" placeholder="Password" name="password" class="auth-input"/>
            </label>
            <br/>
            <label>
                "Confirm Password:"
                <input
                    type="password"
                    placeholder="Password again"
                    name="password_confirmation"
                    class="auth-input"
                />
            </label>
            <br/>
            <label>
                "Remember me?" <input type="checkbox" name="remember" class="auth-input"/>
            </label>

            <br/>
            <button type="submit" class="button">
                "Sign Up"
            </button>
        </ActionForm>
    }
}

#[component]
pub fn Logout(action: Action<Logout, Result<(), ServerFnError>>) -> impl IntoView {
    view! {
        <div id="loginbox">
            <ActionForm action=action>
                <button type="submit" class="button">
                    "Log Out"
                </button>
            </ActionForm>
        </div>
    }
}
