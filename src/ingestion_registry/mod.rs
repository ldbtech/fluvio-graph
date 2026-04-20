pub mod email;
pub mod connector;

pub use connector::{
    ConnectorError,
    FluvioConnector,
    NormalizedChunk,
    PreDefinedEdge,
};
 