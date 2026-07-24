//! Subscription resolvers — stubbed until Day 5 when Apollo Router is wired.
//!
//! When implemented, these will:
//!   1. Open a SurrealDB LIVE SELECT on the nodes table
//!   2. Push events through a tokio broadcast channel
//!   3. Stream to clients via WebSocket through Apollo Router
//!
//! Implementation plan (Day 5):
//!   - Add `live_sync` module that holds a broadcast::Sender<GraphEvent>
//!   - Background task: LIVE SELECT → send to broadcast channel
//!   - Subscription resolver: broadcast::Receiver → async_stream
//!   - Apollo Router config: enable WebSocket subscriptions

use async_graphql::*;
use crate::graphql::types::GqlNode;

pub struct SubscriptionRoot;

#[Subscription(name = "Subscription")]
impl SubscriptionRoot {
    /// Fires when a node is inserted for the given owner.
    /// Stubbed — returns empty stream until Day 5.
    async fn node_inserted(
        &self,
        _ctx:      &Context<'_>,
        _owner_id: String,
    ) -> impl futures_util::Stream<Item = GqlNode> {
        futures_util::stream::empty()
    }

    /// Fires when a node is updated for the given owner.
    async fn node_updated(
        &self,
        _ctx:      &Context<'_>,
        _owner_id: String,
    ) -> impl futures_util::Stream<Item = GqlNode> {
        futures_util::stream::empty()
    }
}