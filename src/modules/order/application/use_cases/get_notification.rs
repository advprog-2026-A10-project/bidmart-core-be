use crate::modules::order::application::dto::GetNotificationDto;
use crate::modules::order::domain::traits::NotificationRepository;

pub struct GetNotificationUseCase<T: NotificationRepository> {
    repository: T,
}

impl<T: NotificationRepository> GetNotificationUseCase<T> {
    pub fn new(repository: T) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        dto: GetNotificationDto,
    ) -> Result<
        crate::modules::order::domain::entities::Notification,
        crate::modules::order::domain::errors::NotificationError,
    > {
        self.repository.get_notification(dto.notification_id).await
    }
}
