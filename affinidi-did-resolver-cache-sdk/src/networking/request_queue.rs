//! When messages are sent via websocket, the response may be out of order
//! [RequestList] helps manage the buffer and returns the right response

use super::network::Responder;
use crate::config::ClientConfig;
use std::collections::HashMap;
use tracing::debug;

/// List of lookups that are in progress.Note the list is not in any order.
/// NOTE: SHA256 Hash of the DID is used as the key for the list
/// - list: The list of requests waiting for a response from the server (key: DID Hash, value: Vec[(Unique ID, Responder Channel)]
/// - list_full: Is the list full based on limits?
/// - limit_count: The maximum number of items to store in the request list
/// - total_count: The total number of items in the list
///
/// NOTE: Handles duplicate DID resolver requests, by matching them in the list by the DID hash, adds elements using
///       the unique ID as an identifier.
pub(crate) struct RequestList {
    list: HashMap<String, Vec<(String, Responder)>>,
    list_full: bool,
    limit_count: u32,
    total_count: u32,
}

impl RequestList {
    /// Create a new request list
    pub fn new(config: &ClientConfig) -> Self {
        debug!(
            "created request list limit_count({})",
            config.network_cache_limit_count
        );
        Self {
            list: HashMap::new(),
            list_full: false,
            limit_count: config.network_cache_limit_count,
            total_count: 0,
        }
    }

    /// Insert a new request into the list
    /// Returns: true if the request is new, false if it is a duplicate (no need to send to server)
    pub fn insert(&mut self, key: String, uid: &str, channel: Responder) -> bool {
        // If the key exists, append the value to the list
        if let Some(element) = self.list.get_mut(&key) {
            element.push((uid.to_string(), channel));
            debug!(
                "Duplicate resolver request, adding to queue to await response. id ({})",
                key
            );
            false
        } else {
            // Otherwise, create a new list with the value
            self.list
                .insert(key.clone(), vec![(uid.to_string(), channel)]);

            self.total_count += 1;

            if self.total_count > self.limit_count {
                self.list_full = true;
            }

            debug!(
                "Request inserted: id({}) list_count({})",
                key, self.total_count
            );
            true
        }
    }

    /// Remove a response from the list returning the value
    /// ^^ This is why we don't need a get() function...
    /// If uid isn't provided, then all channels for given key are removed
    /// If uid is provided, then we just remove that channel for that key (which if empty will delete the key)
    pub(crate) fn remove(&mut self, key: &str, uid: Option<String>) -> Option<Vec<Responder>> {
        // Get the Responder Channels from the list
        // Request must be in the list itself!

        if let Some(uid) = uid {
            let response = if let Some(channels) = self.list.get_mut(key) {
                // Find the index of the element to remove
                let index = channels.iter().position(|(id, _)| *id == uid);

                if let Some(index) = index {
                    // Remove the element from the list
                    let (_, channel) = channels.remove(index);

                    debug!(
                        "Request removed: id({}) channels_waiting({}) list_count({})",
                        key,
                        channels.len(),
                        self.total_count
                    );
                    Some(vec![channel])
                } else {
                    debug!("Request not found: id({}) unique_id({})", key, uid);
                    None
                }
            } else {
                debug!("Request not found: id({})", key);
                None
            };

            // If the list is empty, remove the key
            if let Some(channels) = self.list.get(key) {
                if channels.is_empty() {
                    self.list.remove(key);
                    self.total_count -= 1;
                    self.list_full = false;
                }
            }

            response
        } else {
            // Remove all channels for the key
            if let Some(channels) = self.list.remove(key) {
                self.total_count -= 1;
                self.list_full = false;

                debug!(
                    "Request removed: hash({}) channels_waiting({}) remaining_list_count({})",
                    key,
                    channels.len(),
                    self.total_count
                );

                Some(channels.into_iter().map(|(_, channel)| channel).collect())
            } else {
                debug!("Request not found: hash({})", key);
                None
            }
        }
    }

    /// Is the list full based on limits?
    pub(crate) fn is_full(&self) -> bool {
        self.list_full
    }
}
