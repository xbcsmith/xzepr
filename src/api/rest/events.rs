// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/rest/events.rs

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use tracing::{error, info, warn};

use crate::api::middleware::jwt::AuthenticatedUser;
use crate::api::rest::dtos::{
    CreateEventReceiverGroupRequest, CreateEventReceiverGroupResponse, CreateEventReceiverRequest,
    CreateEventReceiverResponse, CreateEventRequest, CreateEventResponse, ErrorResponse,
    EventReceiverGroupResponse, EventReceiverQueryParams, EventReceiverResponse, EventResponse,
    PaginatedResponse, PaginationMeta, UpdateEventReceiverGroupRequest, UpdateEventReceiverRequest,
};
use crate::application::handlers::event_receiver_group_handler::UpdateEventReceiverGroupParams;
use crate::application::handlers::{EventHandler, EventReceiverGroupHandler, EventReceiverHandler};
use crate::domain::entities::event::CreateEventParams;
use crate::domain::value_objects::{EventId, EventReceiverGroupId, EventReceiverId, UserId};

/// Application state containing handlers
#[derive(Clone)]
pub struct AppState {
    pub event_handler: EventHandler,
    pub event_receiver_handler: EventReceiverHandler,
    pub event_receiver_group_handler: EventReceiverGroupHandler,
}

/// Creates a new event
pub async fn create_event(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<CreateEventRequest>,
) -> Result<Json<CreateEventResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id_str = user.user_id();
    info!(
        user_id = %user_id_str,
        event_name = %request.name,
        "Creating new event"
    );

    // Parse user ID
    let owner_id = match UserId::parse(user_id_str) {
        Ok(id) => id,
        Err(e) => {
            error!("Invalid user ID in JWT token: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "internal_error".to_string(),
                    "Invalid user ID in authentication token".to_string(),
                )),
            ));
        }
    };

    // Validate request
    if let Err(e) = request.validate() {
        warn!("Event creation validation failed: {}", e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "validation_error".to_string(),
                e.to_string(),
            )),
        ));
    }

    // Parse event receiver ID
    let receiver_id = match request.parse_event_receiver_id() {
        Ok(id) => id,
        Err(e) => {
            warn!("Invalid event receiver ID: {}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::with_field(
                    "validation_error".to_string(),
                    e.to_string(),
                    "event_receiver_id".to_string(),
                )),
            ));
        }
    };

    // Create event
    match state
        .event_handler
        .create_event(CreateEventParams {
            name: request.name,
            version: request.version,
            release: request.release,
            platform_id: request.platform_id,
            package: request.package,
            description: request.description,
            payload: request.payload,
            success: request.success,
            receiver_id,
            owner_id,
        })
        .await
    {
        Ok(event_id) => {
            info!("Event created successfully with ID: {}", event_id);
            Ok(Json(CreateEventResponse {
                data: event_id.to_string(),
            }))
        }
        Err(e) => {
            error!("Failed to create event: {}", e);
            let status = e.status_code();
            Err((
                status,
                Json(ErrorResponse::new(
                    "event_creation_failed".to_string(),
                    e.message(),
                )),
            ))
        }
    }
}

/// Gets an event by ID
pub async fn get_event(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> Result<Json<EventResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting event: {}", id_str);

    // Parse event ID
    let event_id = match EventId::parse(&id_str) {
        Ok(id) => id,
        Err(_) => {
            warn!("Invalid event ID format: {}", id_str);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "invalid_id".to_string(),
                    "Invalid event ID format".to_string(),
                )),
            ));
        }
    };

    // Get event
    match state.event_handler.get_event(event_id).await {
        Ok(Some(event)) => {
            info!("Event found: {}", event_id);
            Ok(Json(EventResponse::from(event)))
        }
        Ok(None) => {
            info!("Event not found: {}", event_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "not_found".to_string(),
                    "Event not found".to_string(),
                )),
            ))
        }
        Err(e) => {
            error!("Failed to get event {}: {}", event_id, e);
            let status = e.status_code();
            Err((
                status,
                Json(ErrorResponse::new(
                    "event_retrieval_failed".to_string(),
                    e.message(),
                )),
            ))
        }
    }
}

