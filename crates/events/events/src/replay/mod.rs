//! Event Replay Engine
//!
//! Provides event sourcing capabilities for replaying events:
//! - Replay events from a specific point in time
//! - Replay at different speeds (fast, realtime, custom)
//! - Filter events during replay
//! - Handle failed events during replay

mod engine;

pub use engine::{ReplayEngine, ReplayConfig, ReplaySpeed, ReplayResult, ReplayStats};
