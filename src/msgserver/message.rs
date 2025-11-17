// Message types and protocol for the message passing system

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

/// A message in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID
    pub id: String,
    /// Client ID of the sender
    pub from: String,
    /// Message content
    pub content: String,
    /// When the message was created
    pub timestamp: SystemTime,
    /// When the message expires
    pub expires_at: SystemTime,
}

impl Message {
    /// Create a new message with TTL
    pub fn new(from: String, content: String, ttl: Duration) -> Self {
        let now = SystemTime::now();
        let id = Uuid::new_v4().to_string();

        Self {
            id,
            from,
            content,
            timestamp: now,
            expires_at: now + ttl,
        }
    }

    /// Check if the message has expired
    pub fn is_expired(&self) -> bool {
        SystemTime::now() >= self.expires_at
    }

    /// Get age of message in seconds
    pub fn age_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(self.timestamp)
            .unwrap_or(Duration::from_secs(0))
            .as_secs()
    }
}

/// Protocol messages exchanged between clients and server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProtocolMessage {
    /// Client authentication request
    Auth {
        code: String,
        client_id: String,
    },
    /// Authentication response
    AuthResponse {
        success: bool,
        message: String,
        server_id: Option<String>,
    },
    /// Client sending a message
    Send {
        content: String,
    },
    /// Server broadcasting a message to clients
    Broadcast {
        message: Message,
    },
    /// Request to receive pending messages
    Receive {
        since: Option<SystemTime>,
    },
    /// Response with messages
    ReceiveResponse {
        messages: Vec<Message>,
    },
    /// Ping to keep connection alive
    Ping,
    /// Pong response
    Pong,
    /// Error message
    Error {
        message: String,
    },
}

impl ProtocolMessage {
    /// Serialize to JSON bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut bytes = serde_json::to_vec(self)?;
        bytes.push(b'\n'); // Add newline delimiter
        Ok(bytes)
    }

    /// Deserialize from JSON bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// In-memory message queue with expiration
pub struct MessageQueue {
    messages: Arc<RwLock<VecDeque<Message>>>,
    ttl: Duration,
    max_size: usize,
}

impl MessageQueue {
    /// Create a new message queue
    pub fn new(ttl: Duration, max_size: usize) -> Self {
        Self {
            messages: Arc::new(RwLock::new(VecDeque::new())),
            ttl,
            max_size,
        }
    }

    /// Add a message to the queue
    pub async fn push(&self, from: String, content: String) -> Message {
        let message = Message::new(from, content, self.ttl);

        let mut queue = self.messages.write().await;

        // Remove expired messages
        queue.retain(|m| !m.is_expired());

        // Enforce max size (FIFO)
        while queue.len() >= self.max_size {
            queue.pop_front();
        }

        queue.push_back(message.clone());
        message
    }

    /// Get all non-expired messages
    pub async fn get_all(&self) -> Vec<Message> {
        let mut queue = self.messages.write().await;

        // Remove expired messages
        queue.retain(|m| !m.is_expired());

        queue.iter().cloned().collect()
    }

    /// Get messages since a specific time
    pub async fn get_since(&self, since: SystemTime) -> Vec<Message> {
        let mut queue = self.messages.write().await;

        // Remove expired messages
        queue.retain(|m| !m.is_expired());

        queue
            .iter()
            .filter(|m| m.timestamp >= since)
            .cloned()
            .collect()
    }

    /// Get the number of active messages
    pub async fn len(&self) -> usize {
        let mut queue = self.messages.write().await;

        // Remove expired messages
        queue.retain(|m| !m.is_expired());

        queue.len()
    }

    /// Clear all messages
    pub async fn clear(&self) {
        let mut queue = self.messages.write().await;
        queue.clear();
    }

    /// Start background task to clean up expired messages
    pub fn start_cleanup_task(self: Arc<Self>, interval: Duration) {
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                let mut queue = self.messages.write().await;
                let before = queue.len();
                queue.retain(|m| !m.is_expired());
                let after = queue.len();

                if before != after {
                    tracing::debug!(
                        "Cleaned up {} expired messages ({} remaining)",
                        before - after,
                        after
                    );
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_message_expiration() {
        let msg = Message::new(
            "client1".to_string(),
            "test".to_string(),
            Duration::from_millis(100),
        );

        assert!(!msg.is_expired());

        tokio::time::sleep(Duration::from_millis(150)).await;

        assert!(msg.is_expired());
    }

    #[tokio::test]
    async fn test_message_queue() {
        let queue = MessageQueue::new(Duration::from_secs(60), 10);

        queue.push("client1".to_string(), "msg1".to_string()).await;
        queue.push("client2".to_string(), "msg2".to_string()).await;

        let messages = queue.get_all().await;
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "msg1");
        assert_eq!(messages[1].content, "msg2");
    }

    #[tokio::test]
    async fn test_message_queue_max_size() {
        let queue = MessageQueue::new(Duration::from_secs(60), 2);

        queue.push("client1".to_string(), "msg1".to_string()).await;
        queue.push("client2".to_string(), "msg2".to_string()).await;
        queue.push("client3".to_string(), "msg3".to_string()).await;

        let messages = queue.get_all().await;
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "msg2"); // msg1 was dropped
        assert_eq!(messages[1].content, "msg3");
    }
}
