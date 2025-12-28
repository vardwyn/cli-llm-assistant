use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delimiter {
    pub start: String,
    pub end: String,
}
