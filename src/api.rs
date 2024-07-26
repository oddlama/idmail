use crate::{aliases::validate_address, auth::User, state::AppState};
use axum::{
    extract::{self, rejection::JsonRejection, State},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::WithRejection;
use faker_rand::en_us::internet::Username;
use http::{HeaderMap, StatusCode};
use rand::seq::SliceRandom;
use rand::{rngs::OsRng, Rng};
use serde::Deserialize;
use serde_json::json;
use sqlx::QueryBuilder;
use thiserror::Error;

// We derive `thiserror::Error`
#[derive(Debug, Error)]
pub enum ApiError {
    // The `#[from]` attribute generates `From<JsonRejection> for ApiError`
    // implementation. See `thiserror` docs for more information
    #[error(transparent)]
    JsonExtractorRejection(#[from] JsonRejection),
    /// Unauthorized
    #[error("Unauthorized")]
    Unauthorized(String),
    /// Bad Request
    #[error("BadRequest")]
    BadRequest(String),
    /// Internal Server Error
    #[error("ServerError")]
    ServerError(String),
}

// We implement `IntoResponse` so ApiError can be used as a response
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::JsonExtractorRejection(json_rejection) => (json_rejection.status(), json_rejection.body_text()),
            ApiError::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message),
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::ServerError(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };

        let payload = json!({
            "error": message,
            "statusText": message,
        });

        (status, Json(payload)).into_response()
    }
}

async fn login_with_api_token(app_state: &AppState, headers: &HeaderMap) -> Result<User, ApiError> {
    let Some(api_token) = headers.get("Authorization").and_then(|x| x.to_str().ok()) else {
        return Err(ApiError::Unauthorized("Missing API token in request".to_string()));
    };

    let api_token = api_token.strip_prefix("Bearer").unwrap_or(api_token).trim_start();
    let Some(user) = User::get_by_api_token(api_token, &app_state.pool).await else {
        return Err(ApiError::Unauthorized("Invalid API token".to_string()));
    };

    log::info!("api token used successfully for user '{}'", user.username);
    Ok(user)
}

async fn allowed_domains(app_state: &AppState, user: &User) -> Result<Vec<String>, String> {
    let mut query = QueryBuilder::new("SELECT domain FROM domains");
    query.push(" WHERE active = TRUE AND (public = TRUE");
    if let Some(mailbox_owner) = &user.mailbox_owner {
        query.push(" OR owner = ");
        query.push_bind(mailbox_owner.clone());
    }
    query.push(")");

    query
        .build_query_scalar::<String>()
        .fetch_all(&app_state.pool)
        .await
        .map_err(|e| e.to_string())
}

async fn create_random_alias(
    app_state: &AppState,
    user: &User,
    domain: Option<String>,
    comment: &str,
) -> Result<(String, String, String), ApiError> {
    let target = &user.username;
    let owner = &user.username;
    let allowed_domains = allowed_domains(app_state, user).await.map_err(ApiError::BadRequest)?;

    let Some(domain) = domain.or_else(|| allowed_domains.choose(&mut OsRng).cloned()) else {
        return Err(ApiError::BadRequest("no usable domains are configured".to_string()));
    };

    let alias = OsRng.gen::<Username>().to_string();

    // Check if resulting address is valid
    if !allowed_domains.contains(&domain) {
        return Err(ApiError::BadRequest(format!(
            "Chosen domain '{}' does not exist or is not allowed to be used",
            domain
        )));
    };

    let address = validate_address(&alias, &domain, false /* never allow reserved */)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    sqlx::query("INSERT INTO aliases (address, domain, target, comment, active, owner) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&address)
        .bind(&domain)
        .bind(target)
        .bind(comment)
        .bind(true)
        .bind(owner)
        .execute(&app_state.pool)
        .await
        .map_err(|e| {
            log::error!("database error while creating alias via api token: {e}");
            ApiError::ServerError("database error".to_string())
        })?;

    Ok((address, alias, domain))
}

#[derive(Deserialize)]
pub struct SimpleLoginRequest {
    note: String,
}

pub async fn create_simple_login(
    State(app_state): State<AppState>,
    headers: HeaderMap,
    WithRejection(extract::Json(body), _): WithRejection<extract::Json<SimpleLoginRequest>, ApiError>,
) -> Result<impl IntoResponse, ApiError> {
    let user = login_with_api_token(&app_state, &headers).await?;
    let (address, _, _) = create_random_alias(&app_state, &user, None, &body.note).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "alias": address,
        })),
    )
        .into_response())
}

#[derive(Deserialize)]
pub struct AddyIoRequest {
    domain: String,
    description: Option<String>,
}

pub async fn create_addy_io(
    State(app_state): State<AppState>,
    headers: HeaderMap,
    WithRejection(extract::Json(body), _): WithRejection<extract::Json<AddyIoRequest>, ApiError>,
) -> Result<impl IntoResponse, ApiError> {
    let user = login_with_api_token(&app_state, &headers).await?;
    let description = body.description.unwrap_or("".to_string());
    let (address, _, domain) = create_random_alias(
        &app_state,
        &user,
        (!body.domain.is_empty() && body.domain != "random").then_some(body.domain),
        &description,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "data": {
                "id": "00000000-0000-0000-0000-000000000000",
                "user_id": "00000000-0000-0000-0000-000000000000",
                "aliasable_id": null,
                "aliasable_type": null,
                "local_part": "00000000-0000-0000-0000-000000000000",
                "extension": null,
                "domain": domain,
                "email": address,
                "active": true,
                "description": description,
                "from_name": null,
                "emails_forwarded": 0,
                "emails_blocked": 0,
                "emails_replied": 0,
                "emails_sent": 0,
                "recipients": [],
                "last_forwarded": "2000-01-01 00:00:00",
                "last_blocked": null,
                "last_replied": null,
                "last_sent": null,
                "created_at": "2000-01-01 00:00:00",
                "updated_at": "2000-01-01 00:00:00",
                "deleted_at": null
            }
        })),
    )
        .into_response())
}
