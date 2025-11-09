# examples validation

This document provides a comprehensive validation summary of the example files
in the XZepr project, ensuring they remain compatible with the current API
implementation.

## overview

The XZepr project includes three sets of examples demonstrating API usage:

1. `docs/tutorials/curl_examples.md` - Tutorial showing curl commands for API
   interaction
2. `examples/generate_epr_events.py` - Python script for generating CDEvents
3. `examples/xzepr_client.rs` - Rust CLI client for API operations

This validation confirms these examples align with the API implementation in
`src/api/rest/dtos.rs` and domain entities in `src/domain/entities/`.

## validation results

### curl examples tutorial

**Status**: All examples are valid and correct.

#### create event receiver

```bash
POST http://localhost:8443/api/v1/receivers
```

**Validation**:

- name: "foobar" (valid, non-empty string)
- type: "foo.bar" (valid, non-empty string)
- version: "1.1.3" (valid, non-empty string)
- description: "The event receiver of Brixton" (valid)
- schema: Contains "type" field and "properties" (valid JSON Schema)

**Response Format**: Correctly shows `{ "data": "ULID" }`

**Result**: Passes all validation requirements in
`CreateEventReceiverRequest::validate()` and
`EventReceiver::validate_schema()`.

#### create event

```bash
POST http://localhost:8443/api/v1/events
```

**Validation**:

- name: "magnificent" (valid)
- version: "7.0.1" (valid)
- release: "2023.11.16" (valid)
- platform_id: "linux" (valid)
- package: "docker" (valid)
- description: "blah" (valid)
- payload: `{"name": "joe"}` (valid JSON object)
- success: true (valid boolean)
- event_receiver_id: Placeholder for ULID (valid pattern)

**Response Format**: Correctly shows `{ "data": "ULID" }`

**Result**: Passes all validation requirements in
`CreateEventRequest::validate()`.

#### create event receiver group

```bash
POST http://localhost:8443/api/v1/groups
```

**Validation**:

- name: "the_clash" (valid)
- type: "foo.bar" (valid)
- version: "3.3.3" (valid)
- description: "The only event receiver group that matters" (valid)
- enabled: true (valid boolean)
- event_receiver_ids: Array with placeholder (valid pattern)

**Response Format**: Correctly shows expected response structure

**Result**: Passes all validation requirements in
`CreateEventReceiverGroupRequest::validate()`.

#### query endpoints

All GET endpoints use correct paths and expected response formats:

- `GET /api/v1/events/{id}`
- `GET /api/v1/receivers/{id}`
- `GET /api/v1/groups/{id}`

### python client validation

**File**: `examples/generate_epr_events.py`

**Status**: All validations pass correctly.

#### generate event receivers function

The `generate_event_receivers()` function creates event receivers with an empty
schema object:

```python
evr = dict(name=name, version=version, description=description,
           type=event_type, schema={})
```

**Validation**: Empty schemas `{}` are valid according to JSON Schema
specification. An empty schema means "accept any valid JSON," which allows for
free-form event payloads without constraints.

The domain validation in
`src/domain/entities/event_receiver.rs::validate_schema()` only requires:

```rust
/// Validates JSON schema
fn validate_schema(schema: &JsonValue) -> Result<(), DomainError> {
    // Ensure it's a valid JSON object
    // An empty schema {} is valid and means "accept any valid JSON"
    if !schema.is_object() {
        return Err(DomainError::ValidationError {
            field: "schema".to_string(),
            message: "Schema must be a valid JSON object".to_string(),
        });
    }

    // Empty schema is valid - no further validation needed
    // This allows for free-form event payloads
    Ok(())
}
```

**Schema Flexibility**: The implementation supports multiple schema patterns:

- `{}` - Empty schema, accepts any valid JSON object (used in Python script)
- `{"type": "object"}` - Basic schema requiring object type
- Full JSON Schema with properties, required fields, etc.

**Result**: The empty schema approach is intentionally designed to support
free-form CDEvents payloads without strict schema validation.

#### generate events function

**Validation**:

- name: Generated from names list (valid)
- version: "1.0.0" (valid)
- release: Timestamp-based format (valid)
- platform_id: "x64-linux-oci-2" (valid)
- package: "oci" (valid)
- description: Generated descriptive string (valid)
- payload: Complex CDEvents structure as object (valid)
- success: true (valid)
- event_receiver_id: Retrieved from evrs dictionary (valid pattern)

**Result**: Passes all validation requirements in `CreateEventRequest`.

#### http client functions

The `post_event_receiver()` and `post_event()` functions correctly:

- Use proper endpoints (`/api/v1/receivers`, `/api/v1/events`)
- Set `Content-Type: application/json` header
- Parse response with `data` field containing ULID

**Overall Assessment**: Fully functional and compliant with API requirements.

### rust client validation

**File**: `examples/xzepr_client.rs`

**Status**: All structures and operations are valid.

