use crate::{
    aliases::Aliases,
    auth::{get_user, Login, Logout},
    domains::Domains,
};
use leptos::*;
use leptos_icons::Icon;
use leptos_meta::{provide_meta_context, Link, Stylesheet};
use leptos_router::{ActionForm, Route, Router, Routes};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Stylesheet id="leptos" href="/pkg/idmail.css"/>
        <Router>
            <main>
                <Routes>
                    <Route path="" view=Main/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn Main() -> impl IntoView {
    let login = create_server_action::<Login>();
    let logout = create_server_action::<Logout>();
    let user = create_resource(
        move || (login.version().get(), logout.version().get()),
        move |_| get_user(),
    );

    #[derive(Copy, Clone, PartialEq, Eq)]
    enum Tab {
        Aliases,
        Mailboxes,
        Domains,
    }

    let tab = create_rw_signal(Tab::Aliases);

    view! {
        <Transition fallback=move || {
            view! { <span class="text-gray-300">"Loading..."</span> }
        }>
            {move || {
                user.get()
                    .map(|user| match user {
                        Err(e) => {
                            view! {
                                <div class="absolute">
                                    <span>{format!("Login error: {}", e)}</span>
                                </div>
                                <Login action=login/>
                            }
                                .into_view()
                        }
                        Ok(None) => view! { <Login action=login/> }.into_view(),
                        Ok(Some(user)) => {
                            view! {
                                <div class="flex flex-row items-center py-4 px-4 md:px-12">
                                    <div class="flex-1 flex flex-row">
                                        <h2 class="text-4xl leading-none font-bold bg-gradient-to-br from-purple-600 to-blue-500 inline-block text-transparent bg-clip-text">
                                            idmail
                                        </h2>
                                        <Icon icon=icondata::IoMail class="ml-1 w-6 h-6"/>
                                        <div class="flex flex-row items-center gap-4 ml-12">
                                            <button
                                                type="button"
                                                class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-base hover:text-indigo-700 py-2.5 px-4 transition-all rounded-lg focus-visible:ring-4 hover:bg-indigo-200 focus-visible:ring-blue-300"
                                                class=("bg-indigo-100", move || tab.get() == Tab::Aliases)
                                                class=("text-indigo-700", move || tab.get() == Tab::Aliases)
                                                on:click=move |_| {
                                                    tab.set(Tab::Aliases);
                                                }
                                            >

                                                "Aliases"
                                            </button>
                                            <button
                                                type="button"
                                                class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-base hover:text-indigo-700 py-2.5 px-4 transition-all rounded-lg focus-visible:ring-4 hover:bg-indigo-200 focus-visible:ring-blue-300"
                                                class=("bg-indigo-100", move || tab.get() == Tab::Mailboxes)
                                                class=("text-indigo-700", move || tab.get() == Tab::Mailboxes)
                                                on:click=move |_| {
                                                    tab.set(Tab::Mailboxes);
                                                }
                                            >

                                                "Mailboxes"
                                            </button>
                                            <button
                                                type="button"
                                                class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-base hover:text-indigo-700 py-2.5 px-4 transition-all rounded-lg focus-visible:ring-4 hover:bg-indigo-200 focus-visible:ring-blue-300"
                                                class=("bg-indigo-100", move || tab.get() == Tab::Domains)
                                                class=("text-indigo-700", move || tab.get() == Tab::Domains)
                                                on:click=move |_| {
                                                    tab.set(Tab::Domains);
                                                }
                                            >

                                                "Domains"
                                            </button>
                                        </div>
                                    </div>
                                    <span class="text-base mr-4">{user.username.clone()}</span>
                                    <Logout action=logout/>
                                </div>
                                <div class="overflow-hidden bg-background px-4 md:px-12">
                                    <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4 mt-4">
                                        <div class="rounded-xl border-[1.5px]">
                                            <div class="p-6 flex flex-row items-center justify-between space-y-0 pb-2">
                                                <h3 class="tracking-tight text-sm font-medium">Aliases</h3>
                                                <Icon icon=icondata::FiLogOut class="w-4 h-4"/>
                                            </div>
                                            <div class="p-6 pt-0">
                                                <div class="text-2xl font-bold">1412</div>
                                                <p class="text-xs text-muted-foreground">+20.1% from last month</p>
                                            </div>
                                        </div>
                                        <div class="rounded-xl border-[1.5px]">
                                            <div class="p-6 flex flex-row items-center justify-between space-y-0 pb-2">
                                                <h3 class="tracking-tight text-sm font-medium">Mailboxes</h3>
                                                <Icon icon=icondata::FiLogOut class="w-4 h-4"/>
                                            </div>
                                            <div class="p-6 pt-0">
                                                <div class="text-2xl font-bold">+2350</div>
                                                <p class="text-xs text-muted-foreground">+180.1% from last month</p>
                                            </div>
                                        </div>
                                        <div class="rounded-xl border-[1.5px]">
                                            <div class="p-6 flex flex-row items-center justify-between space-y-0 pb-2">
                                                <h3 class="tracking-tight text-sm font-medium">Total Received</h3>
                                                <Icon icon=icondata::FiLogOut class="w-4 h-4"/>
                                            </div>
                                            <div class="p-6 pt-0">
                                                <div class="text-2xl font-bold">+12,234</div>
                                                <p class="text-xs text-muted-foreground">+19% from last month</p>
                                            </div>
                                        </div>
                                        <div class="rounded-xl border-[1.5px]">
                                            <div class="p-6 flex flex-row items-center justify-between space-y-0 pb-2">
                                                <h3 class="tracking-tight text-sm font-medium">Total Sent</h3>
                                                <Icon icon=icondata::FiLogOut class="w-4 h-4"/>
                                            </div>
                                            <div class="p-6 pt-0">
                                                <div class="text-2xl font-bold">+573</div>
                                                <p class="text-xs text-muted-foreground">+201 since last hour</p>
                                            </div>
                                        </div>
                                    </div>
                                    {move || {
                                        match tab.get() {
                                            Tab::Aliases => view! { <Aliases user=user.clone()/> }.into_view(),
                                            Tab::Mailboxes => view! {}.into_view(),
                                            Tab::Domains => view! { <Domains user=user.clone()/> }.into_view(),
                                        }
                                    }}

                                </div>
                            }
                                .into_view()
                        }
                    })
            }}

        </Transition>
    }
}

#[component]
pub fn Logout(action: Action<Logout, Result<(), ServerFnError>>) -> impl IntoView {
    view! {
        <ActionForm action=action>
            <button
                type="submit"
                class="inline-flex flex-none items-center justify-center whitespace-nowrap font-medium text-base py-2.5 px-4 transition-all rounded-lg focus:ring-4 bg-transparent border-[1.5px] border-gray-200 hover:bg-gray-200 focus:ring-ring"
            >
                <Icon icon=icondata::FiLogOut class="w-6 h-6 me-2"/>
                "Log Out"
            </button>
        </ActionForm>
    }
}
