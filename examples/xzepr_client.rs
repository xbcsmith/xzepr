// examples/xzepr_client.rs
//! XZepr API Client Example
//!
//! This example demonstrates how to interact with the XZepr REST API
//! to create event receivers, events, and event receiver groups.
//!
//! Usage:
//!   cargo run --example xzepr_client -- --help

use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::error::Error;


/// XZepr API Client
#[derive(Parser)]
#[command(name = "xzepr-client")]
#[command(about = "A client for interacting with the XZepr event tracking API")]
struct Cli {
    /// Base URL of the XZepr API
    #[arg(long, default_value = "http://localhost:8042")]
    base_url: String,

    /// API key for authentication (optional)
    #[arg(long)]
    api_key: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Event receiver operations
    Receiver {
        #[command(subcommand)]
        action: ReceiverCommands,
    },
    /// Event operations
    Event {
        #[command(subcommand)]
        action: EventCommands,
    },
    /// Event receiver group operations
    Group {
        #[command(subcommand)]
        action: GroupCommands,
    },
    /// Generate sample data
    Generate {
        /// Number of event receivers to create
        #[arg(long, default_value = "5")]
        receivers: usize,
        /// Number of events per receiver
        #[arg(long, default_value = "3")]
        events_per_receiver: usize,
    },
    /// Health check
    Health,
}

#[derive(Subcommand)]
enum ReceiverCommands {
    /// Create a new event receiver
    Create {
        /// Name of the event receiver
        name: String,
        /// Type of the event receiver
        #[arg(short = 't', long)]
        receiver_type: String,
        /// Version of the event receiver
        #[arg(short = 'v', long)]
        version: String,
        /// Description of the event receiver
        #[arg(short = 'd', long)]
        description: String,
    },
    /// Get an event receiver by ID
    Get {
        /// Event receiver ID (ULID)
        id: String,
    },
    /// List all event receivers
    List {
        /// Maximum number of results
        #[arg(long, default_value = "50")]
        limit: usize,
        /// Offset for pagination
        #[arg(long, default_value = "0")]
        offset: usize,
    },
}

#[derive(Subcommand)]
enum EventCommands {
    /// Create a new event
    Create {
        /// Name of the event
        name: String,
        /// Version of the event
        #[arg(short = 'v', long)]
        version: String,
        /// Release identifier
        #[arg(short = 'r', long)]
        release: String,
        /// Platform ID
        #[arg(short = 'p', long)]
        platform_id: String,
        /// Package name
        #[arg(long)]
        package: String,
        /// Event description
        #[arg(short = 'd', long)]
        description: String,
        /// Success status
        #[arg(long)]
        success: bool,
        /// Event receiver ID
        #[arg(long)]
        event_receiver_id: String,
        /// JSON payload (optional)
        #[arg(long)]
        payload: Option<String>,
    },
    /// Get an event by ID
    Get {
        /// Event ID (ULID)
        id: String,
    },
}

#[derive(Subcommand)]
enum GroupCommands {
    /// Create a new event receiver group
    Create {
        /// Name of the group
        name: String,
        /// Type of the group
        #[arg(short = 't', long)]
        group_type: String,
        /// Version of the group
        #[arg(short = 'v', long)]
        version: String,
        /// Description of the group
        #[arg(short = 'd', long)]
        description: String,
        /// Whether the group is enabled
        #[arg(long)]
        enabled: bool,
        /// Event receiver IDs (comma-separated)
        #[arg(long)]
        receiver_ids: String,
    },
    /// Get an event receiver group by ID
    Get {
        /// Group ID (ULID)
        id: String,
    },
}

// API DTOs
#[derive(Debug, Serialize)]
struct CreateEventReceiverRequest {
    name: String,
    #[serde(rename = "type")]
    receiver_type: String,
    version: String,
    description: String,
    schema: Value,
}

#[derive(Debug, Deserialize)]
struct CreateEventReceiverResponse {
    data: String,
}

#[derive(Debug, Serialize)]
struct CreateEventRequest {
    name: String,
    version: String,
    release: String,
    platform_id: String,
    package: String,
    description: String,
    payload: Value,
    success: bool,
    event_receiver_id: String,
}

#[derive(Debug, Deserialize)]
struct CreateEventResponse {
    data: String,
}

