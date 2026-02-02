//! Dead Letter Queue for Failed Events
//!
//! Handles events that failed processing:
//! - Store failed events with error information
//! - Retry failed events
//! - Query and manage dead letters
//! - Statistics and monitoring

mod handler;
mod storage;

pub use handler::{DeadLetterQueue, DLQConfig, DLQStats};
pub use storage::{DLQStorage, InMemoryDLQStorage, DLQQuery, DeadLetter};
