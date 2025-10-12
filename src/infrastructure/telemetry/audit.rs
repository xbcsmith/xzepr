// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/auth/audit.rs

use serde::{Serialize, Deserialize};

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct AuditLog {
    pub id: AuditLogId,
    pub user_id: Option<UserId>,
    pub action: AuditAction,
    pub resource: Option<String>,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    Login,
    Logout,
    LoginFailed,
    PermissionDenied,
    ApiKeyGenerated,
    ApiKeyRevoked,
    RoleAssigned,
    RoleRemoved,
    PasswordChanged,
    UserCreated,
    UserDisabled,
}

pub struct AuditLogger {
    repo: Arc<dyn AuditLogRepository>,
}

impl AuditLogger {
    pub async fn log_auth_event(
        &self,
        user_id: Option<UserId>,
        action: AuditAction,
        success: bool,
        ip_address: Option<IpAddr>,
        user_agent: Option<String>,
        error: Option<String>,
    ) -> Result<(), AuditError> {
        let log = AuditLog {
            id: AuditLogId::new(),
            user_id,
            action,
            resource: None,
            ip_address,
            user_agent,
            success,
            error_message: error,
            created_at: Utc::now(),
        };
        
        self.repo.save(&log).await?;
        
        // Also emit metric
        if success {
            metrics::counter!("auth_success_total", "action" => format!("{:?}", log.action)).increment(1);
        } else {
            metrics::counter!("auth_failure_total", "action" => format!("{:?}", log.action)).increment(1);
        }
        
        Ok(())
    }
}