#### data structures

All request structures correctly match the API DTOs:

**CreateEventReceiverRequest**:

```rust
struct CreateEventReceiverRequest {
    name: String,
    #[serde(rename = "type")]
    receiver_type: String,
    version: String,
    description: String,
    schema: Value,
}
```

- Correctly uses `#[serde(rename = "type")]` for JSON field mapping
- All required fields present

**CreateEventRequest**:

```rust
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
```

- All fields match API requirements
- ULID passed as string

**CreateEventReceiverGroupRequest**:

```rust
struct CreateEventReceiverGroupRequest {
    name: String,
    #[serde(rename = "type")]
    group_type: String,
    version: String,
    description: String,
    enabled: bool,
    event_receiver_ids: Vec<String>,
}
```

- Correctly uses `#[serde(rename = "type")]`
- Array of ULIDs as strings

#### response structures

All response structures correctly expect:

```rust
struct CreateEventReceiverResponse { data: String }
struct CreateEventResponse { data: String }
struct CreateEventReceiverGroupResponse { data: String }
```

This matches the API response format: `{ "data": "ULID" }`

#### client implementation

The `XzeprClient` implementation:

- Correctly builds requests with proper HTTP methods
- Uses appropriate endpoints
- Handles responses correctly
- Includes proper error handling

**Result**: Fully compliant with API implementation.

## schema validation design rationale

### why empty schemas are valid

The XZepr implementation intentionally allows empty schemas `{}` for event
receivers. This design decision supports several important use cases:

1. **Free-form Event Payloads**: Not all events have a predefined structure.
   CDEvents and other event formats may evolve over time.

2. **JSON Schema Specification**: According to JSON Schema specification, an
   empty schema `{}` is valid and means "any valid JSON is acceptable."

3. **Flexibility**: Event receivers can be created without schema constraints,
   then optionally updated with specific schemas as requirements become clear.

4. **Backward Compatibility**: Systems can accept events without breaking when
   event structures change or expand.

### schema validation levels

The implementation supports three levels of schema enforcement:

**Level 1: No Constraints** (Empty Schema)

```json
{
  "schema": {}
}
```

Accepts any valid JSON object as event payload.

**Level 2: Basic Type Constraint**

```json
{
  "schema": {
    "type": "object"
  }
}
```

Requires payload to be a JSON object (no additional constraints).

**Level 3: Full Schema Validation**

```json
{
  "schema": {
    "type": "object",
    "properties": {
      "context": {
        "type": "object",
        "required": ["version", "id", "type"]
      },
      "subject": {
        "type": "object",
        "required": ["id", "type"]
      }
    },
    "required": ["context", "subject"]
  }
}
```

Enforces specific structure and required fields.

### implementation details

The validation logic is minimal and permissive:

**Domain Layer** (`src/domain/entities/event_receiver.rs`):

- Only validates that schema is a JSON object
- Allows empty objects
- Does not require specific fields

**API Layer** (`src/api/rest/dtos.rs`):

- Same validation as domain layer
- Ensures consistency across layers

**Tests**: Explicit tests verify empty schemas are accepted:

- `test_empty_schema_is_valid()` - Verifies `{}` is accepted
- `test_schema_without_type_field_is_valid()` - Verifies schemas without
  "type" are accepted

## recommendations

### best practices for schema usage

1. **Start Permissive**: Use empty schemas `{}` during development or when
   event structure is evolving
2. **Add Constraints Gradually**: Add schema validation as requirements become
   clear
3. **Document Intent**: Include description explaining whether schema is
   intentionally empty or will be added later
4. **Version Appropriately**: Update event receiver versions when adding or
   changing schemas

### maintenance tasks

1. **Schema Migration Path**: Consider adding schema versioning support for
   evolving event structures
2. **Integration Tests**: Add tests that validate examples against running
   server
3. **Documentation**: Keep examples in sync with API changes through automated
   testing
4. **Error Messages**: Enhance validation error messages to guide users toward
   correct schema formats

## validation methodology

This validation was performed by:

1. Reading API DTO definitions in `src/api/rest/dtos.rs`
2. Examining domain entity validation in `src/domain/entities/`
3. Cross-referencing request structures in all three example files
4. Verifying endpoint paths match routing in `src/api/rest/routes.rs`
5. Confirming response formats align with implementation
6. Running test suite to verify schema validation behavior
7. Analyzing JSON Schema specification for empty schema semantics

## conclusion

All three example files are fully valid and compliant with the XZepr API
implementation:

- **curl_examples.md**: Production-ready tutorial with clear instructions
- **xzepr_client.rs**: Fully compliant Rust CLI client with proper error
  handling
- **generate_epr_events.py**: Correct implementation using empty schemas for
  flexible CDEvents generation

The empty schema pattern in the Python script is an intentional and valid
design choice that provides maximum flexibility for event payload structures.
No changes are required to any of the example files.
