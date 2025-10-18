# Curl

In this section of the workshop we will use curl to make requests to the XZEPR.

---

## Create using the REST API

First thing we need is an event receiver. The event receiver acts as a
classification and gate for events.

The `schema` field defines the structure expected for event payloads. You can
use:

- An empty schema `{}` for free-form payloads (no validation)
- A basic schema `{"type": "object"}` to require object payloads
- A full JSON Schema with properties and validation rules

Create an event receiver with schema validation:

```bash
curl --location --request POST 'http://localhost:8443/api/v1/receivers' \
--header 'Content-Type: application/json' \
--data-raw '{
  "name": "foobar",
  "type": "foo.bar",
  "version": "1.1.3",
  "description": "The event receiver of Brixton",
  "schema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string"
      }
    }
  }
}'
```

The results should look like this:

```json
{ "data": "01HPW0DY340VMM3DNMX8JCQDGN" }
```

---

We need the ULID of the event receiver in the next step.

When you create an event, you must specify an `event_receiver_id` to associate
it with. An event is the record of some action being completed. You cannot
create an event for a non-existent receiver ID. If the event receiver has a
schema defined (non-empty), the payload field of the event should conform to
that schema. If the event receiver has an empty schema `{}`, any valid JSON
object is accepted as the payload.

Create an event:

```bash
curl --location --request POST 'http://localhost:8443/api/v1/events' \
--header 'Content-Type: application/json' \
--data-raw '{
  "name": "magnificent",
  "version": "7.0.1",
  "release": "2023.11.16",
  "platform_id": "linux",
  "package": "docker",
  "description": "blah",
  "payload": {
    "name": "joe"
  },
  "success": true,
  "event_receiver_id": "<PASTE EVENT RECEIVER ID FROM FIRST CURL COMMAND>"
}
```

The results of the command should look like this:

```json
{ "data": "01HPW0GV9PY8HT2Q0XW1QMRBY9" }
```

---

Event Receiver Groups are a way to group together several event receivers. When
all the event receivers in a group have successful events for a given unit the
event receiver group will produce a message on the topic.

Create an event receiver group:

```bash
curl --location --request POST 'http://localhost:8443/api/v1/groups' \
--header 'Content-Type: application/json' \
--data-raw '{
  "name": "the_clash",
  "type": "foo.bar",
  "version": "3.3.3",
  "description": "The only event receiver group that matters",
  "enabled": true,
  "event_receiver_ids": [
    "PASTE EVENT RECEIVER ID FROM FIRST CURL COMMAND"
  ]
}
```

Note: You can extract the event receiver id from the previous command by pipe
the output to `| jq .data`

---

## Schema Flexibility Example

Create an event receiver with an empty schema for free-form payloads:

```bash
curl --location --request POST 'http://localhost:8443/api/v1/receivers' \
--header 'Content-Type: application/json' \
--data-raw '{
  "name": "flexible-receiver",
  "type": "any.event.type",
  "version": "1.0.0",
  "description": "Accepts any valid JSON payload",
  "schema": {}
}'
```

This receiver will accept events with any payload structure, making it ideal for
evolving event formats or CDEvents with varying structures.

---

## Query using the REST API

We can query the event information using a GET on the events endpoint as
follows:

```bash
curl --header 'Content-Type: application/json' --location \
  --request GET 'http://localhost:8443/api/v1/events/01HPW0GV9PY8HT2Q0XW1QMRBY9'
```

Query the information for an event receiver:

```bash
curl --header 'Content-Type: application/json' --location \
  --request GET 'http://localhost:8443/api/v1/receivers/01HPW0DY340VMM3DNMX8JCQDGN'
```

And query the information for an event receiver group:

```bash
curl --header 'Content-Type: application/json' --location \
  --request GET 'http://localhost:8443/api/v1/groups/01HPW0JXG82Q0FBEC9M8P2Q6J8
'
```

---
