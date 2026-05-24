// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! User identifier value object.

crate::define_ulid_id!(
    UserId,
    "ULID-backed unique identifier for a user aggregate."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_user_id_unique() {
        assert_ne!(UserId::new(), UserId::new());
    }

    #[test]
    fn test_user_id_parse_valid() {
        let id = UserId::new();
        // SAFETY: id was just created, its string form is always a valid ULID
        let parsed = UserId::parse(&id.to_string()).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_user_id_from_string() {
        let id = UserId::new();
        // SAFETY: id was just created, its string form is always a valid ULID
        let parsed = UserId::from_string(id.as_str()).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_user_id_parse_invalid() {
        assert!(UserId::parse("not-a-ulid").is_err());
    }

    #[test]
    fn test_user_id_from_ulid() {
        let ulid = ulid::Ulid::new();
        let id = UserId::from_ulid(ulid);
        assert_eq!(id.as_ulid(), ulid);
    }

    #[test]
    fn test_user_id_display() {
        let ulid = ulid::Ulid::new();
        let id = UserId::from_ulid(ulid);
        assert_eq!(id.to_string(), ulid.to_string());
    }

    #[test]
    fn test_user_id_serde_roundtrip() {
        let id = UserId::new();
        // SAFETY: UserId serializes to a ULID string which is always valid JSON
        let json = serde_json::to_string(&id).unwrap();
        // SAFETY: we just serialized this value, it must deserialize cleanly
        let back: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn test_user_id_default_non_empty() {
        assert!(!UserId::default().as_str().is_empty());
    }

    #[test]
    fn test_user_id_timestamp_range() {
        let ts = UserId::new().timestamp_ms();
        assert!(ts > 1_577_836_800_000);
        assert!(ts < 2_000_000_000_000);
    }

    #[test]
    fn test_user_id_ordering_by_time() {
        let id1 = UserId::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id2 = UserId::new();
        assert!(id2.timestamp_ms() >= id1.timestamp_ms());
    }
}
