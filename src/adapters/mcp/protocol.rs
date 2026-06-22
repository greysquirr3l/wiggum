//! MCP protocol version negotiation and dual-shape response helpers.
//!
//! Wiggum speaks both MCP `2025-11-25` and the upcoming `draft` revision in
//! parallel so that old and new clients can connect to the same server. The
//! active version is resolved per-request from
//! `params._meta["io.modelcontextprotocol/protocolVersion"]`, defaulting to
//! the draft when absent (the draft protocol is stateless by default and no
//! longer carries a session-level handshake).
//!
//! Backwards compatibility strategy:
//!
//! * `initialize` is retained and always emits the 2025-11-25 shape. Clients
//!   that still send the handshake get a valid response regardless of which
//!   protocol they would otherwise speak.
//! * `server/discover` is the new entry point for draft clients. It advertises
//!   the set of supported protocol versions and server capabilities.
//! * Every other method responds based on the `_meta` version, defaulting to
//!   the draft when no version is advertised. Draft responses include the
//!   `resultType: "complete"` field that 2025-11-25 responses omit.

use serde::Serialize;
use serde_json::Value;

/// `_meta` key carrying the per-request protocol version.
pub const PROTOCOL_VERSION_META_KEY: &str = "io.modelcontextprotocol/protocolVersion";

/// MCP protocol revisions this server understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum McpVersion {
    /// Stable `2025-11-25` revision (current published spec).
    #[serde(rename = "2025-11-25")]
    V2025_11_25,
    /// Upcoming `draft` revision. The literal string `"draft"` is used until
    /// the revision is published and dated, at which point this arm is
    /// renamed to match the new release label.
    #[serde(rename = "draft")]
    V2026_07Draft,
}

impl McpVersion {
    /// Wire label for this version.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V2025_11_25 => "2025-11-25",
            Self::V2026_07Draft => "draft",
        }
    }

    /// Resolve a version from the request `_meta` block.
    ///
    /// Missing `_meta` defaults to the legacy 2025-11-25 revision: clients
    /// that don't advertise a version are assumed to be older clients that
    /// never learned to send it, so we keep them on the legacy shape rather
    /// than surprise them with new fields.
    ///
    /// Unknown version strings map to the draft so that future revisions
    /// remain connectable while the server is being updated.
    #[must_use]
    pub fn from_meta(meta: Option<&Value>) -> Self {
        let Some(meta) = meta else {
            return Self::V2025_11_25;
        };
        let Some(label) = meta.get(PROTOCOL_VERSION_META_KEY).and_then(Value::as_str) else {
            return Self::V2025_11_25;
        };
        Self::from_label(label)
    }

    /// Map a wire label to the enum. Unknown labels collapse to the draft.
    #[must_use]
    pub fn from_label(label: &str) -> Self {
        if label == Self::V2025_11_25.as_str() {
            Self::V2025_11_25
        } else {
            Self::V2026_07Draft
        }
    }

    /// Protocol versions advertised via `server/discover`. Newest first so
    /// draft clients pick the latest revision when they accept the default.
    #[must_use]
    pub const fn supported() -> &'static [Self] {
        &[Self::V2026_07Draft, Self::V2025_11_25]
    }
}

/// Per-request context used to shape outgoing responses.
#[derive(Debug, Clone, Copy)]
pub struct ResponseContext {
    pub version: McpVersion,
}

impl ResponseContext {
    #[must_use]
    pub const fn new(version: McpVersion) -> Self {
        Self { version }
    }

    /// Whether the response must include `resultType: "complete"`.
    #[must_use]
    pub const fn requires_result_type(self) -> bool {
        matches!(self.version, McpVersion::V2026_07Draft)
    }

    /// Wrap a result payload with version-specific fields.
    ///
    /// For draft responses, every result must carry `resultType`. The payload
    /// is merged into an object if it is not already one, so primitive
    /// results still receive the required field.
    #[must_use]
    pub fn shape_result(self, payload: Value) -> Value {
        match self.version {
            McpVersion::V2025_11_25 => payload,
            McpVersion::V2026_07Draft => inject_result_type(payload, "complete"),
        }
    }
}

fn inject_result_type(payload: Value, result_type: &str) -> Value {
    match payload {
        Value::Object(mut map) => {
            map.entry("resultType".to_string())
                .or_insert(Value::String(result_type.to_string()));
            Value::Object(map)
        }
        other => {
            let mut map = serde_json::Map::new();
            map.insert(
                "resultType".to_string(),
                Value::String(result_type.to_string()),
            );
            map.insert("value".to_string(), other);
            Value::Object(map)
        }
    }
}

/// Per-list cache hints. Draft responses include `ttlMs` and `cacheScope`
/// alongside list/read results so clients can avoid redundant polling.
#[derive(Debug, Clone, Copy)]
pub struct CacheHint {
    pub ttl_ms: u64,
    pub cache_scope: CacheScope,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheScope {
    Public,
    Private,
}

impl CacheHint {
    /// Default hint for `tools/list` — the catalogue does not change for the
    /// lifetime of a server process.
    #[must_use]
    pub const fn tools_list() -> Self {
        Self {
            ttl_ms: 3_600_000,
            cache_scope: CacheScope::Public,
        }
    }

