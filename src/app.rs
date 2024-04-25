use std::collections::VecDeque;

use crate::{
    aliases::{delete_alias, Alias, AliasTableDataProvider},
    auth::{get_user, Login, Logout, Signup},
    utils::Modal,
};
use leptos::{ev::MouseEvent, html::Dialog, logging::error, *};
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

    sqlx::query("INSERT INTO aliases (address, target, comment) VALUES (?, ?, ?)")
        .bind(address)
        .bind(target)
        .bind(comment)
        .execute(&pool)
        .await
        .map(|_| ())?;

    Ok(())
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
                                            <A href="/signup" class="text-blue-400">
                                                "Signup"
                                            </A>
                                            <span class="text-gray-300">", "</span>
                                            <A href="/login" class="text-blue-400">
                                                "Login"
                                            </A>
                                            <span class="text-gray-300">", "</span>
                                            <span>{format!("Login error: {}", e)}</span>
                                        </div>
                                    }
                                        .into_view()
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
                                    }
                                        .into_view()
                                }
                                Ok(Some(user)) => {
                                    view! {
                                        <div>
                                            <A href="/settings" class="text-blue-400">
                                                "Settings"
                                            </A>
                                            <span class="text-gray-300">", "</span>
                                            <span>{format!("Logged in as: {}", user.username)}</span>
                                        </div>
                                    }
                                        .into_view()
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

    let mut rows = AliasTableDataProvider::default();
    let default_sorting = VecDeque::from([(5, ColumnSort::Descending)]);
    rows.set_sorting(&default_sorting);
    let sorting = create_rw_signal(default_sorting);

    let reload_controller = ReloadController::default();
    let on_input = use_debounce_fn_with_arg(move |value| rows.search.set(value), 300.0);
    let (count, set_count) = create_signal(0);

    let pending_alias_address = create_rw_signal(None);
    let delete_modal_open = create_rw_signal(false);
    let (delete_modal_spinning, set_delete_modal_spinning) = create_signal(false);
    let delete_modal_elem = create_node_ref::<Dialog>();
    let delete_modal_close = Callback::new(move |()| {
        delete_modal_elem
            .get_untracked()
            .expect("dialog to have been created")
            .close();
    });

    #[allow(unused_variables, non_snake_case)]
    let alias_row_renderer = move |class: Signal<String>,
                                   row: Alias,
                                   index: usize,
                                   selected: Signal<bool>,
                                   on_select: EventHandler<MouseEvent>,
                                   on_change: EventHandler<ChangeEvent<Alias>>| {
        let address = row.address.clone();
        view! {
            <tr class=class on:click=move |mouse_event| on_select.run(mouse_event)>
                {row.render_row(index, on_change)}
                <td class="w-1 p-4 whitespace-nowrap text-ellipsis">
                    <div class="inline-flex items-center rounded-md">
                        <button class="text-gray-800 hover:text-blue-600 bg-white hover:bg-gray-100 transition-all border-[1.5px] border-gray-200 rounded-l-lg font-medium px-4 py-2 inline-flex space-x-1 items-center">
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
                                    />
                                </svg>
                            </span>
                        </button>
                        <button
                            class="text-gray-800 hover:text-white bg-white hover:bg-red-600 transition-all border-l-0 border-[1.5px] border-gray-200 rounded-r-lg font-medium px-4 py-2 inline-flex space-x-1 items-center"
                            on:click=move |_| {
                                pending_alias_address.set(Some(address.clone()));
                                set_delete_modal_spinning(false);
                                delete_modal_open.set(true);
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
                                    />
                                </svg>
                            </span>
                        </button>
                    </div>
                </td>
            </tr>
        }
    };

    view! {
        <MultiActionForm action=add_todo class="mb-4">
            <label class="block mb-2">"Add a Todo" <input type="text" name="address" class="form-input"/></label>
            <input type="submit" value="Add" class="button"/>
        </MultiActionForm>
        <div class="overflow-hidden bg-background">
            <div class="h-full flex-1 flex-col space-y-12 p-4 md:p-12">
                <div class="flex items-center justify-between space-y-2">
                    <div>
                        <h2 class="text-4xl font-bold">Aliases</h2>
                        <p class="text-xl text-muted-foreground">coolmailbox@somemail.com</p>
                    </div>
                    <div class="flex items-center space-x-2">
                        <button
                            type="button"
                            class="inline-flex items-center justify-center whitespace-nowrap font-medium transition-all focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground px-4 py-2 relative h-8 w-8 rounded-full"
                        >
                            <div class="relative flex shrink-0 overflow-hidden rounded-full h-9 w-9">
                                <img class="aspect-square h-full w-full" alt="TODO" src="/avatars/01.png"/>
                            </div>
                        </button>
                    </div>
                </div>
                <div class="space-y-4">
                    <div class="flex flex-wrap items-center justify-between">
                        <input
                            class="flex flex-none rounded-lg border-[1.5px] border-input bg-transparent text-xl px-3 py-1 me-2 mb-2 h-12 w-full md:w-[360px] lg:w-[520px] transition-all placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                            type="search"
                            placeholder="Search"
                            value=rows.search
                            on:input=move |e| {
                                on_input(event_target_value(&e));
                            }
                        />

                        <button
                            type="button"
                            class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-lg text-white px-4 h-12 me-2 mb-2 transition-all rounded-lg focus:ring-4 bg-blue-700 hover:bg-blue-800 focus:ring-blue-300"
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
                        <button
                            type="button"
                            class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-lg text-white px-4 h-12 me-2 mb-2 transition-all rounded-lg focus:ring-4 bg-green-700 hover:bg-green-800 focus:ring-green-300"
                        >
                            <svg
                                class="w-6 h-6 me-2"
                                viewBox="-2.5 5 22 22"
                                fill="currentColor"
                                xmlns="http://www.w3.org/2000/svg"
                            >
                                <path d="M14.92 17.56c-0.32-0.32-0.88-0.32-1.2 0s-0.32 0.88 0 1.2l0.76 0.76h-3.76c-0.6 0-1.080-0.32-1.6-0.96-0.28-0.36-0.8-0.44-1.2-0.16-0.36 0.28-0.44 0.8-0.16 1.2 0.84 1.12 1.8 1.64 2.92 1.64h3.76l-0.76 0.76c-0.32 0.32-0.32 0.88 0 1.2 0.16 0.16 0.4 0.24 0.6 0.24s0.44-0.080 0.6-0.24l2.2-2.2c0.32-0.32 0.32-0.88 0-1.2l-2.16-2.24zM10.72 12.48h3.76l-0.76 0.76c-0.32 0.32-0.32 0.88 0 1.2 0.16 0.16 0.4 0.24 0.6 0.24s0.44-0.080 0.6-0.24l2.2-2.2c0.32-0.32 0.32-0.88 0-1.2l-2.2-2.2c-0.32-0.32-0.88-0.32-1.2 0s-0.32 0.88 0 1.2l0.76 0.76h-3.76c-2.48 0-3.64 2.56-4.68 4.84-0.88 2-1.76 3.84-3.12 3.84h-2.080c-0.48 0-0.84 0.36-0.84 0.84s0.36 0.88 0.84 0.88h2.080c2.48 0 3.64-2.56 4.68-4.84 0.88-2 1.72-3.88 3.12-3.88zM0.84 12.48h2.080c0.6 0 1.080 0.28 1.56 0.92 0.16 0.2 0.4 0.32 0.68 0.32 0.2 0 0.36-0.040 0.52-0.16 0.36-0.28 0.44-0.8 0.16-1.2-0.84-1.040-1.8-1.6-2.92-1.6h-2.080c-0.48 0.040-0.84 0.4-0.84 0.88s0.36 0.84 0.84 0.84z"></path>
                            </svg>
                            New Random
                        </button>
                        <div class="flex flex-1"></div>
                        <div class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-lg text-right px-4 h-12">
                            {count} " results"
                        </div>
                    </div>

                    <div class="rounded-lg border-[1.5px] text-lg flex flex-col overflow-hidden">
                        <div class="overflow-auto grow min-h-0">
                            <table class="table-auto text-left w-full">
                                <TableContent
                                    rows
                                    sorting=sorting
                                    row_renderer=alias_row_renderer
                                    reload_controller=reload_controller
                                    loading_row_display_limit=0
                                    on_row_count=set_count
                                />
                            </table>
                        </div>
                    </div>
                </div>
            </div>
            <Modal open=delete_modal_open dialog_el=delete_modal_elem>
                <div class="relative p-4 sm:pr-6 transform overflow-hidden rounded-lg bg-white text-left transition-all sm:w-full sm:max-w-lg">
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
                                <h3 class="text-xl font-semibold leading-6 text-gray-900" id="modal-title">
                                    "Delete " {pending_alias_address}
                                </h3>
                                <div class="mt-2">
                                    <p class="text-lg text-gray-500">
                                        "Are you sure you want to delete this alias? This action cannot be undone."
                                    </p>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div class="flex flex-col-reverse gap-3 sm:flex-row-reverse">
                        <button
                            type="button"
                            class="inline-flex w-full justify-center rounded-lg transition-all bg-white px-3 py-2 font-semibold text-gray-900 focus:ring-4 focus:ring-gray-300 border-[1.5px] border-gray-300 hover:bg-gray-100 sm:w-auto"
                            on:click=move |_ev| { delete_modal_close(()); }
                        >
                            Cancel
                        </button>
                        <button
                            type="button"
                            disabled=delete_modal_spinning
                            class="inline-flex w-full justify-center items-center rounded-lg transition-all px-3 py-2 bg-red-600 hover:bg-red-500 font-semibold text-white focus:ring-4 focus:ring-red-300 sm:w-auto"
                            class=("!bg-red-500", delete_modal_spinning)
                            on:click=move |_ev| {
                                if !delete_modal_spinning() {
                                    let addr = pending_alias_address.get().expect("no pending alias");
                                    let delete_modal_close = delete_modal_close.clone();
                                    set_delete_modal_spinning(true);
                                    spawn_local(async move {
                                        if let Err(e) = delete_alias(addr).await {
                                            // TODO: use toast once ssr available
                                            error!("Failed to delete: {}", e);
                                        } else {
                                            reload_controller.reload();
                                        }
                                        delete_modal_close(());
                                    });
                                    //delete_modal_close(());
                                }
                            }
                        >
                            <Show when=delete_modal_spinning>
                                <svg aria-hidden="true" role="status" class="inline w-4 h-4 me-2 text-red-900 animate-spin" viewBox="0 0 100 101" fill="none" xmlns="http://www.w3.org/2000/svg">
                                    <path d="M100 50.5908C100 78.2051 77.6142 100.591 50 100.591C22.3858 100.591 0 78.2051 0 50.5908C0 22.9766 22.3858 0.59082 50 0.59082C77.6142 0.59082 100 22.9766 100 50.5908ZM9.08144 50.5908C9.08144 73.1895 27.4013 91.5094 50 91.5094C72.5987 91.5094 90.9186 73.1895 90.9186 50.5908C90.9186 27.9921 72.5987 9.67226 50 9.67226C27.4013 9.67226 9.08144 27.9921 9.08144 50.5908Z" fill="currentColor"/>
                                    <path d="M93.9676 39.0409C96.393 38.4038 97.8624 35.9116 97.0079 33.5539C95.2932 28.8227 92.871 24.3692 89.8167 20.348C85.8452 15.1192 80.8826 10.7238 75.2124 7.41289C69.5422 4.10194 63.2754 1.94025 56.7698 1.05124C51.7666 0.367541 46.6976 0.446843 41.7345 1.27873C39.2613 1.69328 37.813 4.19778 38.4501 6.62326C39.0873 9.04874 41.5694 10.4717 44.0505 10.1071C47.8511 9.54855 51.7191 9.52689 55.5402 10.0491C60.8642 10.7766 65.9928 12.5457 70.6331 15.2552C75.2735 17.9648 79.3347 21.5619 82.5849 25.841C84.9175 28.9121 86.7997 32.2913 88.1811 35.8758C89.083 38.2158 91.5421 39.6781 93.9676 39.0409Z" fill="#ffffff"/>
                                </svg>
                            </Show>
                            Delete
                        </button>
                    </div>
                </div>
            </Modal>
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
                <input type="password" placeholder="Password" name="password" class="auth-input mt-1 block w-full"/>
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
                "User ID:" <input type="text" placeholder="User ID" maxlength="32" name="username" class="auth-input"/>
            </label>
            <br/>
            <label>
                "Password:" <input type="password" placeholder="Password" name="password" class="auth-input"/>
            </label>
            <br/>
            <label>
                "Confirm Password:"
                <input type="password" placeholder="Password again" name="password_confirmation" class="auth-input"/>
            </label>
            <br/>
            <label>"Remember me?" <input type="checkbox" name="remember" class="auth-input"/></label>

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
