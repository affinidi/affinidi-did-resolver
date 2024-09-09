use rand::{distributions::Alphanumeric, Rng};
use serde::{de::DeserializeOwned, Serialize};

/// Helps with deserializing the generic data field in the SuccessResponse struct
pub trait GenericDataStruct: DeserializeOwned + Serialize {}

// Creates a random transaction identifier for each transaction
pub(crate) fn create_session_id() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect()
}
