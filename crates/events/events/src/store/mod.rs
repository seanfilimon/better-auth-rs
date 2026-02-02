//! Event Store - Persistence layer for events
//!
//! Provides event sourcing capabilities including:
//! - Event persistence and retrieval
//! - Stream-based event storage
//! - Event querying and filtering
//! - Snapshot support for performance

mod trait_def;
mod memory;
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "redis")]
mod redis;

pub use trait_def::{
    EventStore, EventQuery, EventOrdering, EventStream, 
    EventStreamSubscription, EventId, StreamVersion, StoredEvent, EventSnapshot
};
pub use memory::MemoryEventStore;

#[cfg(feature = "postgres")]
pub use postgres::PostgresEventStore;

#[cfg(feature = "redis")]
pub use redis::RedisEventStore;
