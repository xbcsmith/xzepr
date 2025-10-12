// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/api/rest/events.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

pub struct EventsRouter {
    create_handler: CreateEventHandler,
    query_handler: QueryEventHandler,
}

pub async fn create_event(
    State(handler): State<CreateEventHandler>,
    Json(request): Json<CreateEventRequest>,
) -> Result<Json<CreateEventResponse>, ApiError> {
    let command = request.into_command()?;
    let event_id = handler.handle(command).await?;
    
    Ok(Json(CreateEventResponse { id: event_id }))
}

pub async fn get_event(
    State(handler): State<QueryEventHandler>,
    Path(id): Path<String>,
) -> Result<Json<EventResponse>, ApiError> {
    let event_id = EventId::parse(&id)?;
    let event = handler.get_event(event_id).await?;
    
    Ok(Json(EventResponse::from(event)))
}

// Router setup
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events", post(create_event))
        .route("/events/:id", get(get_event))
        .layer(TraceLayer::new_for_http())
}