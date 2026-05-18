use crate::modules::order::application::dto::PublishEventDto;
use crate::modules::order::domain::traits::NotificationRepository;

pub struct PublishEventUseCase<T: NotificationRepository> {
    repository: T,
}

impl<T: NotificationRepository> PublishEventUseCase<T> {
    pub fn new(repository: T) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        dto: PublishEventDto,
    ) -> Result<(), crate::modules::order::domain::errors::NotificationError> {
        self.repository.publish_event(dto.payload).await
    }
}