/// Creates a new event receiver
pub async fn create_event_receiver(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<CreateEventReceiverRequest>,
) -> Result<Json<CreateEventReceiverResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id_str = user.user_id();
    info!(
        user_id = %user_id_str,
        receiver_name = %request.name,
        "Creating new event receiver"
    );

    // Parse user ID
    let owner_id = match UserId::parse(user_id_str) {
        Ok(id) => id,
        Err(e) => {
            error!("Invalid user ID in JWT token: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "internal_error".to_string(),
                    "Invalid user ID in authentication token".to_string(),
                )),
            ));
        }
    };

    // Validate request
    if let Err(e) = request.validate() {
        warn!("Event receiver creation validation failed: {}", e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "validation_error".to_string(),
                e.to_string(),
            )),
        ));
    }

    // Create event receiver
    match state
        .event_receiver_handler
        .create_event_receiver(
            request.name,
            request.receiver_type,
            request.version,
            request.description,
            request.schema,
            owner_id,
        )
        .await
    {
        Ok(receiver_id) => {
            info!(
                "Event receiver created successfully with ID: {}",
                receiver_id
            );
            Ok(Json(CreateEventReceiverResponse {
                data: receiver_id.to_string(),
            }))
        }
        Err(e) => {
            error!("Failed to create event receiver: {}", e);
            let status = e.status_code();
            Err((
                status,
                Json(ErrorResponse::new(
                    "receiver_creation_failed".to_string(),
                    e.message(),
                )),
            ))
        }
    }
}

/// Gets an event receiver by ID
pub async fn get_event_receiver(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> Result<Json<EventReceiverResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting event receiver: {}", id_str);

    // Parse receiver ID
    let receiver_id = match EventReceiverId::parse(&id_str) {
        Ok(id) => id,
        Err(_) => {
            warn!("Invalid event receiver ID format: {}", id_str);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "invalid_id".to_string(),
                    "Invalid event receiver ID format".to_string(),
                )),
            ));
        }
    };

    // Get event receiver
    match state
        .event_receiver_handler
        .get_event_receiver(receiver_id)
        .await
    {
        Ok(Some(receiver)) => {
            info!("Event receiver found: {}", receiver_id);
            Ok(Json(EventReceiverResponse::from(receiver)))
        }
        Ok(None) => {
            info!("Event receiver not found: {}", receiver_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "not_found".to_string(),
                    "Event receiver not found".to_string(),
                )),
            ))
        }
        Err(e) => {
            error!("Failed to get event receiver {}: {}", receiver_id, e);
            let status = e.status_code();
            Err((
                status,
                Json(ErrorResponse::new(
                    "receiver_retrieval_failed".to_string(),
                    e.message(),
                )),
            ))
        }
    }
}

/// Lists event receivers with optional filtering and pagination
pub async fn list_event_receivers(
    State(state): State<AppState>,
    Query(params): Query<EventReceiverQueryParams>,
) -> Result<Json<PaginatedResponse<EventReceiverResponse>>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Listing event receivers with limit: {}, offset: {}",
        params.limit, params.offset
    );

    // Validate query parameters
    if let Err(e) = params.validate() {
        warn!("Event receiver list validation failed: {}", e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "validation_error".to_string(),
                e.to_string(),
            )),
        ));
    }

    // Get total count for pagination
    let total = match state.event_receiver_handler.count_event_receivers().await {
        Ok(count) => count,
        Err(e) => {
            error!("Failed to count event receivers: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("count_failed".to_string(), e.message())),
            ));
        }
    };

    // List event receivers
    match state
        .event_receiver_handler
        .list_event_receivers(params.limit, params.offset)
        .await
    {
        Ok(receivers) => {
            let responses: Vec<EventReceiverResponse> = receivers
                .into_iter()
                .map(EventReceiverResponse::from)
                .collect();

            let pagination = PaginationMeta::new(params.limit, params.offset, total);

            Ok(Json(PaginatedResponse {
                data: responses,
                pagination,
            }))
        }
        Err(e) => {
            error!("Failed to list event receivers: {}", e);
            let status = e.status_code();
            Err((
                status,
                Json(ErrorResponse::new("list_failed".to_string(), e.message())),
            ))
        }
    }
}

