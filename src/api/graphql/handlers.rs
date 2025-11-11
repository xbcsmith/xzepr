// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/graphql/handlers.rs

use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::graphql::Schema;

/// GraphQL request structure
#[derive(Debug, Deserialize)]
pub struct GraphQLRequest {
    /// The GraphQL query string
    pub query: String,
    /// Optional operation name
    #[serde(rename = "operationName")]
    pub operation_name: Option<String>,
    /// Optional variables
    pub variables: Option<serde_json::Value>,
}

/// GraphQL response structure
#[derive(Debug, Serialize)]
pub struct GraphQLResponse {
    /// Response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Response errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<serde_json::Value>>,
}

/// GraphQL query handler
///
/// Processes GraphQL queries, mutations, and returns responses.
/// This endpoint accepts POST requests with GraphQL queries in the request body.
///
/// # Arguments
///
/// * `schema` - The GraphQL schema containing queries and mutations
/// * `req` - The incoming GraphQL request
///
/// # Returns
///
/// A GraphQL response containing the query results or errors
///
/// # Examples
///
/// ```text
/// POST /graphql
/// Content-Type: application/json
///
/// {
///   "query": "{ eventReceivers(eventReceiver: {}) { id name type } }"
/// }
/// ```
pub async fn graphql_handler(
    State(schema): State<Schema>,
    Json(req): Json<GraphQLRequest>,
) -> Response {
    // Build the GraphQL request
    let mut request = async_graphql::Request::new(req.query);

    // Add operation name if provided
    if let Some(operation_name) = req.operation_name {
        request = request.operation_name(operation_name);
    }

    // Add variables if provided
    if let Some(variables) = req.variables {
        if let Ok(vars) = serde_json::from_value(variables) {
            request = request.variables(vars);
        }
    }

    // Execute the query
    let response = schema.execute(request).await;

    // Convert to JSON response
    let json_response = serde_json::to_value(&response).unwrap_or_else(|_| {
        serde_json::json!({
            "errors": [{
                "message": "Failed to serialize response"
            }]
        })
    });

    Json(json_response).into_response()
}

/// GraphQL Playground IDE handler
///
/// Serves the GraphQL Playground interactive development environment.
/// This provides a browser-based IDE for exploring the GraphQL API,
/// writing queries, and viewing documentation.
///
/// # Arguments
///
/// * `_schema` - The GraphQL schema (not used but kept for consistency)
///
/// # Returns
///
/// An HTML response containing the GraphQL Playground interface
///
/// # Examples
///
/// Access the playground by navigating to `/graphql/playground` in your browser.
/// The playground will be configured to send queries to `/graphql`.
pub async fn graphql_playground(State(_schema): State<Schema>) -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

