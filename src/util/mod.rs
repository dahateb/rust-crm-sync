use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub enum MessageType {
    SetupTriggerType,
    SyncMessageType,
    TriggerMessageType,
}

pub trait Message: std::marker::Send + Sync {
    fn message_type(&self) -> &MessageType;
    fn to_string(&self) -> String;
    fn as_value(&self) -> serde_json::Value;
}

#[derive(Serialize, Deserialize)]
pub struct SyncMessage {
    message_type: MessageType,
    message: String,
    obj_type: String,
    obj_count: usize,
}

impl SyncMessage {
    pub fn new(message: &str, obj_type: &str, obj_count: usize) -> Self {
        Self {
            message_type: MessageType::SyncMessageType,
            message: message.to_owned(),
            obj_type: obj_type.to_owned(),
            obj_count: obj_count,
        }
    }
}

impl Message for SyncMessage {
    fn message_type(&self) -> &MessageType {
        &self.message_type
    }
    fn to_string(&self) -> String {
        self.as_value().to_string()
    }

    fn as_value(&self) -> serde_json::Value {
        json!({
            "message": self.message,
            "obj_type": self.obj_type,
            "obj_count": self.obj_count
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct TriggerMessage {
    message_type: MessageType,
    message: String,
    obj_count: u64,
    elapsed: Duration,
}

impl TriggerMessage {
    pub fn new(message: &str, obj_count: u64, elapsed: Duration) -> Self {
        Self {
            message_type: MessageType::TriggerMessageType,
            message: message.to_owned(),
            obj_count: obj_count,
            elapsed: elapsed,
        }
    }
}

impl Message for TriggerMessage {
    fn message_type(&self) -> &MessageType {
        &self.message_type
    }
    fn to_string(&self) -> String {
        self.as_value().to_string()
    }
    fn as_value(&self) -> serde_json::Value {
        json!({
            "message": self.message,
            "obj_count": self.obj_count,
            "elapsed": self.elapsed.as_secs()
        })
    }
}
