//! Information-restricted calibration solvers. NO runtime dependencies on
//! the authoritative protocol simulator, world kernel, or organism VM.
//! The only bridge to these systems is in development-only test adapters.
#![forbid(unsafe_code)]
pub mod restricted_p2;
