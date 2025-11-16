// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/rest/group_membership.rs

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use tracing::{error, info, warn};

use crate::api::middleware::jwt::AuthenticatedUser;
use crate::api::rest::dtos::{
    AddMemberRequest, ErrorResponse, GroupMemberResponse, GroupMembersResponse, RemoveMemberRequest,
};
use crate::application::handlers::EventReceiverGroupHandler;
use crate::domain::value_objects::{EventReceiverGroupId, UserId};

/// Application state containing the event receiver group handler
#[derive(Clone)]
pub struct GroupMembershipState {
    pub group_handler: EventReceiverGroupHandler,
}

/// Adds a member to an event receiver group
///
/// # Arguments
///
/// * `group_id` - The ID of the group to add the member to
/// * `user` - The authenticated user making the request (must be group owner)
/// * `request` - The request body containing the user_id to add
///
/// # Returns
///
/// Returns a JSON response with the added member information on success,
/// or an error response on failure.
///
/// # Errors
///
/// * `400 BAD_REQUEST` - Invalid request data or validation failure
/// * `401 UNAUTHORIZED` - Invalid authentication token
/// * `403 FORBIDDEN` - User is not authorized to add members to this group
/// * `404 NOT_FOUND` - Group not found
/// * `409 CONFLICT` - User is already a member of the group
/// * `500 INTERNAL_SERVER_ERROR` - Unexpected server error
pub async fn add_group_member(
    State(state): State<GroupMembershipState>,
    Path(group_id): Path<String>,
    user: AuthenticatedUser,
    Json(request): Json<AddMemberRequest>,
) -> Result<Json<GroupMemberResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id_str = user.user_id();
    info!(
        group_id = %group_id,
        member_to_add = %request.user_id,
        added_by = %user_id_str,
        "Adding member to group"
    );

    // Validate request
    if let Err(err) = request.validate() {
        warn!("Add member validation failed: {:?}", err);
        return Err((StatusCode::BAD_REQUEST, Json(err)));
    }

    // Parse group ID
    let group_id = match group_id.parse::<EventReceiverGroupId>() {
        Ok(id) => id,
        Err(e) => {
            warn!("Invalid group ID format: {}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "ValidationError".to_string(),
                    "Invalid group ID format".to_string(),
                )),
            ));
        }
    };

    // Parse authenticated user ID (who is adding the member)
    let added_by = match UserId::parse(user_id_str) {
        Ok(id) => id,
        Err(e) => {
            error!("Invalid user ID in JWT token: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "InternalError".to_string(),
                    "Invalid user ID in authentication token".to_string(),
                )),
            ));
        }
    };

    // Parse user ID to add
    let user_id = match request.parse_user_id() {
        Ok(id) => id,
        Err(err) => {
            warn!("Invalid user ID to add: {:?}", err);
            return Err((StatusCode::BAD_REQUEST, Json(err)));
        }
    };

    // Check if group exists and user is owner
    let group = match state.group_handler.find_group_by_id(group_id).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            warn!("Group not found: {}", group_id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NotFound".to_string(),
                    "Group not found".to_string(),
                )),
            ));
        }
        Err(e) => {
            error!("Failed to fetch group: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "InternalError".to_string(),
                    "Failed to fetch group information".to_string(),
                )),
            ));
        }
    };

    // Verify the authenticated user owns the group
    if group.owner_id() != added_by {
        warn!(
            "User {} is not authorized to add members to group {}",
            added_by, group_id
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "Forbidden".to_string(),
                "Only the group owner can add members".to_string(),
            )),
        ));
    }

    // Add the member
    match state
        .group_handler
        .add_group_member(group_id, user_id, added_by)
        .await
    {
        Ok(_) => {
            info!(
                "Successfully added user {} to group {} by {}",
                user_id, group_id, added_by
            );

            // Return success response with member info
            // Note: In a real implementation, we would fetch user details
            // from a user service or repository. For now, we return basic info.
            Ok(Json(GroupMemberResponse {
                user_id: user_id.to_string(),
                username: format!("user_{}", user_id), // Placeholder
                email: format!("{}@example.com", user_id), // Placeholder
                added_at: chrono::Utc::now(),
                added_by: added_by.to_string(),
            }))
        }
        Err(e) => {
            error!("Failed to add member to group: {}", e);

            // Check for specific error types
            let error_msg = e.to_string();
            if error_msg.contains("already a member") || error_msg.contains("duplicate") {
                return Err((
                    StatusCode::CONFLICT,
                    Json(ErrorResponse::new(
                        "Conflict".to_string(),
                        "User is already a member of this group".to_string(),
                    )),
                ));
            }

            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "InternalError".to_string(),
                    "Failed to add member".to_string(),
                )),
            ))
        }
    }
}

