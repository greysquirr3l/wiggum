use thiserror::Error;

#[derive(Debug, Error)]
pub enum WiggumError {
    #[error("plan parse error: {0}")]
    PlanParse(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("dependency cycle detected: {0}")]
    CycleDetected(String),

    #[error("unknown task dependency: {referenced} (referenced by {referencing})")]
    UnknownDependency {
        referenced: String,
        referencing: String,
    },

    #[error("duplicate task slug: {0}")]
    DuplicateSlug(String),

    #[error("template error: {0}")]
    Template(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, WiggumError>;
