// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/application/commands/create_event.rs
use crate::domain::{Event, EventRepository};

pub struct CreateEventCommand {
    pub name: String,
    pub version: String,
    pub receiver_id: String,
    pub payload: serde_json::Value,
    pub success: bool,
}

pub struct CreateEventHandler<R: EventRepository> {
    event_repo: R,
    event_publisher: Box<dyn EventPublisher>,
}

impl<R: EventRepository> CreateEventHandler<R> {
    pub async fn handle(
        &self,
        cmd: CreateEventCommand,
    ) -> Result<EventId, ApplicationError> {
        // 1. Validate receiver exists
        // 2. Create domain entity
        let event = Event::new(
            cmd.name,
            Version::parse(&cmd.version)?,
            ReceiverId::parse(&cmd.receiver_id)?,
            cmd.payload,
        )?;
        
        // 3. Save to repository
        self.event_repo.save(&event).await?;
        
        // 4. Publish event
        self.event_publisher.publish(&event).await?;
        
        Ok(event.id())
    }
}