/// Removes a member from an event receiver group
///
/// # Arguments
///
/// * `group_id` - The ID of the group to remove the member from
/// * `user` - The authenticated user making the request (must be group owner)
/// * `request` - The request body containing the user_id to remove
///
/// # Returns
///
/// Returns a 204 NO_CONTENT on success, or an error response on failure.
///
/// # Errors
///
/// * `400 BAD_REQUEST` - Invalid request data or validation failure
/// * `401 UNAUTHORIZED` - Invalid authentication token
/// * `403 FORBIDDEN` - User is not authorized to remove members from this group
/// * `404 NOT_FOUND` - Group not found or user is not a member
/// * `500 INTERNAL_SERVER_ERROR` - Unexpected server error
pub async fn remove_group_member(
    State(state): State<GroupMembershipState>,
    Path(group_id): Path<String>,
    user: AuthenticatedUser,
    Json(request): Json<RemoveMemberRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let user_id_str = user.user_id();
    info!(
        group_id = %group_id,
        member_to_remove = %request.user_id,
        removed_by = %user_id_str,
        "Removing member from group"
    );

    // Validate request
    if let Err(err) = request.validate() {
        warn!("Remove member validation failed: {:?}", err);
        return Err((StatusCode::BAD_REQUEST, Json(err)));
    }

    // Parse group ID
    let group_id = match group_id.parse::<EventReceiverGroupId>() {
        Ok(id) => id,
        Err(e) => {
            warn!("Invalid group ID format: {}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "ValidationError".to_string(),
                    "Invalid group ID format".to_string(),
                )),
            ));
        }
    };

    // Parse authenticated user ID (who is removing the member)
    let removed_by = match UserId::parse(user_id_str) {
        Ok(id) => id,
        Err(e) => {
            error!("Invalid user ID in JWT token: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "InternalError".to_string(),
                    "Invalid user ID in authentication token".to_string(),
                )),
            ));
        }
    };

    // Parse user ID to remove
    let user_id = match request.parse_user_id() {
        Ok(id) => id,
        Err(err) => {
            warn!("Invalid user ID to remove: {:?}", err);
            return Err((StatusCode::BAD_REQUEST, Json(err)));
        }
    };

    // Check if group exists and user is owner
    let group = match state.group_handler.find_group_by_id(group_id).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            warn!("Group not found: {}", group_id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NotFound".to_string(),
                    "Group not found".to_string(),
                )),
            ));
        }
        Err(e) => {
            error!("Failed to fetch group: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "InternalError".to_string(),
                    "Failed to fetch group information".to_string(),
                )),
            ));
        }
    };

    // Verify the authenticated user owns the group
    if group.owner_id() != removed_by {
        warn!(
            "User {} is not authorized to remove members from group {}",
            removed_by, group_id
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "Forbidden".to_string(),
                "Only the group owner can remove members".to_string(),
            )),
        ));
    }

    // Remove the member
    match state
        .group_handler
        .remove_group_member(group_id, user_id)
        .await
    {
        Ok(_) => {
            info!(
                "Successfully removed user {} from group {} by {}",
                user_id, group_id, removed_by
            );
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!("Failed to remove member from group: {}", e);

            let error_msg = e.to_string();
            if error_msg.contains("not a member") || error_msg.contains("not found") {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new(
                        "NotFound".to_string(),
                        "User is not a member of this group".to_string(),
                    )),
                ));
            }

            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "InternalError".to_string(),
                    "Failed to remove member".to_string(),
                )),
            ))
        }
    }
}

