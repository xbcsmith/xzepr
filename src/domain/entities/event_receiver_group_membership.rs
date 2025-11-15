// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/domain/entities/event_receiver_group_membership.rs

use crate::domain::value_objects::{EventReceiverGroupId, UserId};
use crate::error::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Parameters for creating a group membership from existing data
#[derive(Debug, Clone)]
pub struct GroupMembershipData {
    pub group_id: EventReceiverGroupId,
    pub user_id: UserId,
    pub added_by: UserId,
    pub added_at: DateTime<Utc>,
}

/// Event receiver group membership entity representing user access to a group
///
/// This entity tracks which users have permission to POST events to event receivers
/// that belong to a specific group. Only the group owner or authorized members can
/// modify group membership.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventReceiverGroupMembership {
    group_id: EventReceiverGroupId,
    user_id: UserId,
    added_by: UserId,
    added_at: DateTime<Utc>,
}

impl EventReceiverGroupMembership {
    /// Creates a new group membership with validation
    ///
    /// # Arguments
    ///
    /// * `group_id` - The ID of the event receiver group
    /// * `user_id` - The ID of the user being added to the group
    /// * `added_by` - The ID of the user who is adding this member
    ///
    /// # Returns
    ///
    /// Returns a new EventReceiverGroupMembership or DomainError if validation fails
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::domain::entities::event_receiver_group_membership::EventReceiverGroupMembership;
    /// use xzepr::domain::value_objects::{EventReceiverGroupId, UserId};
    ///
    /// let group_id = EventReceiverGroupId::new();
    /// let user_id = UserId::new();
    /// let added_by = UserId::new();
    ///
    /// let membership = EventReceiverGroupMembership::new(group_id, user_id, added_by);
    /// assert!(membership.is_ok());
    /// ```
    pub fn new(
        group_id: EventReceiverGroupId,
        user_id: UserId,
        added_by: UserId,
    ) -> Result<Self, DomainError> {
        Self::validate_membership(group_id, user_id, added_by)?;

        Ok(Self {
            group_id,
            user_id,
            added_by,
            added_at: Utc::now(),
        })
    }

    /// Creates a group membership from existing data (e.g., from database)
    ///
    /// # Arguments
    ///
    /// * `data` - Data structure containing all membership fields
    ///
    /// # Returns
    ///
    /// Returns a reconstructed EventReceiverGroupMembership or DomainError if validation fails
    pub fn from_existing(data: GroupMembershipData) -> Result<Self, DomainError> {
        Self::validate_membership(data.group_id, data.user_id, data.added_by)?;

        Ok(Self {
            group_id: data.group_id,
            user_id: data.user_id,
            added_by: data.added_by,
            added_at: data.added_at,
        })
    }

    /// Validates the membership data
    ///
    /// Ensures that a user cannot add themselves to a group as a regular member.
    /// Only the group owner (validated elsewhere) should be able to add themselves.
    fn validate_membership(
        _group_id: EventReceiverGroupId,
        user_id: UserId,
        added_by: UserId,
    ) -> Result<(), DomainError> {
        // Prevent users from adding themselves as members
        // The group owner is implicitly a member and doesn't need explicit membership
        if user_id == added_by {
            return Err(DomainError::BusinessRuleViolation {
                rule:
                    "Users cannot add themselves to a group. Only the group owner can add members."
                        .to_string(),
            });
        }

        Ok(())
    }

    /// Checks if this membership matches a specific user and group
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group ID to check
    /// * `user_id` - The user ID to check
    ///
    /// # Returns
    ///
    /// Returns true if this membership matches both the group and user IDs
    pub fn matches(&self, group_id: EventReceiverGroupId, user_id: UserId) -> bool {
        self.group_id == group_id && self.user_id == user_id
    }

    // Getters

    /// Returns the group ID
    pub fn group_id(&self) -> EventReceiverGroupId {
        self.group_id
    }

    /// Returns the user ID of the member
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// Returns the user ID of who added this member
    pub fn added_by(&self) -> UserId {
        self.added_by
    }

    /// Returns when this membership was created
    pub fn added_at(&self) -> DateTime<Utc> {
        self.added_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_group_membership() {
        let group_id = EventReceiverGroupId::new();
        let user_id = UserId::new();
        let added_by = UserId::new();

        let membership = EventReceiverGroupMembership::new(group_id, user_id, added_by);

        assert!(membership.is_ok());
        let membership = membership.unwrap();
        assert_eq!(membership.group_id(), group_id);
        assert_eq!(membership.user_id(), user_id);
        assert_eq!(membership.added_by(), added_by);
    }

    #[test]
    fn test_user_cannot_add_themselves() {
        let group_id = EventReceiverGroupId::new();
        let user_id = UserId::new();

        // Try to add themselves
        let result = EventReceiverGroupMembership::new(group_id, user_id, user_id);

        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::BusinessRuleViolation { rule } => {
                assert!(rule.contains("cannot add themselves"));
            }
            _ => panic!("Expected BusinessRuleViolation"),
        }
    }

