pub mod mutation;
pub mod query;
pub mod types;

use async_graphql::{EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{routing::get, Router};
use uuid::Uuid;

use crate::server::AppState;
use mutation::MutationRoot;
use query::QueryRoot;

pub type DatabaseSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(state: AppState) -> DatabaseSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(state)
        .enable_federation()
        .finish()
}

pub fn graphql_router(schema: DatabaseSchema) -> Router {
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
                            .and_then(|s| Uuid::parse_str(s).ok());

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
            .finish(),
    )
}