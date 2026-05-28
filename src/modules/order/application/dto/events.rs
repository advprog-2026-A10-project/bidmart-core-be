use crate::modules::order::domain::entities::NotificationEventPayload;

pub struct PublishEventDto {
    pub payload: NotificationEventPayload,
}