#[derive(Debug, Serialize)]
struct CreateEventReceiverGroupRequest {
    name: String,
    #[serde(rename = "type")]
    group_type: String,
    version: String,
    description: String,
    enabled: bool,
    event_receiver_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CreateEventReceiverGroupResponse {
    data: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EventReceiverResponse {
    id: String,
    name: String,
    #[serde(rename = "type")]
    receiver_type: String,
    version: String,
    description: String,
    schema: Value,
    fingerprint: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EventResponse {
    id: String,
    name: String,
    version: String,
    release: String,
    platform_id: String,
    package: String,
    description: String,
    payload: Value,
    success: bool,
    event_receiver_id: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EventReceiverGroupResponse {
    id: String,
    name: String,
    #[serde(rename = "type")]
    group_type: String,
    version: String,
    description: String,
    enabled: bool,
    event_receiver_ids: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PaginatedResponse<T> {
    data: Vec<T>,
    pagination: PaginationMeta,
}

#[derive(Debug, Serialize, Deserialize)]
struct PaginationMeta {
    limit: usize,
    offset: usize,
    total: usize,
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
    message: String,
    field: Option<String>,
}

/// XZepr API Client
struct XzeprClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl XzeprClient {
    fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
        }
    }

    fn build_request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.request(method, &url);

        if let Some(ref api_key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        request.header("Content-Type", "application/json")
    }

    async fn handle_response<T: for<'de> Deserialize<'de>>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, Box<dyn Error>> {
        let status = response.status();
        let body = response.text().await?;

        if status.is_success() {
            Ok(serde_json::from_str(&body)?)
        } else {
            let error: ErrorResponse = serde_json::from_str(&body)
                .unwrap_or_else(|_| ErrorResponse {
                    error: "unknown_error".to_string(),
                    message: body,
                    field: None,
                });
            Err(format!("API Error ({}): {}", status, error.message).into())
        }
    }

    async fn health_check(&self) -> Result<Value, Box<dyn Error>> {
        let response = self.build_request(reqwest::Method::GET, "/health").send().await?;
        self.handle_response(response).await
    }

    async fn create_event_receiver(
        &self,
        name: &str,
        receiver_type: &str,
        version: &str,
        description: &str,
    ) -> Result<String, Box<dyn Error>> {
        let schema = json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Event message"
                },
                "timestamp": {
                    "type": "string",
                    "format": "date-time",
                    "description": "Event timestamp"
                },
                "source": {
                    "type": "string",
                    "description": "Event source"
                }
            },
            "required": ["message"]
        });

        let request = CreateEventReceiverRequest {
            name: name.to_string(),
            receiver_type: receiver_type.to_string(),
            version: version.to_string(),
            description: description.to_string(),
            schema,
        };

        let response = self
            .build_request(reqwest::Method::POST, "/api/v1/receivers")
            .json(&request)
            .send()
            .await?;

