// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/domain/repositories/event_repo.rs
use async_trait::async_trait;

#[async_trait]
pub trait EventRepository: Send + Sync {
    async fn save(&self, event: &Event) -> Result<(), RepositoryError>;
    
    async fn find_by_id(&self, id: EventId) -> Result<Option<Event>, RepositoryError>;
    
    async fn find_by_receiver(
        &self,
        receiver_id: ReceiverId,
    ) -> Result<Vec<Event>, RepositoryError>;
    
    async fn search(
        &self,
        filters: EventFilters,
    ) -> Result<Vec<Event>, RepositoryError>;
}