//! Category tree. Only these paths may appear on a Finding. See docs/TAXONOMY.md.
use crate::analyze::Severity;

pub struct Node {
    pub path: &'static str,
    pub default_severity: Severity,
}

pub const NODES: &[Node] = &[
    Node {
        path: "common.secrets.credential",
        default_severity: Severity::Block,
    },
    Node {
        path: "common.secrets.env_file",
        default_severity: Severity::Block,
    },
    Node {
        path: "common.deps.vulnerable",
        default_severity: Severity::Block,
    },
    Node {
        path: "common.debug.leftover",
        default_severity: Severity::Medium,
    },
    Node {
        path: "common.todo.unreferenced",
        default_severity: Severity::Low,
    },
    Node {
        path: "common.process.large_commit",
        default_severity: Severity::Low,
    },
    Node {
        path: "common.process.large_file",
        default_severity: Severity::Low,
    },
    Node {
        path: "common.tests.untested_public_change",
        default_severity: Severity::Medium,
    },
    Node {
        path: "common.tests.assertionless",
        default_severity: Severity::Medium,
    },
    Node {
        path: "common.tests.sleep",
        default_severity: Severity::Low,
    },
    Node {
        path: "go.errors.unchecked",
        default_severity: Severity::High,
    },
    Node {
        path: "go.errors.shadowed",
        default_severity: Severity::Medium,
    },
    Node {
        path: "go.errors.panic_in_lib",
        default_severity: Severity::High,
    },
    Node {
        path: "go.context.background",
        default_severity: Severity::Medium,
    },
    Node {
        path: "go.context.not_propagated",
        default_severity: Severity::Medium,
    },
    Node {
        path: "go.concurrency.lifecycle",
        default_severity: Severity::High,
    },
    Node {
        path: "go.concurrency.shared_state",
        default_severity: Severity::High,
    },
    Node {
        path: "go.concurrency.unbounded",
        default_severity: Severity::Medium,
    },
    Node {
        path: "go.resources.unclosed",
        default_severity: Severity::High,
    },
    Node {
        path: "go.nil.map_write",
        default_severity: Severity::High,
    },
    Node {
        path: "go.nil.type_assert",
        default_severity: Severity::Medium,
    },
    Node {
        path: "go.http.no_timeout",
        default_severity: Severity::Medium,
    },
    Node {
        path: "ts.types.any",
        default_severity: Severity::Low,
    },
    Node {
        path: "ts.types.non_null",
        default_severity: Severity::Low,
    },
    Node {
        path: "ts.types.suppression",
        default_severity: Severity::Medium,
    },
    Node {
        path: "ts.types.exhaustiveness",
        default_severity: Severity::Medium,
    },
    Node {
        path: "ts.async.floating_promise",
        default_severity: Severity::High,
    },
    Node {
        path: "ts.async.foreach_async",
        default_severity: Severity::High,
    },
    Node {
        path: "ts.async.empty_catch",
        default_severity: Severity::High,
    },
    Node {
        path: "ts.async.no_timeout",
        default_severity: Severity::Medium,
    },
    Node {
        path: "ts.runtime.unvalidated_input",
        default_severity: Severity::Medium,
    },
];

pub fn lookup(path: &str) -> Option<&'static Node> {
    NODES.iter().find(|n| n.path == path)
}