    #[test]
    fn test_matches_correct_group_and_user() {
        let group_id = EventReceiverGroupId::new();
        let user_id = UserId::new();
        let added_by = UserId::new();

        let membership = EventReceiverGroupMembership::new(group_id, user_id, added_by).unwrap();

        assert!(membership.matches(group_id, user_id));
    }

    #[test]
    fn test_does_not_match_different_group() {
        let group_id = EventReceiverGroupId::new();
        let other_group_id = EventReceiverGroupId::new();
        let user_id = UserId::new();
        let added_by = UserId::new();

        let membership = EventReceiverGroupMembership::new(group_id, user_id, added_by).unwrap();

        assert!(!membership.matches(other_group_id, user_id));
    }

    #[test]
    fn test_does_not_match_different_user() {
        let group_id = EventReceiverGroupId::new();
        let user_id = UserId::new();
        let other_user_id = UserId::new();
        let added_by = UserId::new();

        let membership = EventReceiverGroupMembership::new(group_id, user_id, added_by).unwrap();

        assert!(!membership.matches(group_id, other_user_id));
    }

    #[test]
    fn test_from_existing_data() {
        let group_id = EventReceiverGroupId::new();
        let user_id = UserId::new();
        let added_by = UserId::new();
        let added_at = Utc::now();

        let data = GroupMembershipData {
            group_id,
            user_id,
            added_by,
            added_at,
        };

        let membership = EventReceiverGroupMembership::from_existing(data);

        assert!(membership.is_ok());
        let membership = membership.unwrap();
        assert_eq!(membership.group_id(), group_id);
        assert_eq!(membership.user_id(), user_id);
        assert_eq!(membership.added_by(), added_by);
        assert_eq!(membership.added_at(), added_at);
    }

    #[test]
    fn test_membership_added_at_is_set() {
        let group_id = EventReceiverGroupId::new();
        let user_id = UserId::new();
        let added_by = UserId::new();

        let before = Utc::now();
        let membership = EventReceiverGroupMembership::new(group_id, user_id, added_by).unwrap();
        let after = Utc::now();

        assert!(membership.added_at() >= before);
        assert!(membership.added_at() <= after);
    }

    #[test]
    fn test_membership_clone() {
        let group_id = EventReceiverGroupId::new();
        let user_id = UserId::new();
        let added_by = UserId::new();

        let membership1 = EventReceiverGroupMembership::new(group_id, user_id, added_by).unwrap();
        let membership2 = membership1.clone();

        assert_eq!(membership1.group_id(), membership2.group_id());
        assert_eq!(membership1.user_id(), membership2.user_id());
        assert_eq!(membership1.added_by(), membership2.added_by());
        assert_eq!(membership1.added_at(), membership2.added_at());
    }

    #[test]
    fn test_membership_serialization() {
        let group_id = EventReceiverGroupId::new();
        let user_id = UserId::new();
        let added_by = UserId::new();

        let membership = EventReceiverGroupMembership::new(group_id, user_id, added_by).unwrap();

        let serialized = serde_json::to_string(&membership);
        assert!(serialized.is_ok());

        let json_str = serialized.unwrap();
        assert!(json_str.contains("group_id"));
        assert!(json_str.contains("user_id"));
        assert!(json_str.contains("added_by"));
    }

    #[test]
    fn test_membership_deserialization() {
        let group_id = EventReceiverGroupId::new();
        let user_id = UserId::new();
        let added_by = UserId::new();

        let membership = EventReceiverGroupMembership::new(group_id, user_id, added_by).unwrap();

        let serialized = serde_json::to_string(&membership).unwrap();
        let deserialized: Result<EventReceiverGroupMembership, _> =
            serde_json::from_str(&serialized);

        assert!(deserialized.is_ok());
        let deserialized_membership = deserialized.unwrap();
        assert_eq!(deserialized_membership.group_id(), membership.group_id());
        assert_eq!(deserialized_membership.user_id(), membership.user_id());
    }

    #[test]
    fn test_different_memberships_are_different() {
        let group_id = EventReceiverGroupId::new();
        let user_id1 = UserId::new();
        let user_id2 = UserId::new();
        let added_by = UserId::new();

        let membership1 = EventReceiverGroupMembership::new(group_id, user_id1, added_by).unwrap();
        let membership2 = EventReceiverGroupMembership::new(group_id, user_id2, added_by).unwrap();

        assert_ne!(membership1, membership2);
    }
}
