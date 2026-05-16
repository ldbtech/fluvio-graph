//! GraphQL schema assembly for fluvio-graph subgraph.

pub mod mutation;
pub mod query;
pub mod subscription;
pub mod types;

use async_graphql::Schema;
use async_graphql_axum::{GraphQL, GraphQLSubscription};
use axum::{routing::get, Router};
use uuid::Uuid;

use crate::server::AppState;
use mutation::MutationRoot;
use query::QueryRoot;
use subscription::SubscriptionRoot;

pub type FluvioGraphSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

pub fn build_schema(state: AppState) -> FluvioGraphSchema {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(state)
        .finish()
}

pub fn graphql_router(schema: FluvioGraphSchema) -> Router {
    Router::new()
        .route(
            "/graphql",
            get(graphiql).post(
                move |headers: axum::http::HeaderMap,
                      req: async_graphql_axum::GraphQLRequest| {
                    let schema = schema.clone();
                    async move {
                        let user_id = headers
                            .get("x-user-id")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| uuid::Uuid::parse_str(s).ok());

                        let mut request = req.into_inner();
                        if let Some(uid) = user_id {
                            request = request.data(uid);
                        }
                        async_graphql_axum::GraphQLResponse::from(
                            schema.execute(request).await
                        )
                    }
                }
            ),
        )
}

async fn graphiql() -> impl axum::response::IntoResponse {
    axum::response::Html(
        async_graphql::http::GraphiQLSource::build()
            .endpoint("/graphql")
            .subscription_endpoint("/graphql/ws")
            .finish(),
    )
}

pub fn extract_user_id_from_headers(headers: &axum::http::HeaderMap) -> Option<Uuid> {
    headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
}