/// Lists all members of an event receiver group
///
/// # Arguments
///
/// * `group_id` - The ID of the group to list members for
/// * `user` - The authenticated user making the request
///
/// # Returns
///
/// Returns a JSON response with all group members on success,
/// or an error response on failure.
///
/// # Errors
///
/// * `400 BAD_REQUEST` - Invalid group ID format
/// * `401 UNAUTHORIZED` - Invalid authentication token
/// * `403 FORBIDDEN` - User is not authorized to view members (not owner or member)
/// * `404 NOT_FOUND` - Group not found
/// * `500 INTERNAL_SERVER_ERROR` - Unexpected server error
pub async fn list_group_members(
    State(state): State<GroupMembershipState>,
    Path(group_id): Path<String>,
    user: AuthenticatedUser,
) -> Result<Json<GroupMembersResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id_str = user.user_id();
    info!(
        group_id = %group_id,
        requested_by = %user_id_str,
        "Listing group members"
    );

    // Parse group ID
    let group_id = match group_id.parse::<EventReceiverGroupId>() {
        Ok(id) => id,
        Err(e) => {
            warn!("Invalid group ID format: {}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "ValidationError".to_string(),
                    "Invalid group ID format".to_string(),
                )),
            ));
        }
    };

    // Parse authenticated user ID
    let requesting_user_id = match UserId::parse(user_id_str) {
        Ok(id) => id,
        Err(e) => {
            error!("Invalid user ID in JWT token: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "InternalError".to_string(),
                    "Invalid user ID in authentication token".to_string(),
                )),
            ));
        }
    };

    // Check if group exists
    let group = match state.group_handler.find_group_by_id(group_id).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            warn!("Group not found: {}", group_id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NotFound".to_string(),
                    "Group not found".to_string(),
                )),
            ));
        }
        Err(e) => {
            error!("Failed to fetch group: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "InternalError".to_string(),
                    "Failed to fetch group information".to_string(),
                )),
            ));
        }
    };

    // Verify the user is authorized (owner or member)
    let is_owner = group.owner_id() == requesting_user_id;
    let is_member = match state
        .group_handler
        .is_group_member(group_id, requesting_user_id)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to check group membership: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "InternalError".to_string(),
                    "Failed to check authorization".to_string(),
                )),
            ));
        }
    };

    if !is_owner && !is_member {
        warn!(
            "User {} is not authorized to view members of group {}",
            requesting_user_id, group_id
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "Forbidden".to_string(),
                "Only group owners and members can view the member list".to_string(),
            )),
        ));
    }

    // Get all members
    match state.group_handler.get_group_members(group_id).await {
        Ok(member_ids) => {
            info!("Found {} members in group {}", member_ids.len(), group_id);

            // Convert user IDs to member responses
            // Note: In a real implementation, we would fetch full user details
            // from a user service or repository. For now, we return basic info.
            let members: Vec<GroupMemberResponse> = member_ids
                .iter()
                .map(|uid| GroupMemberResponse {
                    user_id: uid.to_string(),
                    username: format!("user_{}", uid), // Placeholder
                    email: format!("{}@example.com", uid), // Placeholder
                    added_at: chrono::Utc::now(),      // Placeholder
                    added_by: group.owner_id().to_string(), // Placeholder
                })
                .collect();

            Ok(Json(GroupMembersResponse {
                group_id: group_id.to_string(),
                members,
            }))
        }
        Err(e) => {
            error!("Failed to fetch group members: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "InternalError".to_string(),
                    "Failed to fetch group members".to_string(),
                )),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_membership_state_clone() {
        // This test ensures GroupMembershipState can be cloned,
        // which is required for Axum state
        use crate::application::handlers::EventReceiverGroupHandler;
        use std::sync::Arc;

        // Create mock repositories
        use crate::domain::repositories::EventReceiverGroupRepository;
        use crate::domain::repositories::EventReceiverRepository;

        // Note: This is a compile-time test to ensure Clone is implemented
        // In practice, we'd use actual mock repositories for testing
        struct MockGroupRepo;
        struct MockReceiverRepo;

        #[async_trait::async_trait]
        impl EventReceiverGroupRepository for MockGroupRepo {
            async fn save(
                &self,
                _group: &crate::domain::entities::event_receiver_group::EventReceiverGroup,
            ) -> crate::error::Result<()> {
                unimplemented!()
            }

            async fn find_by_id(
                &self,
                _id: EventReceiverGroupId,
            ) -> crate::error::Result<
                Option<crate::domain::entities::event_receiver_group::EventReceiverGroup>,
            > {
                unimplemented!()
            }

            async fn find_by_name(
                &self,
                _name: &str,
            ) -> crate::error::Result<
                Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>,
            > {
                unimplemented!()
            }

            async fn find_by_type(
                &self,
                _group_type: &str,
            ) -> crate::error::Result<
                Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>,
            > {
                unimplemented!()
            }

            async fn find_by_type_and_version(
                &self,
                _group_type: &str,
                _version: &str,
            ) -> crate::error::Result<
                Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>,
            > {
                unimplemented!()
            }

            async fn find_enabled(
                &self,
            ) -> crate::error::Result<
                Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>,
            > {
                unimplemented!()
            }

            async fn find_disabled(
                &self,
            ) -> crate::error::Result<
                Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>,
            > {
                unimplemented!()
            }

            async fn find_by_event_receiver_id(
                &self,
                _receiver_id: EventReceiverId,
            ) -> crate::error::Result<
                Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>,
            > {
                unimplemented!()
            }

            async fn list(
                &self,
                _limit: usize,
                _offset: usize,
            ) -> crate::error::Result<
                Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>,
            > {
                unimplemented!()
            }

            async fn count(&self) -> crate::error::Result<usize> {
                unimplemented!()
            }

            async fn count_enabled(&self) -> crate::error::Result<usize> {
                unimplemented!()
            }

            async fn count_disabled(&self) -> crate::error::Result<usize> {
                unimplemented!()
            }

            async fn update(
                &self,
                _group: &crate::domain::entities::event_receiver_group::EventReceiverGroup,
            ) -> crate::error::Result<()> {
                unimplemented!()
            }

            async fn delete(&self, _id: EventReceiverGroupId) -> crate::error::Result<()> {
                unimplemented!()
            }

            async fn enable(&self, _id: EventReceiverGroupId) -> crate::error::Result<()> {
                unimplemented!()
            }

            async fn disable(&self, _id: EventReceiverGroupId) -> crate::error::Result<()> {
                unimplemented!()
            }

            async fn exists_by_name_and_type(
                &self,
                _name: &str,
                _group_type: &str,
            ) -> crate::error::Result<bool> {
                unimplemented!()
            }

            async fn find_by_criteria(
                &self,
                _criteria: crate::domain::repositories::event_receiver_group_repo::FindEventReceiverGroupCriteria,
            ) -> crate::error::Result<
                Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>,
            > {
                unimplemented!()
            }

            async fn add_event_receiver_to_group(
                &self,
                _group_id: EventReceiverGroupId,
                _receiver_id: EventReceiverId,
            ) -> crate::error::Result<()> {
                unimplemented!()
            }

            async fn remove_event_receiver_from_group(
                &self,
                _group_id: EventReceiverGroupId,
                _receiver_id: EventReceiverId,
            ) -> crate::error::Result<()> {
                unimplemented!()
            }

            async fn get_group_event_receivers(
                &self,
                _group_id: EventReceiverGroupId,
            ) -> crate::error::Result<Vec<EventReceiverId>> {
                unimplemented!()
            }

            async fn find_by_owner(
                &self,
                _owner_id: UserId,
            ) -> crate::error::Result<
                Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>,
            > {
                unimplemented!()
            }

            async fn find_by_owner_paginated(
                &self,
                _owner_id: UserId,
                _limit: usize,
                _offset: usize,
            ) -> crate::error::Result<
                Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>,
            > {
                unimplemented!()
            }

            async fn is_owner(
                &self,
                _group_id: EventReceiverGroupId,
                _user_id: UserId,
            ) -> crate::error::Result<bool> {
                unimplemented!()
            }

            async fn get_resource_version(
                &self,
                _group_id: EventReceiverGroupId,
            ) -> crate::error::Result<Option<i64>> {
                unimplemented!()
            }

            async fn is_member(
                &self,
                _group_id: EventReceiverGroupId,
                _user_id: UserId,
            ) -> crate::error::Result<bool> {
                unimplemented!()
            }

            async fn get_group_members(
                &self,
                _group_id: EventReceiverGroupId,
            ) -> crate::error::Result<Vec<UserId>> {
                unimplemented!()
            }

            async fn add_member(
                &self,
                _group_id: EventReceiverGroupId,
                _user_id: UserId,
                _added_by: UserId,
            ) -> crate::error::Result<()> {
                unimplemented!()
            }

            async fn remove_member(
                &self,
                _group_id: EventReceiverGroupId,
                _user_id: UserId,
            ) -> crate::error::Result<()> {
                unimplemented!()
            }

            async fn find_groups_for_user(
                &self,
                _user_id: UserId,
            ) -> crate::error::Result<
                Vec<crate::domain::entities::event_receiver_group::EventReceiverGroup>,
            > {
                unimplemented!()
            }
        }

        #[async_trait::async_trait]
        impl EventReceiverRepository for MockReceiverRepo {
            async fn save(
                &self,
                _receiver: &crate::domain::entities::event_receiver::EventReceiver,
            ) -> crate::error::Result<()> {
                unimplemented!()
            }

            async fn find_by_id(
                &self,
                _id: EventReceiverId,
            ) -> crate::error::Result<Option<crate::domain::entities::event_receiver::EventReceiver>>
            {
                unimplemented!()
            }

            async fn find_by_name(
                &self,
                _name: &str,
            ) -> crate::error::Result<Vec<crate::domain::entities::event_receiver::EventReceiver>>
            {
                unimplemented!()
            }

            async fn find_by_type(
                &self,
                _receiver_type: &str,
            ) -> crate::error::Result<Vec<crate::domain::entities::event_receiver::EventReceiver>>
            {
                unimplemented!()
            }

            async fn find_by_type_and_version(
                &self,
                _receiver_type: &str,
                _version: &str,
            ) -> crate::error::Result<Vec<crate::domain::entities::event_receiver::EventReceiver>>
            {
                unimplemented!()
            }

            async fn list(
                &self,
                _limit: usize,
                _offset: usize,
            ) -> crate::error::Result<Vec<crate::domain::entities::event_receiver::EventReceiver>>
            {
                unimplemented!()
            }

            async fn count(&self) -> crate::error::Result<usize> {
                unimplemented!()
            }

            async fn update(
                &self,
                _receiver: &crate::domain::entities::event_receiver::EventReceiver,
            ) -> crate::error::Result<()> {
                unimplemented!()
            }

            async fn delete(&self, _id: EventReceiverId) -> crate::error::Result<()> {
                unimplemented!()
            }

            async fn exists_by_name_and_type(
                &self,
                _name: &str,
                _receiver_type: &str,
            ) -> crate::error::Result<bool> {
                unimplemented!()
            }

            async fn find_by_criteria(
                &self,
                _criteria: crate::domain::repositories::event_receiver_repo::FindEventReceiverCriteria,
            ) -> crate::error::Result<Vec<crate::domain::entities::event_receiver::EventReceiver>>
            {
                unimplemented!()
            }

            async fn find_by_owner(
                &self,
                _owner_id: UserId,
            ) -> crate::error::Result<Vec<crate::domain::entities::event_receiver::EventReceiver>>
            {
                unimplemented!()
            }

            async fn find_by_owner_paginated(
                &self,
                _owner_id: UserId,
                _limit: usize,
                _offset: usize,
            ) -> crate::error::Result<Vec<crate::domain::entities::event_receiver::EventReceiver>>
            {
                unimplemented!()
            }

            async fn is_owner(
                &self,
                _receiver_id: EventReceiverId,
                _user_id: UserId,
            ) -> crate::error::Result<bool> {
                unimplemented!()
            }

            async fn get_resource_version(
                &self,
                _receiver_id: EventReceiverId,
            ) -> crate::error::Result<Option<i64>> {
                unimplemented!()
            }
        }

        let handler = EventReceiverGroupHandler::new(
            Arc::new(MockGroupRepo),
            Arc::new(MockReceiverRepo),
            None,
        );

        let state = GroupMembershipState {
            group_handler: handler,
        };

        let _cloned_state = state.clone();
    }
}
