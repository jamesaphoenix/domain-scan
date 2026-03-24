//! Adversarial test suite for input validation.
//!
//! Tests exercise specific hallucination patterns that AI agents produce
//! in practice: path traversal, control characters, embedded query params,
//! JSON bombs, regex DoS, unicode edge cases, symlink escapes, and
//! concurrent scan race conditions.

mod adversarial {
    mod path_traversal;
    mod control_chars;
    mod embedded_query_params;
    mod json_bombs;
    mod regex_dos;
    mod unicode_edge_cases;
    mod symlink_escapes;
    mod concurrent_scans;
}
