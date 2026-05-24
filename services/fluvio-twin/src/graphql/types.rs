//! GraphQL types for fluvio-twin subgraph.

use async_graphql::*;

#[derive(SimpleObject, Clone, Debug)]
pub struct GqlChatResponse {
    pub answer:  String,
    pub sources: Vec<GqlSource>,
}

#[derive(SimpleObject, Clone, Debug)]
pub struct GqlSource {
    pub id:    String,
    pub page:  String,
    pub score: f64,
    pub text:  String,
}

#[derive(SimpleObject, Clone, Debug)]
pub struct GqlDocument {
    pub id:      String,
    pub title:   String,
    pub kind:    String,
    pub domain:  String,
    pub excerpt: String,
    pub zone:    i32,
}

#[derive(InputObject, Clone, Debug)]
pub struct GqlChatMessage {
    pub role:    String,
    pub content: String,
}