    #[must_use]
    pub fn to_json(self) -> Value {
        serde_json::json!({
            "ttlMs": self.ttl_ms,
            "cacheScope": match self.cache_scope {
                CacheScope::Public => "public",
                CacheScope::Private => "private",
            }
        })
    }
}

/// Negotiate the protocol version for an incoming request.
///
/// `initialize` always resolves to 2025-11-25 because the handshake was
/// removed in the draft revision — clients that still send `initialize` are
/// necessarily speaking the older protocol. All other methods honour the
/// `_meta` version tag, falling back to the draft.
#[must_use]
pub fn negotiate(method: &str, params: &Value) -> McpVersion {
    if method == "initialize" {
        return McpVersion::V2025_11_25;
    }
    McpVersion::from_meta(params.get("_meta"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_label_round_trips() {
        for version in McpVersion::supported() {
            assert_eq!(McpVersion::from_label(version.as_str()), *version);
        }
    }

    #[test]
    fn unknown_label_collapses_to_draft() {
        assert_eq!(
            McpVersion::from_label("2099-01-01"),
            McpVersion::V2026_07Draft
        );
    }

    #[test]
    fn missing_meta_defaults_to_legacy() {
        assert_eq!(McpVersion::from_meta(None), McpVersion::V2025_11_25);
    }

    #[test]
    fn empty_object_meta_defaults_to_legacy() {
        let meta = serde_json::json!({});
        assert_eq!(McpVersion::from_meta(Some(&meta)), McpVersion::V2025_11_25);
    }

    #[test]
    fn meta_with_known_version_is_respected() {
        let meta = serde_json::json!({
            PROTOCOL_VERSION_META_KEY: "2025-11-25"
        });
        assert_eq!(McpVersion::from_meta(Some(&meta)), McpVersion::V2025_11_25);
    }

    #[test]
    fn meta_with_unknown_version_collapses_to_draft() {
        let meta = serde_json::json!({
            PROTOCOL_VERSION_META_KEY: "2099-01-01"
        });
        assert_eq!(
            McpVersion::from_meta(Some(&meta)),
            McpVersion::V2026_07Draft
        );
    }

    #[test]
    fn initialize_always_negotiates_2025_11_25() {
        // Even when the client sends a draft `_meta`, `initialize` is the
        // legacy handshake and must respond with the legacy shape.
        let meta = serde_json::json!({
            PROTOCOL_VERSION_META_KEY: "draft"
        });
        let params = serde_json::json!({ "_meta": meta });
        assert_eq!(negotiate("initialize", &params), McpVersion::V2025_11_25);
    }

    #[test]
    fn shape_result_is_passthrough_for_2025_11_25() {
        let ctx = ResponseContext::new(McpVersion::V2025_11_25);
        let payload = serde_json::json!({ "tools": [] });
        assert_eq!(ctx.shape_result(payload.clone()), payload);
    }

    #[test]
    fn shape_result_injects_result_type_for_draft() {
        let ctx = ResponseContext::new(McpVersion::V2026_07Draft);
        let payload = serde_json::json!({ "tools": [] });
        let shaped = ctx.shape_result(payload);

        assert_eq!(
            shaped.get("resultType").and_then(Value::as_str),
            Some("complete")
        );
        assert!(shaped.get("tools").is_some());
    }

    #[test]
    fn shape_result_wraps_primitive_payloads() {
        let ctx = ResponseContext::new(McpVersion::V2026_07Draft);
        let shaped = ctx.shape_result(Value::String("hi".to_string()));

        assert_eq!(
            shaped.get("resultType").and_then(Value::as_str),
            Some("complete")
        );
        assert_eq!(shaped.get("value").and_then(Value::as_str), Some("hi"));
    }

    #[test]
    fn shape_result_does_not_overwrite_existing_result_type() {
        let ctx = ResponseContext::new(McpVersion::V2026_07Draft);
        let payload = serde_json::json!({
            "resultType": "input_required",
            "inputRequests": []
        });
        let shaped = ctx.shape_result(payload);

        assert_eq!(
            shaped.get("resultType").and_then(Value::as_str),
            Some("input_required")
        );
    }

    #[test]
    fn cache_hint_serializes_both_fields() {
        let hint = CacheHint::tools_list();
        let json = hint.to_json();

        assert_eq!(json.get("ttlMs").and_then(Value::as_u64), Some(3_600_000));
        assert_eq!(
            json.get("cacheScope").and_then(Value::as_str),
            Some("public")
        );
    }

    #[test]
    fn supported_includes_both_versions() {
        let versions = McpVersion::supported();
        assert!(versions.contains(&McpVersion::V2025_11_25));
        assert!(versions.contains(&McpVersion::V2026_07Draft));
    }
}
