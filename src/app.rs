use crate::{
    aliases::{alias_count, Aliases},
    auth::{get_user, Login, LoginView, Logout},
    domains::Domains,
    mailboxes::Mailboxes,
    users::{AccountSettings, Users},
    utils::ColorModeToggle,
};
use chrono::{Months, Utc};
use leptos::{html::Div, *};
use leptos_icons::Icon;
use leptos_meta::{provide_meta_context, Body, Link, Stylesheet, Title};
use leptos_router::{ActionForm, Redirect, Route, Router, Routes, A};
use leptos_use::{
    on_click_outside_with_options, use_color_mode_with_options, use_preferred_dark, ColorMode, OnClickOutsideOptions,
    UseColorModeOptions,
};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Tab {
    Aliases,
    Mailboxes,
    Domains,
    Users,
    AccountSettings,
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let login = create_server_action::<Login>();
    let logout = create_server_action::<Logout>();
    let color_mode = use_color_mode_with_options(UseColorModeOptions::default().emit_auto(true));
    let prefers_dark = use_preferred_dark();
    let body_class = Signal::derive(move || {
        let dark = match (color_mode.mode)() {
            ColorMode::Light => "",
            ColorMode::Dark => "dark",
            ColorMode::Auto | _ => {
                if prefers_dark() {
                    "dark"
                } else {
                    ""
                }
            }
        };
        format!("bg-white text-black dark:bg-black dark:text-zinc-100 {}", dark)
    });

    view! {
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Stylesheet id="leptos" href="/pkg/idmail.css"/>
        <Title formatter=|text| format!("{text} · idmail")/>
        <Body class=body_class/>
        <Router>
            <main>
                <Routes>
                    <Route
                        path="/"
                        view=move || {
                            view! {
                                <Title text="Login"/>
                                <Redirect path="/login"/>
                            }
                        }
                    />

                    <Route
                        path="/login"
                        view=move || {
                            view! {
                                <Title text="Login"/>
                                <LoginView login logout color_mode=color_mode.mode set_color_mode=color_mode.set_mode/>
                            }
                        }
                    />

                    <Route
                        path="/aliases"
                        view=move || {
                            view! {
                                <Title text="Aliases"/>
                                <Tab
                                    login
                                    logout
                                    color_mode=color_mode.mode
                                    set_color_mode=color_mode.set_mode
                                    tab=Tab::Aliases
                                />
                            }
                        }
                    />

                    <Route
                        path="/mailboxes"
                        view=move || {
                            view! {
                                <Title text="Mailboxes"/>
                                <Tab
                                    login
                                    logout
                                    color_mode=color_mode.mode
                                    set_color_mode=color_mode.set_mode
                                    tab=Tab::Mailboxes
                                />
                            }
                        }
                    />

                    <Route
                        path="/domains"
                        view=move || {
                            view! {
                                <Title text="Domains"/>
                                <Tab
                                    login
                                    logout
                                    color_mode=color_mode.mode
                                    set_color_mode=color_mode.set_mode
                                    tab=Tab::Domains
                                />
                            }
                        }
                    />

                    <Route
                        path="/users"
                        view=move || {
                            view! {
                                <Title text="Users"/>
                                <Tab
                                    login
                                    logout
                                    color_mode=color_mode.mode
                                    set_color_mode=color_mode.set_mode
                                    tab=Tab::Users
                                />
                            }
                        }
                    />

                    <Route
                        path="/account"
                        view=move || {
                            view! {
                                <Title text="Account Settings"/>
                                <Tab
                                    login
                                    logout
                                    color_mode=color_mode.mode
                                    set_color_mode=color_mode.set_mode
                                    tab=Tab::AccountSettings
                                />
                            }
                        }
                    />

                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn Tab(
    login: Action<Login, Result<(), ServerFnError>>,
    logout: Action<Logout, Result<(), ServerFnError>>,
    color_mode: Signal<ColorMode>,
    set_color_mode: WriteSignal<ColorMode>,
    tab: Tab,
) -> impl IntoView {
    let user = create_resource(
        move || (login.version().get(), logout.version().get()),
        move |_| get_user(),
    );

    let class_for = move |t| {
        let a_class_inactive = "inline-flex flex-1 sm:flex-none items-center justify-ceter whitespace-nowrap font-medium text-base hover:text-indigo-700 dark:hover:text-indigo-300 py-2.5 px-4 transition-all rounded-lg focus-visible:ring-4 hover:bg-indigo-200 dark:hover:bg-indigo-900 focus-visible:ring-blue-300 dark:focus-visible:ring-blue-900".to_string();
        let a_class_active =
            format!("{a_class_inactive} bg-indigo-100 dark:bg-indigo-950 text-indigo-700 dark:text-indigo-100");
        if t == tab {
            a_class_active
        } else {
            a_class_inactive
        }
    };

    let account_dropdown = create_node_ref::<Div>();
    let (show_account_dropdown, set_show_account_dropdown) = create_signal(false);
    let toggle_show_account_dropdown = move || set_show_account_dropdown.update(|val| *val = !*val);
    let _ = on_click_outside_with_options(
        account_dropdown,
        move |_event| {
            set_show_account_dropdown(false);
        },
        OnClickOutsideOptions::default().ignore(["#account-button"]),
    );

    let active_alias_count = create_resource(|| (), |_| async move { alias_count(Some(true), None).await });
    let inactive_alias_count = create_resource(|| (), |_| async move { alias_count(Some(false), None).await });
    let new_since_last_month = create_resource(
        || (),
        |_| async move { alias_count(None, Some(Utc::now() - Months::new(1))).await },
    );
    let reload_stats = Callback::new(move |_: ()| {
        active_alias_count.refetch();
        inactive_alias_count.refetch();
    });

    view! {
        <Transition fallback=move || {
            view! { <span class="text-gray-300 dark:text-gray-600">"Loading..."</span> }
        }>
            {move || {
                user.get()
                    .map(|user| match user {
                        Ok(Some(user)) => {
                            let is_mailbox = user.mailbox_owner.is_some();
                            view! {
                                <div class="flex flex-col sm:flex-row items-center py-6 px-4 md:px-12">
                                    <div class="flex-1 flex flex-col sm:flex-row items-center w-full sm:w-auto">
                                        <A href="/aliases" class="flex flex-row items-center mb-4 sm:mb-0 items-center">
                                            <img class="w-16 h-16 me-2" src="/logo.svg"/>
                                            <h2 class="text-4xl leading-none font-bold inline-block">idmail</h2>
                                        </A>
                                        <div class="flex flex-row w-full sm:w-auto items-center gap-4 sm:ml-12 mb-4 sm:mb-0">
                                            <A href="/aliases" class=class_for(Tab::Aliases)>
                                                "Aliases"
                                            </A>
                                            <Show when=move || !is_mailbox>
                                                <A href="/mailboxes" class=class_for(Tab::Mailboxes)>
                                                    "Mailboxes"
                                                </A>
                                                <A href="/domains" class=class_for(Tab::Domains)>
                                                    "Domains"
                                                </A>
                                            </Show>
                                            <Show when=move || user.admin>
                                                <A href="/users" class=class_for(Tab::Users)>
                                                    "Users"
                                                </A>
                                            </Show>
                                        </div>
                                    </div>
                                    <div class="flex flex-row items-center w-full sm:w-auto relative">
                                        <div class="flex-1 sm:flex-none"></div>

                                        <ColorModeToggle color_mode set_color_mode/>
                                        <button
                                            id="account-button"
                                            type="button"
                                            class="flex items-center text-sm pe-1 font-medium text-gray-900 dark:text-gray-200 rounded-lg hover:text-indigo-600 dark:hover:text-indigo-400 md:me-0"
                                            on:click=move |_ev| {
                                                toggle_show_account_dropdown();
                                            }
                                        >

                                            {user.username.clone()}
                                            <svg
                                                class="w-2.5 h-2.5 ms-3"
                                                xmlns="http://www.w3.org/2000/svg"
                                                fill="none"
                                                viewBox="0 0 10 6"
                                            >
                                                <path
                                                    stroke="currentColor"
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                    stroke-width="2"
                                                    d="m1 1 4 4 4-4"
                                                ></path>
                                            </svg>
                                        </button>

                                        <div
                                            node_ref=account_dropdown
                                            class="z-10 bg-white dark:bg-black divide-y-[1.5px] divide-gray-200 dark:divide-zinc-800 rounded-lg border-[1.5px] border-gray-200 dark:border-zinc-800 min-w-44 max-w-80 hidden absolute top-6 right-0"
                                            class=("!block", show_account_dropdown)
                                        >
                                            <div class="px-4 py-3 text-sm text-gray-900 dark:text-gray-200">
                                                <div class="font-medium">
                                                    {if user.admin {
                                                        "Admin"
                                                    } else if is_mailbox {
                                                        "Mailbox"
                                                    } else {
                                                        "User"
                                                    }}

                                                </div>
                                                <div class="truncate">{user.username.clone()}</div>
                                            </div>
                                            <ul class="py-2 text-sm">
                                                <li>
                                                    <A
                                                        href="/account"
                                                        class="block px-4 py-2 w-full text-sm text-left hover:bg-gray-100 dark:hover:bg-gray-700"
                                                    >
                                                        "Settings"
                                                    </A>
                                                </li>
                                            </ul>
                                            <div class="py-2">
                                                <ActionForm action=logout>
                                                    <button
                                                        type="submit"
                                                        class="block px-4 py-2 w-full text-sm text-left hover:bg-gray-100 dark:hover:bg-gray-700"
                                                    >
                                                        "Sign Out"
                                                    </button>
                                                </ActionForm>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                                <div class="overflow-hidden px-4 md:px-12">
                                    <Show when=move || tab != Tab::AccountSettings>
                                        <div class="grid gap-4 lg:grid-cols-3">
                                            <div class="rounded-xl border-[1.5px] border-gray-200 dark:border-zinc-800">
                                                <div class="p-4 flex flex-row items-center justify-between space-y-0 pb-2">
                                                    <h3 class="tracking-tight text-sm font-medium">Aliases</h3>
                                                    <Icon icon=icondata::TbMailForward class="w-5 h-5"/>
                                                </div>
                                                <div class="p-4 pt-0">
                                                    <div class="text-2xl font-bold">
                                                        <Transition fallback=move || {
                                                            view! { <p class="animate-pulse">"..."</p> }
                                                        }>
                                                            {move || match active_alias_count.get() {
                                                                Some(Ok(count)) => view! { {count} }.into_view(),
                                                                _ => view! {}.into_view(),
                                                            }}
                                                            " active"

                                                        </Transition>
                                                    </div>
                                                    <p class="text-xs text-gray-500 dark:text-gray-400">
                                                        <Transition fallback=move || {
                                                            view! { <span class="animate-pulse">"..."</span> }
                                                        }>
                                                            {move || match inactive_alias_count.get() {
                                                                Some(Ok(count)) => view! { {count} }.into_view(),
                                                                _ => view! {}.into_view(),
                                                            }}
                                                            " inactive, "
                                                        </Transition>
                                                        <Transition fallback=move || {
                                                            view! { <span class="animate-pulse">"..."</span> }
                                                        }>
                                                            "+"
                                                            {move || match new_since_last_month.get() {
                                                                Some(Ok(count)) => view! { {count} }.into_view(),
                                                                _ => view! {}.into_view(),
                                                            }}
                                                            " new last month"
                                                        </Transition>
                                                    </p>
                                                </div>
                                            </div>
                                            <div class="rounded-xl border-[1.5px] border-gray-200 dark:border-zinc-800">
                                                <div class="p-4 flex flex-row items-center justify-between space-y-0 pb-2">
                                                    <h3 class="tracking-tight text-sm font-medium">Total Received</h3>
                                                    <Icon icon=icondata::BsArrowDown class="w-5 h-5"/>
                                                </div>
                                                <div class="p-4 pt-0">
                                                    <div class="text-2xl font-bold">+12,234</div>
                                                    <p class="text-xs text-gray-500 dark:text-gray-400">
                                                        +19% from last month
                                                    </p>
                                                </div>
                                            </div>
                                            <div class="rounded-xl border-[1.5px] border-gray-200 dark:border-zinc-800">
                                                <div class="p-4 flex flex-row items-center justify-between space-y-0 pb-2">
                                                    <h3 class="tracking-tight text-sm font-medium">Total Sent</h3>
                                                    <Icon icon=icondata::BsArrowUp class="w-5 h-5"/>
                                                </div>
                                                <div class="p-4 pt-0">
                                                    <div class="text-2xl font-bold">+573</div>
                                                    <p class="text-xs text-gray-500 dark:text-gray-400">
                                                        +201 since last hour
                                                    </p>
                                                </div>
                                            </div>
                                        </div>
                                    </Show>

                                    {match tab {
                                        Tab::Aliases => view! { <Aliases user=user.clone() reload_stats/> }.into_view(),
                                        Tab::Mailboxes => {
                                            view! { <Mailboxes user=user.clone() reload_stats/> }.into_view()
                                        }
                                        Tab::Domains => view! { <Domains user=user.clone()/> }.into_view(),
                                        Tab::Users => view! { <Users/> }.into_view(),
                                        Tab::AccountSettings => {
                                            view! { <AccountSettings user=user.clone()/> }.into_view()
                                        }
                                    }}

                                </div>
                            }
                                .into_view()
                        }
                        _ => view! { <Redirect path="/login"/> }.into_view(),
                    })
            }}

        </Transition>
    }
}
