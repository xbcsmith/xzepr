// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Event publisher port trait for the domain layer.
//!
//! Defines the `EventPublisher` trait that decouples application-layer handlers
//! from any concrete messaging infrastructure.  Concrete implementations live
//! in `src/infrastructure/` and are injected at startup.

use crate::domain::entities::event::Event;
use crate::domain::entities::event_receiver::EventReceiver;
use crate::domain::entities::event_receiver_group::EventReceiverGroup;

/// Port trait for publishing domain events to a messaging backend.
///
/// Implementors must be `Send + Sync` so they can be shared across async tasks
/// behind an `Arc`.  The provided default implementations of
/// `publish_with_receiver` and `publish_with_group` delegate to `publish`, so
/// concrete types only need to implement `publish` unless they want to include
/// receiver- or group-specific metadata in the message envelope.
///
/// # Examples
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use xzepr::domain::repositories::event_publisher::EventPublisher;
/// use xzepr::domain::entities::event::Event;
///
/// struct NoopPublisher;
///
/// #[async_trait::async_trait]
/// impl EventPublisher for NoopPublisher {
///     async fn publish(&self, _event: &Event) -> xzepr::error::Result<()> {
///         Ok(())
///     }
/// }
///
/// let publisher: Arc<dyn EventPublisher> = Arc::new(NoopPublisher);
/// ```
#[async_trait::async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publishes a single event to the messaging backend.
    ///
    /// # Arguments
    ///
    /// * `event` - The domain event to publish.
    ///
    /// # Errors
    ///
    /// Returns an error if the event could not be delivered.
    async fn publish(&self, event: &Event) -> crate::error::Result<()>;

    /// Publishes an event enriched with receiver context.
    ///
    /// The default implementation discards the receiver reference and delegates
    /// to [`Self::publish`].  Override this to embed receiver metadata in the
    /// message envelope.
    ///
    /// # Arguments
    ///
    /// * `event` - The domain event to publish.
    /// * `receiver` - The `EventReceiver` associated with this lifecycle event.
    ///
    /// # Errors
    ///
    /// Returns an error if the event could not be delivered.
    async fn publish_with_receiver(
        &self,
        event: &Event,
        receiver: &EventReceiver,
    ) -> crate::error::Result<()> {
        let _ = receiver;
        self.publish(event).await
    }

    /// Publishes an event enriched with group context.
    ///
    /// The default implementation discards the group reference and delegates
    /// to [`Self::publish`].  Override this to embed group metadata in the
    /// message envelope.
    ///
    /// # Arguments
    ///
    /// * `event` - The domain event to publish.
    /// * `group` - The `EventReceiverGroup` associated with this lifecycle event.
    ///
    /// # Errors
    ///
    /// Returns an error if the event could not be delivered.
    async fn publish_with_group(
        &self,
        event: &Event,
        group: &EventReceiverGroup,
    ) -> crate::error::Result<()> {
        let _ = group;
        self.publish(event).await
    }
}
