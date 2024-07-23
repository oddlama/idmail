use anyhow::{anyhow, Result};
use axum::{
    body::Body as AxumBody,
    extract::{Path, State},
    http::Request,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_session::{SessionConfig, SessionLayer, SessionStore};
use axum_session_auth::{AuthConfig, AuthSessionLayer, SessionSqlitePool};
use idmail::{
    app::App,
    auth::{ssr::AuthSession, User},
    fileserv::file_and_error_handler,
    state::AppState,
};
use leptos::{get_configuration, provide_context};
use leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes};
use log::{info, warn};
use sqlx::{sqlite::SqliteConnectOptions, QueryBuilder, SqlitePool};

async fn server_fn_handler(
    State(app_state): State<AppState>,
    auth_session: AuthSession,
    _path: Path<String>,
    request: Request<AxumBody>,
) -> impl IntoResponse {
    handle_server_fns_with_context(
        move || {
            provide_context(auth_session.clone());
            provide_context(app_state.pool.clone());
        },
        request,
    )
    .await
}

async fn leptos_routes_handler(
    auth_session: AuthSession,
    State(app_state): State<AppState>,
    req: Request<AxumBody>,
) -> Response {
    let handler = leptos_axum::render_route_with_context(
        app_state.leptos_options.clone(),
        app_state.routes.clone(),
        move || {
            provide_context(auth_session.clone());
            provide_context(app_state.pool.clone());
        },
        App,
    );
    handler(req).await.into_response()
}

async fn connect(filename: impl AsRef<std::path::Path>) -> Result<sqlx::Pool<sqlx::Sqlite>> {
    let options = SqliteConnectOptions::new().filename(filename).create_if_missing(true);
    Ok(SqlitePool::connect_with(options).await?)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().without_time().init();

    let pool = connect("aliases.db").await?;

    // Auth section
    let session_config = SessionConfig::default().with_table_name("axum_sessions");
    // Disable user caching
    let auth_config = AuthConfig::<String>::default().set_cache(false);
    let session_store =
        SessionStore::<SessionSqlitePool>::new(Some(SessionSqlitePool::from(pool.clone())), session_config).await?;

    sqlx::migrate!().run(&pool).await?;

    // Create admin user if none exist
    let admin_user_exists = QueryBuilder::new("SELECT COUNT(*) FROM users WHERE username = 'admin'")
        .build_query_scalar::<i64>()
        .fetch_one(&pool)
        .await?
        > 0;
    if !admin_user_exists {
        warn!("admin user doesn't exist in database, recovering...");

        let mut buf = [0u8; 24];
        getrandom::getrandom(&mut buf)?;
        let password = hex::encode(buf);

        let password_hash = idmail::users::mk_password_hash(&password)
            .map_err(|e| anyhow!("failed to hash password for admin user: {e}"))?;
        sqlx::query("INSERT INTO users (username, password_hash, admin) VALUES ('admin', ?, TRUE)")
            .bind(password_hash)
            .execute(&pool)
            .await
            .map(|_| ())?;

        warn!("created admin user with password '{password}'");
    }

    // Setting this to None means we'll be using cargo-leptos and its env vars
    let conf = get_configuration(None).await?;
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app_state = AppState {
        leptos_options,
        pool: pool.clone(),
        routes: routes.clone(),
    };

    // build our application with a route
    let app = Router::new()
        .route("/api/*fn_name", get(server_fn_handler).post(server_fn_handler))
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .fallback(file_and_error_handler)
        .layer(
            AuthSessionLayer::<User, String, SessionSqlitePool, SqlitePool>::new(Some(pool.clone()))
                .with_config(auth_config),
        )
        .layer(SessionLayer::new(session_store))
        .with_state(app_state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    info!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}
