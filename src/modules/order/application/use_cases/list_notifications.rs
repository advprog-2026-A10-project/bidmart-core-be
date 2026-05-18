use crate::modules::order::application::dto::ListNotificationsDto;
use crate::modules::order::domain::traits::NotificationRepository;

pub struct ListNotificationsUseCase<T: NotificationRepository> {
    repository: T,
}

impl<T: NotificationRepository> ListNotificationsUseCase<T> {
    pub fn new(repository: T) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        dto: ListNotificationsDto,
    ) -> Result<
        Vec<crate::modules::order::domain::entities::Notification>,
        crate::modules::order::domain::errors::NotificationError,
    > {
        self.repository
            .list_notifications(dto.user_id.as_deref(), dto.limit, dto.unread_only)
            .await
    }
}
