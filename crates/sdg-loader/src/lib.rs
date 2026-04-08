pub mod error;
pub mod schema;
pub mod types;
pub mod validation;

// Re-export key types
pub use error::SdgError;
pub use types::ServiceDefinition;
pub use validation::load;
