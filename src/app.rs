use crate::{
    aliases::Aliases,
    auth::{get_user, Login, LoginView, Logout},
    domains::Domains,
    mailboxes::Mailboxes,
    users::Users,
};
use leptos::*;
use leptos_icons::Icon;
use leptos_meta::{provide_meta_context, Link, Stylesheet};
use leptos_router::{Redirect, Route, Router, Routes, A};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Tab {
    Aliases,
    Mailboxes,
    Domains,
    Users,
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let login = create_server_action::<Login>();
    let logout = create_server_action::<Logout>();

    view! {
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Stylesheet id="leptos" href="/pkg/idmail.css"/>
        <Router>
            <main>
                <Routes>
                    <Route path="/" view=move || view! { <Redirect path="/login"/> }/>
                    <Route path="/login" view=move || view! { <LoginView login logout/> }/>
                    <Route path="/aliases" view=move || view! { <Tab login logout tab=Tab::Aliases/> }/>
                    <Route path="/mailboxes" view=move || view! { <Tab login logout tab=Tab::Mailboxes/> }/>
                    <Route path="/domains" view=move || view! { <Tab login logout tab=Tab::Domains/> }/>
                    <Route path="/users" view=move || view! { <Tab login logout tab=Tab::Users/> }/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn Tab(
    login: Action<Login, Result<(), ServerFnError>>,
    logout: Action<Logout, Result<(), ServerFnError>>,
    tab: Tab,
) -> impl IntoView {
    let user = create_resource(
        move || (login.version().get(), logout.version().get()),
        move |_| get_user(),
    );

    let class_for = move |t| {
        let a_class_inactive = "inline-flex flex-1 sm:flex-none items-center justify-center whitespace-nowrap font-medium text-base hover:text-indigo-700 py-2.5 px-4 transition-all rounded-lg focus-visible:ring-4 hover:bg-indigo-200 focus-visible:ring-blue-300".to_string();
        let a_class_active = format!("{a_class_inactive} bg-indigo-100 text-indigo-700");
        if t == tab {
            a_class_active
        } else {
            a_class_inactive
        }
    };

    view! {
        <Transition fallback=move || {
            view! { <span class="text-gray-300">"Loading..."</span> }
        }>
            {move || {
                user.get()
                    .map(|user| match user {
                        Ok(Some(user)) => {
                            view! {
                                <div class="flex flex-col sm:flex-row items-center py-6 px-4 md:px-12">
                                    <div class="flex-1 flex flex-col sm:flex-row items-center w-full sm:w-auto">
                                        <div class="flex flex-row items-center mb-4 sm:mb-0">
                                            <h2 class="text-4xl leading-none font-bold bg-gradient-to-br from-purple-600 to-blue-500 inline-block text-transparent bg-clip-text">
                                                idmail
                                            </h2>
                                            <Icon icon=icondata::IoMail class="ml-1 w-6 h-6"/>
                                        </div>
                                        <div class="flex flex-row w-full sm:w-auto items-center gap-4 sm:ml-12 mb-4 sm:mb-0">
                                            <A href="/aliases" class=class_for(Tab::Aliases)>
                                                "Aliases"
                                            </A>
                                            <A href="/mailboxes" class=class_for(Tab::Mailboxes)>
                                                "Mailboxes"
                                            </A>
                                            <A href="/domains" class=class_for(Tab::Domains)>
                                                "Domains"
                                            </A>
                                            <Show when=move || user.admin>
                                                <A href="/users" class=class_for(Tab::Users)>
                                                    "Users"
                                                </A>
                                            </Show>
                                        </div>
                                    </div>
                                    <div class="flex flex-row items-center w-full sm:w-auto">
                                        <div class="flex-1 sm:flex-none"></div>
                                        <span class="text-base mr-4">{user.username.clone()}</span>
                                        <Logout action=logout/>
                                    </div>
                                </div>
                                <div class="overflow-hidden bg-background px-4 md:px-12">
                                    <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
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

                                    {match tab {
                                        Tab::Aliases => view! { <Aliases user=user.clone()/> }.into_view(),
                                        Tab::Mailboxes => view! { <Mailboxes user=user.clone()/> }.into_view(),
                                        Tab::Domains => view! { <Domains user=user.clone()/> }.into_view(),
                                        Tab::Users => view! { <Users/> }.into_view(),
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
