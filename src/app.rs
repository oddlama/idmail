use crate::{
    aliases::Aliases,
    auth::{get_user, Login, Logout},
    domains::Domains,
};
use leptos::*;
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
                                <div>
                                    <span>{format!("Logged in as: {}", user.username)}</span>
                                    <Logout action=logout/>
                                </div>
                                <Domains user />
                                <Aliases/>
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
        <div id="loginbox">
            <ActionForm action=action>
                <button type="submit" class="button">
                    "Log Out"
                </button>
            </ActionForm>
        </div>
    }
}