        let result: CreateEventReceiverResponse = self.handle_response(response).await?;
        Ok(result.data)
    }

    async fn get_event_receiver(&self, id: &str) -> Result<EventReceiverResponse, Box<dyn Error>> {
        let response = self
            .build_request(reqwest::Method::GET, &format!("/api/v1/receivers/{}", id))
            .send()
            .await?;

        self.handle_response(response).await
    }

    async fn list_event_receivers(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<PaginatedResponse<EventReceiverResponse>, Box<dyn Error>> {
        let response = self
            .build_request(
                reqwest::Method::GET,
                &format!("/api/v1/receivers?limit={}&offset={}", limit, offset),
            )
            .send()
            .await?;

        self.handle_response(response).await
    }

    async fn create_event(
        &self,
        name: &str,
        version: &str,
        release: &str,
        platform_id: &str,
        package: &str,
        description: &str,
        success: bool,
        event_receiver_id: &str,
        payload: Option<&str>,
    ) -> Result<String, Box<dyn Error>> {
        let payload_value = if let Some(payload_str) = payload {
            serde_json::from_str(payload_str)?
        } else {
            json!({
                "message": format!("Event {} created", name),
                "timestamp": Utc::now().to_rfc3339(),
                "source": "xzepr-client"
            })
        };

        let request = CreateEventRequest {
            name: name.to_string(),
            version: version.to_string(),
            release: release.to_string(),
            platform_id: platform_id.to_string(),
            package: package.to_string(),
            description: description.to_string(),
            payload: payload_value,
            success,
            event_receiver_id: event_receiver_id.to_string(),
        };

        let response = self
            .build_request(reqwest::Method::POST, "/api/v1/events")
            .json(&request)
            .send()
            .await?;

        let result: CreateEventResponse = self.handle_response(response).await?;
        Ok(result.data)
    }

    async fn get_event(&self, id: &str) -> Result<EventResponse, Box<dyn Error>> {
        let response = self
            .build_request(reqwest::Method::GET, &format!("/api/v1/events/{}", id))
            .send()
            .await?;

        self.handle_response(response).await
    }

    async fn create_event_receiver_group(
        &self,
        name: &str,
        group_type: &str,
        version: &str,
        description: &str,
        enabled: bool,
        receiver_ids: Vec<String>,
    ) -> Result<String, Box<dyn Error>> {
        let request = CreateEventReceiverGroupRequest {
            name: name.to_string(),
            group_type: group_type.to_string(),
            version: version.to_string(),
            description: description.to_string(),
            enabled,
            event_receiver_ids: receiver_ids,
        };

        let response = self
            .build_request(reqwest::Method::POST, "/api/v1/groups")
            .json(&request)
            .send()
            .await?;

        let result: CreateEventReceiverGroupResponse = self.handle_response(response).await?;
        Ok(result.data)
    }

    async fn get_event_receiver_group(&self, id: &str) -> Result<EventReceiverGroupResponse, Box<dyn Error>> {
        let response = self
            .build_request(reqwest::Method::GET, &format!("/api/v1/groups/{}", id))
            .send()
            .await?;

        self.handle_response(response).await
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let cli = Cli::parse();
    let client = XzeprClient::new(cli.base_url, cli.api_key);

    match cli.command {
        Commands::Health => {
            let health = client.health_check().await?;
            println!("Health check: {}", serde_json::to_string_pretty(&health)?);
        }
        Commands::Receiver { action } => match action {
            ReceiverCommands::Create {
                name,
                receiver_type,
                version,
                description,
            } => {
                let id = client
                    .create_event_receiver(&name, &receiver_type, &version, &description)
                    .await?;
                println!("Created event receiver with ID: {}", id);
            }
            ReceiverCommands::Get { id } => {
                let receiver = client.get_event_receiver(&id).await?;
                println!("{}", serde_json::to_string_pretty(&receiver)?);
            }
            ReceiverCommands::List { limit, offset } => {
                let receivers = client.list_event_receivers(limit, offset).await?;
                println!("{}", serde_json::to_string_pretty(&receivers)?);
            }
        },
        Commands::Event { action } => match action {
            EventCommands::Create {
                name,
                version,
                release,
                platform_id,
                package,
                description,
                success,
                event_receiver_id,
                payload,
            } => {
                let id = client
                    .create_event(
                        &name,
                        &version,
                        &release,
                        &platform_id,
                        &package,
                        &description,
                        success,
                        &event_receiver_id,
                        payload.as_deref(),
                    )
                    .await?;
                println!("Created event with ID: {}", id);
            }
            EventCommands::Get { id } => {
                let event = client.get_event(&id).await?;
                println!("{}", serde_json::to_string_pretty(&event)?);
            }
        },
        Commands::Group { action } => match action {
            GroupCommands::Create {
                name,
                group_type,
                version,
                description,
                enabled,
                receiver_ids,
            } => {
                let ids: Vec<String> = receiver_ids
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                let id = client
                    .create_event_receiver_group(&name, &group_type, &version, &description, enabled, ids)
                    .await?;
                println!("Created event receiver group with ID: {}", id);
            }
            GroupCommands::Get { id } => {
                let group = client.get_event_receiver_group(&id).await?;
                println!("{}", serde_json::to_string_pretty(&group)?);
            }
        },
        Commands::Generate {
            receivers,
            events_per_receiver,
        } => {
            println!("Generating {} receivers with {} events each...", receivers, events_per_receiver);

            let mut receiver_ids = Vec::new();

            // Create event receivers
            for i in 0..receivers {
                let name = format!("test-receiver-{}", i);
                let receiver_type = match i % 3 {
                    0 => "webhook".to_string(),
                    1 => "queue".to_string(),
                    _ => "stream".to_string(),
                };
                let version = format!("1.{}.0", i % 10);
                let description = format!("Test event receiver {} for demonstration", i);

                let id = client
                    .create_event_receiver(&name, &receiver_type, &version, &description)
                    .await?;

                println!("Created receiver {}: {}", name, id);
                receiver_ids.push(id);
            }

            // Create events for each receiver
            for receiver_id in &receiver_ids {
                for j in 0..events_per_receiver {
                    let name = format!("test-event-{}", j);
                    let version = "1.0.0";
                    let release = format!("2024.01.{:02}", (j % 31) + 1);
                    let platform_id = match j % 3 {
                        0 => "linux",
                        1 => "windows",
                        _ => "macos",
                    };
                    let package = "test-package";
                    let description = format!("Test event {} for receiver {}", j, receiver_id);
                    let success = j % 4 != 3; // 75% success rate

                    let event_id = client
                        .create_event(
                            &name,
                            version,
                            &release,
                            platform_id,
                            package,
                            &description,
                            success,
                            receiver_id,
                            None,
                        )
                        .await?;

                    println!("Created event {}: {}", name, event_id);
                }
            }

            // Create a group with all receivers
            if !receiver_ids.is_empty() {
                let group_id = client
                    .create_event_receiver_group(
                        "test-group",
                        "integration",
                        "1.0.0",
                        "Test group containing all generated receivers",
                        true,
                        receiver_ids,
                    )
                    .await?;

                println!("Created group: {}", group_id);
            }

            println!("Data generation completed!");
        }
    }

    Ok(())
}
