use super::error::{err_to_response, ApiErrorKind};
use super::Controller;
use crate::database::connect::ConnectError;
use crate::types::{ConnectRequest, ConnectResponse};
use axum::{extract::State, response::Response, Json};
use reqwest::StatusCode;

fn connect_error_to_response(connect_error: &ConnectError) -> Response {
    match connect_error {
        ConnectError::FailedToAcquireKey => err_to_response(
            connect_error,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to acquire lock.",
            ApiErrorKind::FailedToAcquireKey,
        ),
        ConnectError::KeyUnheldNoSpawnConfig => err_to_response(
            connect_error,
            StatusCode::CONFLICT,
            "Lock is unheld but no spawn config was provided.",
            ApiErrorKind::KeyUnheldNoSpawnConfig,
        ),
        ConnectError::KeyHeldUnhealthy => err_to_response(
            connect_error,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Lock is held but unhealthy.",
            ApiErrorKind::KeyHeldUnhealthy,
        ),
        ConnectError::KeyHeld { .. } => err_to_response(
            connect_error,
            StatusCode::CONFLICT,
            "Lock is held but tag does not match.",
            ApiErrorKind::KeyHeld,
        ),
        ConnectError::NoDroneAvailable => err_to_response(
            connect_error,
            StatusCode::INTERNAL_SERVER_ERROR,
            "No active drone available.",
            ApiErrorKind::NoDroneAvailable,
        ),
        ConnectError::FailedToRemoveKey => err_to_response(
            connect_error,
            StatusCode::CONFLICT,
            "Failed to remove lock.",
            ApiErrorKind::FailedToRemoveKey,
        ),
        ConnectError::DatabaseError(_) | ConnectError::Serialization(_) => err_to_response(
            connect_error,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal error.",
            ApiErrorKind::Other,
        ),
        ConnectError::NoClusterProvided => err_to_response(
            connect_error,
            StatusCode::BAD_REQUEST,
            "No cluster provided, and no default cluster for this controller.",
            ApiErrorKind::NoClusterProvided,
        ),
        ConnectError::Other(_) => err_to_response(
            connect_error,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal error.",
            ApiErrorKind::Other,
        ),
    }
}

pub async fn handle_connect(
    State(controller): State<Controller>,
    Json(request): Json<ConnectRequest>,
) -> Result<Json<ConnectResponse>, Response> {
    let response = controller
        .connect(&request)
        .await
        .map_err(|e| connect_error_to_response(&e))?;
    Ok(Json(response))
}
