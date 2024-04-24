use std::collections::VecDeque;

use crate::{
    aliases::{AliasRowRenderer, AliasTableDataProvider},
    auth::{get_user, Login, Logout, Signup},
};
use leptos::*;
use leptos_meta::{provide_meta_context, Link, Stylesheet};
use leptos_router::{ActionForm, MultiActionForm, Route, Router, Routes, A};
use leptos_struct_table::*;
use leptos_use::use_debounce_fn_with_arg;

#[server]
pub async fn add_todo(address: String) -> Result<(), ServerFnError> {
    use crate::database::ssr::pool;
    let user = get_user().await?;
    let pool = pool()?;
    let target = "target@example.com";
    let comment = "yes very mcuthjgba";
    // TODO: FIXME: AAA

    let r = sqlx::query("INSERT INTO aliases (address, target, comment) VALUES (?, ?, ?)")
        .bind(address)
        .bind(target)
        .bind(comment)
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

    Ok(sqlx::query("DELETE FROM aliases WHERE id = $1")
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
                                                {format!("Logged in as: {}", user.username)}
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

    let rows = AliasTableDataProvider::default();
    let sorting = create_rw_signal(VecDeque::new());
    let reload_controller = ReloadController::default();
    let on_input = use_debounce_fn_with_arg(move |value| rows.search.set(value), 300.0);
    let (count, set_count) = create_signal(0);

    //let reload = move |_| {
    //    reload_controller.reload();
    //};

    view! {
        <MultiActionForm action=add_todo class="mb-4">
            <label class="block mb-2">
                "Add a Todo" <input type="text" name="address" class="form-input"/>
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
                                class="flex rounded-md border border-input bg-transparent px-3 py-1 text-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 h-8 w-[250px] lg:w-[350px]"
                                type="search"
                                placeholder="Search"
                                value=rows.search
                                on:input=move |e| { on_input(event_target_value(&e)); }
                            />
                            <button type="button" class="inline-flex items-center justify-center whitespace-nowrap font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground rounded-md px-3 text-xs h-8 border-dashed">
                                <svg width="24" height="24" role="img" aria-label="plus circled," viewBox="0 0 15 15"
                                    fill="currentColor" xmlns="http://www.w3.org/2000/svg" class="mr-2 h-4 w-4">
                                    <path fill-rule="evenodd" clip-rule="evenodd"
                                        d="M7.49991 0.876892C3.84222 0.876892 0.877075 3.84204 0.877075 7.49972C0.877075 11.1574 3.84222 14.1226 7.49991 14.1226C11.1576 14.1226 14.1227 11.1574 14.1227 7.49972C14.1227 3.84204 11.1576 0.876892 7.49991 0.876892ZM1.82707 7.49972C1.82707 4.36671 4.36689 1.82689 7.49991 1.82689C10.6329 1.82689 13.1727 4.36671 13.1727 7.49972C13.1727 10.6327 10.6329 13.1726 7.49991 13.1726C4.36689 13.1726 1.82707 10.6327 1.82707 7.49972ZM7.50003 4C7.77617 4 8.00003 4.22386 8.00003 4.5V7H10.5C10.7762 7 11 7.22386 11 7.5C11 7.77614 10.7762 8 10.5 8H8.00003V10.5C8.00003 10.7761 7.77617 11 7.50003 11C7.22389 11 7.00003 10.7761 7.00003 10.5V8H4.50003C4.22389 8 4.00003 7.77614 4.00003 7.5C4.00003 7.22386 4.22389 7 4.50003 7H7.00003V4.5C7.00003 4.22386 7.22389 4 7.50003 4Z"
                                        fill="currentColor">
                                    </path>
                                </svg>
                                New
                            </button>
                            <button type="button" class="inline-flex items-center justify-center whitespace-nowrap font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground rounded-md px-3 text-xs h-8 border-dashed">
                                <svg width="24" height="24" role="img" aria-label="plus circled," viewBox="0 0 15 15"
                                    fill="currentColor" xmlns="http://www.w3.org/2000/svg" class="mr-2 h-4 w-4">
                                    <path fill-rule="evenodd" clip-rule="evenodd"
                                        d="M7.49991 0.876892C3.84222 0.876892 0.877075 3.84204 0.877075 7.49972C0.877075 11.1574 3.84222 14.1226 7.49991 14.1226C11.1576 14.1226 14.1227 11.1574 14.1227 7.49972C14.1227 3.84204 11.1576 0.876892 7.49991 0.876892ZM1.82707 7.49972C1.82707 4.36671 4.36689 1.82689 7.49991 1.82689C10.6329 1.82689 13.1727 4.36671 13.1727 7.49972C13.1727 10.6327 10.6329 13.1726 7.49991 13.1726C4.36689 13.1726 1.82707 10.6327 1.82707 7.49972ZM7.50003 4C7.77617 4 8.00003 4.22386 8.00003 4.5V7H10.5C10.7762 7 11 7.22386 11 7.5C11 7.77614 10.7762 8 10.5 8H8.00003V10.5C8.00003 10.7761 7.77617 11 7.50003 11C7.22389 11 7.00003 10.7761 7.00003 10.5V8H4.50003C4.22389 8 4.00003 7.77614 4.00003 7.5C4.00003 7.22386 4.22389 7 4.50003 7H7.00003V4.5C7.00003 4.22386 7.22389 4 7.50003 4Z"
                                        fill="currentColor">
                                    </path>
                                </svg>
                                Random Alias
                            </button>
                        </div>
                        <div class="items-center justify-center whitespace-nowrap font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground rounded-md px-3 text-xs ml-auto hidden h-8 lg:flex">
                            {count} " results"
                        </div>
                    </div>

                    <div class="rounded-md border flex flex-col overflow-hidden">
                        <div class="overflow-auto grow min-h-0">
                            <table class="table-auto text-sm text-left w-full">
                                <TableContent
                                    rows
                                    sorting=sorting
                                    row_renderer=AliasRowRenderer
                                    reload_controller=reload_controller
                                    loading_row_display_limit=0
                                    on_row_count=set_count
                                />
                            </table>
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
