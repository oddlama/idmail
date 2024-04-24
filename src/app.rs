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
            <div class="hidden h-full flex-1 flex-col space-y-12 p-12 md:flex">
                <div class="flex items-center justify-between space-y-2">
                    <div>
                        <h2 class="text-4xl font-bold">Aliases</h2>
                        <p class="text-xl text-muted-foreground">coolmailbox@somemail.com</p>
                    </div>
                    <div class="flex items-center space-x-2">
                        <button type="button" class="inline-flex items-center justify-center whitespace-nowrap font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground px-4 py-2 relative h-8 w-8 rounded-full">
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
                                class="flex rounded-lg border-[1.5px] border-input bg-transparent text-xl px-3 py-1 h-12 w-[400px] lg:w-[550px] transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                                type="search"
                                placeholder="Search"
                                value=rows.search
                                on:input=move |e| { on_input(event_target_value(&e)); }
                            />

                            <button type="button" class="inline-flex items-center justify-center whitespace-nowrap font-medium text-lg text-white px-4 h-12 me-3 transition-colors rounded-lg focus:ring-4 bg-blue-700 hover:bg-blue-800 focus:ring-blue-300">
                                <svg class="w-6 h-6 me-2" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <line x1="12" y1="5" x2="12" y2="19" />
                                    <line x1="5" y1="12" x2="19" y2="12" />
                                </svg>
                                New
                            </button>
                            <button type="button" class="inline-flex items-center justify-center whitespace-nowrap font-medium text-lg text-white px-4 h-12 me-3 transition-colors rounded-lg focus:ring-4 bg-green-700 hover:bg-green-800 focus:ring-green-300">
                                <svg class="w-6 h-6 me-2" viewBox="-2.5 5 22 22" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                                    <path d="M14.92 17.56c-0.32-0.32-0.88-0.32-1.2 0s-0.32 0.88 0 1.2l0.76 0.76h-3.76c-0.6 0-1.080-0.32-1.6-0.96-0.28-0.36-0.8-0.44-1.2-0.16-0.36 0.28-0.44 0.8-0.16 1.2 0.84 1.12 1.8 1.64 2.92 1.64h3.76l-0.76 0.76c-0.32 0.32-0.32 0.88 0 1.2 0.16 0.16 0.4 0.24 0.6 0.24s0.44-0.080 0.6-0.24l2.2-2.2c0.32-0.32 0.32-0.88 0-1.2l-2.16-2.24zM10.72 12.48h3.76l-0.76 0.76c-0.32 0.32-0.32 0.88 0 1.2 0.16 0.16 0.4 0.24 0.6 0.24s0.44-0.080 0.6-0.24l2.2-2.2c0.32-0.32 0.32-0.88 0-1.2l-2.2-2.2c-0.32-0.32-0.88-0.32-1.2 0s-0.32 0.88 0 1.2l0.76 0.76h-3.76c-2.48 0-3.64 2.56-4.68 4.84-0.88 2-1.76 3.84-3.12 3.84h-2.080c-0.48 0-0.84 0.36-0.84 0.84s0.36 0.88 0.84 0.88h2.080c2.48 0 3.64-2.56 4.68-4.84 0.88-2 1.72-3.88 3.12-3.88zM0.84 12.48h2.080c0.6 0 1.080 0.28 1.56 0.92 0.16 0.2 0.4 0.32 0.68 0.32 0.2 0 0.36-0.040 0.52-0.16 0.36-0.28 0.44-0.8 0.16-1.2-0.84-1.040-1.8-1.6-2.92-1.6h-2.080c-0.48 0.040-0.84 0.4-0.84 0.88s0.36 0.84 0.84 0.84z" />
                                </svg>
                                New Random
                            </button>
                        </div>
                        <div class="inline-flex items-center justify-center whitespace-nowrap font-medium text-lg border-[1.5px] border-input px-4 h-12 rounded-lg">
                            {count} " results"
                        </div>
                    </div>

                    <div class="rounded-lg border-[1.5px] text-lg flex flex-col overflow-hidden">
                        <div class="overflow-auto grow min-h-0">
                            <table class="table-auto text-left w-full">
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
