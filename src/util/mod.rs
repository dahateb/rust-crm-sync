pub enum MessageType {
    SetupTriggerType,
    SyncMessageType,
    TriggerMessageType,
}

pub trait Message: std::marker::Send {
    fn message_type(&self) -> &MessageType;
    fn to_string(&self) -> String;
}

pub struct SyncMessage {
    message_type: MessageType,
    message: String,
}

impl SyncMessage {
    pub fn new(message: &str) -> Self {
        Self {
            message_type: MessageType::SyncMessageType,
            message: message.to_owned(),
        }
    }
}

impl Message for SyncMessage {
    fn message_type(&self) -> &MessageType {
        &self.message_type
    }
    fn to_string(&self) -> String {
        json!({ "message": self.message }).to_string()
    }
}