/// Health check endpoint for GraphQL
///
/// Returns a simple health status for the GraphQL endpoint.
pub async fn graphql_health() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "healthy",
            "service": "graphql"
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::graphql::create_schema;
    use crate::application::handlers::{EventReceiverGroupHandler, EventReceiverHandler};
    use crate::domain::entities::event_receiver::EventReceiver;
    use crate::domain::repositories::{
        event_receiver_group_repo::{EventReceiverGroupRepository, FindEventReceiverGroupCriteria},
        event_receiver_repo::{EventReceiverRepository, FindEventReceiverCriteria},
    };
    use crate::domain::value_objects::{EventReceiverGroupId, EventReceiverId};
    use crate::error::Result;

    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    // Mock repository for testing
    struct MockEventReceiverRepository {
        receivers: Mutex<HashMap<EventReceiverId, EventReceiver>>,
    }

    impl MockEventReceiverRepository {
        fn new() -> Self {
            Self {
                receivers: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl EventReceiverRepository for MockEventReceiverRepository {
        async fn save(&self, receiver: &EventReceiver) -> Result<()> {
            let id = receiver.id();
            self.receivers.lock().unwrap().insert(id, receiver.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: EventReceiverId) -> Result<Option<EventReceiver>> {
            Ok(self.receivers.lock().unwrap().get(&id).cloned())
        }

        async fn find_by_name(&self, _name: &str) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }

        async fn find_by_type(&self, _receiver_type: &str) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }

        async fn find_by_type_and_version(
            &self,
            _receiver_type: &str,
            _version: &str,
        ) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }

        async fn find_by_fingerprint(&self, _fingerprint: &str) -> Result<Option<EventReceiver>> {
            Ok(None)
        }

        async fn list(&self, _limit: usize, _offset: usize) -> Result<Vec<EventReceiver>> {
            Ok(self.receivers.lock().unwrap().values().cloned().collect())
        }

        async fn count(&self) -> Result<usize> {
            Ok(self.receivers.lock().unwrap().len())
        }

        async fn update(&self, _receiver: &EventReceiver) -> Result<()> {
            Ok(())
        }

        async fn delete(&self, _id: EventReceiverId) -> Result<()> {
            Ok(())
        }

        async fn exists_by_name_and_type(&self, _name: &str, _receiver_type: &str) -> Result<bool> {
            Ok(false)
        }

        async fn find_by_criteria(
            &self,
            _criteria: FindEventReceiverCriteria,
        ) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }
    }

    // Mock group repository for testing
    struct MockEventReceiverGroupRepository;

    #[async_trait]
    impl EventReceiverGroupRepository for MockEventReceiverGroupRepository {
        async fn save(
            &self,
            _group: &crate::domain::entities::event_receiver_group::EventReceiverGroup,
        ) -> Result<()> {
            Ok(())
        }

        async fn find_by_id(
            &self,
            _id: EventReceiverGroupId,
        ) -> Result<Option<crate::domain::entities::event_receiver_group::EventReceiverGroup>>
        {
            Ok(None)
        }

        async fn find_by_name(
            &self,
            _name: &str,
        ) -> Result<Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>>
        {
            Ok(vec![])
        }

        async fn find_by_type(
            &self,
            _group_type: &str,
        ) -> Result<Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>>
        {
            Ok(vec![])
        }

        async fn find_by_type_and_version(
            &self,
            _group_type: &str,
            _version: &str,
        ) -> Result<Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>>
        {
            Ok(vec![])
        }

        async fn find_enabled(
            &self,
        ) -> Result<Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>>
        {
            Ok(vec![])
        }

        async fn find_disabled(
            &self,
        ) -> Result<Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>>
        {
            Ok(vec![])
        }

        async fn find_by_event_receiver_id(
            &self,
            _receiver_id: EventReceiverId,
        ) -> Result<Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>>
        {
            Ok(vec![])
        }

        async fn list(
            &self,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>>
        {
            Ok(vec![])
        }

        async fn count(&self) -> Result<usize> {
            Ok(0)
        }

        async fn count_enabled(&self) -> Result<usize> {
            Ok(0)
        }

        async fn count_disabled(&self) -> Result<usize> {
            Ok(0)
        }

        async fn update(
            &self,
            _group: &crate::domain::entities::event_receiver_group::EventReceiverGroup,
        ) -> Result<()> {
            Ok(())
        }

        async fn delete(&self, _id: EventReceiverGroupId) -> Result<()> {
            Ok(())
        }

        async fn enable(&self, _id: EventReceiverGroupId) -> Result<()> {
            Ok(())
        }

        async fn disable(&self, _id: EventReceiverGroupId) -> Result<()> {
            Ok(())
        }

        async fn exists_by_name_and_type(&self, _name: &str, _group_type: &str) -> Result<bool> {
            Ok(false)
        }

        async fn add_event_receiver_to_group(
            &self,
            _group_id: EventReceiverGroupId,
            _receiver_id: EventReceiverId,
        ) -> Result<()> {
            Ok(())
        }

        async fn remove_event_receiver_from_group(
            &self,
            _group_id: EventReceiverGroupId,
            _receiver_id: EventReceiverId,
        ) -> Result<()> {
            Ok(())
        }

        async fn get_group_event_receivers(
            &self,
            _group_id: EventReceiverGroupId,
        ) -> Result<Vec<EventReceiverId>> {
            Ok(vec![])
        }

        async fn find_by_criteria(
            &self,
            _criteria: FindEventReceiverGroupCriteria,
        ) -> Result<Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>>
        {
            Ok(vec![])
        }
    }

    fn create_test_schema() -> Schema {
        let receiver_repo = Arc::new(MockEventReceiverRepository::new());
        let group_repo = Arc::new(MockEventReceiverGroupRepository);

        let receiver_handler = Arc::new(EventReceiverHandler::new(receiver_repo.clone()));
        let group_handler = Arc::new(EventReceiverGroupHandler::new(group_repo, receiver_repo));

        create_schema(receiver_handler, group_handler)
    }

    #[tokio::test]
    async fn test_graphql_handler_executes_query() {
        let schema = create_test_schema();

        let request = GraphQLRequest {
            query: r#"
                {
                    eventReceivers(eventReceiver: {}) {
                        id
                        name
                    }
                }
            "#
            .to_string(),
            operation_name: None,
            variables: None,
        };

        let response = graphql_handler(State(schema), Json(request)).await;

        // Verify we get a response (status check)
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_graphql_handler_with_operation_name() {
        let schema = create_test_schema();

        let request = GraphQLRequest {
            query: r#"
                query GetReceivers {
                    eventReceivers(eventReceiver: {}) {
                        id
                        name
                    }
                }
            "#
            .to_string(),
            operation_name: Some("GetReceivers".to_string()),
            variables: None,
        };

        let response = graphql_handler(State(schema), Json(request)).await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_graphql_playground_returns_html() {
        let schema = create_test_schema();

        let response = graphql_playground(State(schema)).await;
        let html_response = response.into_response();

        // Verify the response has HTML content type
        assert_eq!(
            html_response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("text/html; charset=utf-8")
        );
    }

    #[tokio::test]
    async fn test_graphql_health_endpoint() {
        let (status, json) = graphql_health().await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["service"], "graphql");
    }

    #[tokio::test]
    async fn test_graphql_handler_with_variables() {
        let schema = create_test_schema();

        let variables = serde_json::json!({
            "limit": 10
        });

        let request = GraphQLRequest {
            query: r#"
                query GetReceivers($limit: Int) {
                    eventReceivers(eventReceiver: {}) {
                        id
                        name
                    }
                }
            "#
            .to_string(),
            operation_name: Some("GetReceivers".to_string()),
            variables: Some(variables),
        };

        let response = graphql_handler(State(schema), Json(request)).await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}
