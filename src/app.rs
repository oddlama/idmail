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
                                        <h2 class="text-4xl leading-none font-bold bg-gradient-to-br from-purple-600 to-blue-500 inline-block text-transparent bg-clip-text">idmail</h2>
                                        <Icon icon=icondata::IoMail class="ml-1 w-6 h-6"/>
                                    </div>
                                    <span class="text-base font-semibold mr-4">{user.username.clone()}</span>
                                    <Logout action=logout/>
                                </div>
                                <Domains user=user.clone()/>
                                <Aliases user=user.clone()/>
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
