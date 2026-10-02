//! The authorization contract shared by every compiled-in evaluator.
//!
//! `hand_rolled` and `casbin` are mutually exclusive at the `cfg` level
//! — exactly one is compiled into any given build — so neither can
//! call into the other. That is precisely why the two had drifted, and
//! the drift was not cosmetic:
//!
//! - the casbin adapter could not reuse the hand-rolled object parser
//!   and grew its own, which hardcoded `ResourceType::Canvas`; a
//!   request for a workspace object was evaluated against the canvas
//!   policy rows, and an object with no recognised prefix was
//!   authorized instead of denied.
//! - the hand-rolled evaluator gated `Delete` behind the ownership
//!   flag while the casbin matcher could not, because every row in
//!   `policies/base_policy.csv` set `is_owner = "*"`, which makes
//!   `(p.is_owner == r.is_owner || p.is_owner == "*")` true
//!   unconditionally. `role:owner` could therefore delete anything.
//!
//! Both evaluators now resolve the resource type and decide ownership
//! through this module, so the public `enforce` surface cannot mean one
//! thing under one evaluator and something else under the other.
//!
//! Everything here is deliberately fail-closed: an input that cannot be
//! resolved resolves to `None`, and every caller turns that into a deny.

use ada_m11_rbac_collab::{Action, ResourceType};

/// Does this action require the caller to own the object?
///
/// `Delete` is destructive and irreversible from the RBAC layer's point
/// of view, so it is restricted to the object's own principal. Every
/// other action is governed purely by the role ladder.
///
/// This lives here rather than in either evaluator because the answer
/// is a security boundary, not an implementation detail: when the two
/// evaluators disagreed about it, the stricter one was the hand-rolled
/// evaluator and the looser one — the production evaluator — allowed
/// `role:owner` to delete objects it did not own.
#[must_use]
pub fn requires_ownership(action: Action) -> bool {
    matches!(action, Action::Delete)
}

/// Extract the resource type from a `"<kind>:<id>"` object string.
///
/// `enforce_typed` builds this composite itself; the untyped `enforce`
/// receives it from the caller. Returns `None` when the prefix is
/// absent, names no known resource type, or carries no instance id —
/// so an unrecognised object is denied rather than checked against
/// every resource type in turn, or worse, against a hardcoded default.
#[must_use]
pub fn resource_type_of(object_id: &str) -> Option<ResourceType> {
    let (kind, id) = object_id.split_once(':')?;
    if id.is_empty() {
        return None;
    }
    [
        ResourceType::Canvas,
        ResourceType::Workspace,
        ResourceType::Credential,
    ]
    .into_iter()
    .find(|rt| rt.as_str() == kind)
}

#[cfg(test)]
mod tests {
    use super::{requires_ownership, resource_type_of};
    use ada_m11_rbac_collab::{Action, ResourceType};

    #[test]
    fn delete_is_the_only_ownership_gated_action() {
        assert!(requires_ownership(Action::Delete));
        for action in [
            Action::Read,
            Action::Write,
            Action::Execute,
            Action::ShareManage,
        ] {
            assert!(
                !requires_ownership(action),
                "{action:?} must be governed by the role ladder alone"
            );
        }
    }

    #[test]
    fn each_known_prefix_resolves_to_its_own_resource_type() {
        assert_eq!(resource_type_of("canvas:abc"), Some(ResourceType::Canvas));
        assert_eq!(
            resource_type_of("workspace:abc"),
            Some(ResourceType::Workspace)
        );
        assert_eq!(
            resource_type_of("credential:abc"),
            Some(ResourceType::Credential)
        );
    }

    #[test]
    fn the_per_instance_id_is_not_part_of_the_decision() {
        // The uuid is tenant scoping for the gateway; it must not be
        // able to change which resource type is being checked.
        assert_eq!(
            resource_type_of("workspace:00000000-0000-0000-0000-000000000000"),
            Some(ResourceType::Workspace)
        );
    }

    #[test]
    fn an_unknown_or_malformed_object_resolves_to_nothing() {
        for object in [
            "abc",         // no prefix at all
            "notares:abc", // prefix names no known resource type
            "canvas",      // no separator
            "",            // empty
            ":abc",        // empty prefix
            "canvas:",     // known prefix but no instance id
        ] {
            assert_eq!(
                resource_type_of(object),
                None,
                "object {object:?} must not resolve to a resource type"
            );
        }
    }

    #[test]
    fn case_does_not_flip_the_resource_type() {
        // `as_str()` emits the lowercase canonical names. A caller
        // sending "Canvas" must not be silently treated as something
        // else, and must not be silently treated as a canvas either.
        assert_eq!(resource_type_of("Canvas:abc"), None);
        assert_eq!(resource_type_of("WORKSPACE:abc"), None);
    }
}
