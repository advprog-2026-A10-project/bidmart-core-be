use crate::modules::order::application::dto::CreateDisputeDto;
use crate::modules::order::domain::traits::OrderRepository;

pub struct CreateDisputeUseCase<T: OrderRepository> {
    repository: T,
}

impl<T: OrderRepository> CreateDisputeUseCase<T> {
    pub fn new(repository: T) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        dto: CreateDisputeDto,
    ) -> Result<(), crate::modules::order::domain::errors::OrderError> {
        self.repository
            .create_dispute(dto.order_id, &dto.reporter_id, &dto.reason)
            .await
    }
}
