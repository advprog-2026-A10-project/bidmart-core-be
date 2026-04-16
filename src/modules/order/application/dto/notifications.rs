use uuid::Uuid;

pub struct ListNotificationsDto {
    pub user_id: Option<String>,
    pub limit: Option<u32>,
    pub unread_only: bool,
}

pub struct GetNotificationDto {
    pub notification_id: Uuid,
}

pub struct MarkNotificationDto {
    pub notification_id: Uuid,
    pub actor_id: String,
}
