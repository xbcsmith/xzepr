// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/infrastructure/messaging/producer.rs
use rdkafka::producer::{FutureProducer, FutureRecord};

pub struct KafkaEventPublisher {
    producer: FutureProducer,
    topic: String,
}

#[async_trait]
impl EventPublisher for KafkaEventPublisher {
    async fn publish(&self, event: &Event) -> Result<(), MessagingError> {
        let payload = serde_json::to_string(event)?;
        let key = event.id().to_string();
        
        let record = FutureRecord::to(&self.topic)
            .key(&key)
            .payload(&payload);
        
        self.producer
            .send(record, Duration::from_secs(5))
            .await
            .map_err(|(err, _)| MessagingError::from(err))?;
        
        Ok(())
    }
}