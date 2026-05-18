use crate::modules::order::application::dto::MarkNotificationDto;
use crate::modules::order::domain::traits::NotificationRepository;

pub struct MarkNotificationUseCase<T: NotificationRepository> {
    repository: T,
}

impl<T: NotificationRepository> MarkNotificationUseCase<T> {
    pub fn new(repository: T) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        dto: MarkNotificationDto,
    ) -> Result<(), crate::modules::order::domain::errors::NotificationError> {
        self.repository
            .mark_as_read(dto.notification_id, &dto.actor_id)
            .await
    }
}