/// Updates an event receiver
pub async fn update_event_receiver(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    Json(request): Json<UpdateEventReceiverRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    info!("Updating event receiver: {}", id_str);

    // Parse receiver ID
    let receiver_id = match EventReceiverId::parse(&id_str) {
        Ok(id) => id,
        Err(_) => {
            warn!("Invalid event receiver ID format: {}", id_str);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "invalid_id".to_string(),
                    "Invalid event receiver ID format".to_string(),
                )),
            ));
        }
    };

    // Validate request
    if let Err(e) = request.validate() {
        warn!("Event receiver update validation failed: {}", e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "validation_error".to_string(),
                e.to_string(),
            )),
        ));
    }

    // Update event receiver
    match state
        .event_receiver_handler
        .update_event_receiver(
            receiver_id,
            request.name,
            request.receiver_type,
            request.version,
            request.description,
            request.schema,
        )
        .await
    {
        Ok(()) => {
            info!("Event receiver updated successfully: {}", receiver_id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!("Failed to update event receiver {}: {}", receiver_id, e);
            let status = e.status_code();
            Err((
                status,
                Json(ErrorResponse::new("update_failed".to_string(), e.message())),
            ))
        }
    }
}

/// Deletes an event receiver
pub async fn delete_event_receiver(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    info!("Deleting event receiver: {}", id_str);

    // Parse receiver ID
    let receiver_id = match EventReceiverId::parse(&id_str) {
        Ok(id) => id,
        Err(_) => {
            warn!("Invalid event receiver ID format: {}", id_str);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "invalid_id".to_string(),
                    "Invalid event receiver ID format".to_string(),
                )),
            ));
        }
    };

    // Delete event receiver
    match state
        .event_receiver_handler
        .delete_event_receiver(receiver_id)
        .await
    {
        Ok(()) => {
            info!("Event receiver deleted successfully: {}", receiver_id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!("Failed to delete event receiver {}: {}", receiver_id, e);
            let status = e.status_code();
            Err((
                status,
                Json(ErrorResponse::new("delete_failed".to_string(), e.message())),
            ))
        }
    }
}

/// Creates a new event receiver group
pub async fn create_event_receiver_group(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<CreateEventReceiverGroupRequest>,
) -> Result<Json<CreateEventReceiverGroupResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id_str = user.user_id();
    info!(
        user_id = %user_id_str,
        group_name = %request.name,
        "Creating new event receiver group"
    );

    // Parse user ID
    let owner_id = match UserId::parse(user_id_str) {
        Ok(id) => id,
        Err(e) => {
            error!("Invalid user ID in JWT token: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "internal_error".to_string(),
                    "Invalid user ID in authentication token".to_string(),
                )),
            ));
        }
    };

    // Validate request
    if let Err(e) = request.validate() {
        warn!("Event receiver group creation validation failed: {}", e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "validation_error".to_string(),
                e.to_string(),
            )),
        ));
    }

    // Parse event receiver IDs
    let receiver_ids = match request.parse_event_receiver_ids() {
        Ok(ids) => ids,
        Err(e) => {
            warn!("Invalid event receiver IDs: {}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::with_field(
                    "validation_error".to_string(),
                    e.to_string(),
                    "event_receiver_ids".to_string(),
                )),
            ));
        }
    };

    // Create event receiver group
    match state
        .event_receiver_group_handler
        .create_event_receiver_group(
            request.name,
            request.group_type,
            request.version,
            request.description,
            request.enabled,
            receiver_ids,
            owner_id,
        )
        .await
    {
        Ok(group_id) => {
            info!(
                "Event receiver group created successfully with ID: {}",
                group_id
            );
            Ok(Json(CreateEventReceiverGroupResponse {
                data: group_id.to_string(),
            }))
        }
        Err(e) => {
            error!("Failed to create event receiver group: {}", e);
            let status = e.status_code();
            Err((
                status,
                Json(ErrorResponse::new(
                    "group_creation_failed".to_string(),
                    e.message(),
                )),
            ))
        }
    }
}

