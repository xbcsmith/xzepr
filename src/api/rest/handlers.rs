// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/api/rest/events.rs
pub async fn create_event(
    State(handler): State<Arc<CreateEventHandler>>,
    Extension(user): Extension<AuthenticatedUser>,  // Injected by middleware
    Json(request): Json<CreateEventRequest>,
) -> Result<Json<CreateEventResponse>, ApiError> {
    tracing::info!(
        user_id = %user.user_id,
        username = %user.username,
        "Creating event"
    );
    
    let command = request.into_command()?;
    let event_id = handler.handle(command).await?;
    
    Ok(Json(CreateEventResponse { id: event_id }))
}

// Owner-based authorization
pub async fn delete_event(
    State(repo): State<Arc<dyn EventRepository>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(event_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let event_id = EventId::parse(&event_id)?;
    let event = repo.find_by_id(event_id).await?
        .ok_or(ApiError::NotFound)?;
    
    // Check ownership or admin role
    if event.created_by() != user.user_id && !user.has_role(&Role::Admin) {
        return Err(ApiError::Forbidden(
            "You can only delete your own events".to_string()
        ));
    }
    
    repo.delete(event_id).await?;
    Ok(StatusCode::NO_CONTENT)
}