/// Gets an event receiver group by ID
pub async fn get_event_receiver_group(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> Result<Json<EventReceiverGroupResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting event receiver group: {}", id_str);

    // Parse group ID
    let group_id = match EventReceiverGroupId::parse(&id_str) {
        Ok(id) => id,
        Err(_) => {
            warn!("Invalid event receiver group ID format: {}", id_str);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "invalid_id".to_string(),
                    "Invalid event receiver group ID format".to_string(),
                )),
            ));
        }
    };

    // Get event receiver group
    match state
        .event_receiver_group_handler
        .get_event_receiver_group(group_id)
        .await
    {
        Ok(Some(group)) => {
            info!("Event receiver group found: {}", group_id);
            Ok(Json(EventReceiverGroupResponse::from(group)))
        }
        Ok(None) => {
            info!("Event receiver group not found: {}", group_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "not_found".to_string(),
                    "Event receiver group not found".to_string(),
                )),
            ))
        }
        Err(e) => {
            error!("Failed to get event receiver group {}: {}", group_id, e);
            let status = e.status_code();
            Err((
                status,
                Json(ErrorResponse::new(
                    "group_retrieval_failed".to_string(),
                    e.message(),
                )),
            ))
        }
    }
}

/// Updates an event receiver group
pub async fn update_event_receiver_group(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    Json(request): Json<UpdateEventReceiverGroupRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    info!("Updating event receiver group: {}", id_str);

    // Parse group ID
    let group_id = match EventReceiverGroupId::parse(&id_str) {
        Ok(id) => id,
        Err(_) => {
            warn!("Invalid event receiver group ID format: {}", id_str);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "invalid_id".to_string(),
                    "Invalid event receiver group ID format".to_string(),
                )),
            ));
        }
    };

    // Validate request
    if let Err(e) = request.validate() {
        warn!("Event receiver group update validation failed: {}", e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "validation_error".to_string(),
                e.to_string(),
            )),
        ));
    }

    // Parse event receiver IDs if provided
    let receiver_ids = match request.parse_event_receiver_ids() {
        Ok(ids) => ids,
        Err(e) => {
            warn!("Invalid event receiver IDs: {}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::with_field(
                    "validation_error".to_string(),
                    e.to_string(),
                    "event_receiver_ids".to_string(),
                )),
            ));
        }
    };

    // Update event receiver group
    match state
        .event_receiver_group_handler
        .update_event_receiver_group(
            group_id,
            UpdateEventReceiverGroupParams {
                name: request.name,
                group_type: request.group_type,
                version: request.version,
                description: request.description,
                enabled: request.enabled,
                event_receiver_ids: receiver_ids,
            },
        )
        .await
    {
        Ok(()) => {
            info!("Event receiver group updated successfully: {}", group_id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!("Failed to update event receiver group {}: {}", group_id, e);
            let status = e.status_code();
            Err((
                status,
                Json(ErrorResponse::new("update_failed".to_string(), e.message())),
            ))
        }
    }
}

/// Deletes an event receiver group
pub async fn delete_event_receiver_group(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    info!("Deleting event receiver group: {}", id_str);

    // Parse group ID
    let group_id = match EventReceiverGroupId::parse(&id_str) {
        Ok(id) => id,
        Err(_) => {
            warn!("Invalid event receiver group ID format: {}", id_str);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "invalid_id".to_string(),
                    "Invalid event receiver group ID format".to_string(),
                )),
            ));
        }
    };

    // Delete event receiver group
    match state
        .event_receiver_group_handler
        .delete_event_receiver_group(group_id)
        .await
    {
        Ok(()) => {
            info!("Event receiver group deleted successfully: {}", group_id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!("Failed to delete event receiver group {}: {}", group_id, e);
            let status = e.status_code();
            Err((
                status,
                Json(ErrorResponse::new("delete_failed".to_string(), e.message())),
            ))
        }
    }
}

/// Health check endpoint
pub async fn health_check() -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "service": "xzepr",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_invalid_id_parsing() {
        // This would be tested with actual request handling in integration tests
        // Unit tests for ID parsing are covered in the domain value objects
        assert_eq!(StatusCode::BAD_REQUEST.as_u16(), 400);
        assert_eq!(StatusCode::NOT_FOUND.as_u16(), 404);
    }

    #[test]
    fn test_error_response_creation() {
        let error = ErrorResponse::new("test_error".to_string(), "Test error message".to_string());
        assert_eq!(error.error, "test_error");
        assert_eq!(error.message, "Test error message");
        assert!(error.field.is_none());

        let error_with_field = ErrorResponse::with_field(
            "validation_error".to_string(),
            "Field is required".to_string(),
            "name".to_string(),
        );
        assert_eq!(error_with_field.field, Some("name".to_string()));
    }
}
