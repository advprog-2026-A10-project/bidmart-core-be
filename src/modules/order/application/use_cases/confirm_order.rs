use crate::modules::order::application::dto::ConfirmOrderDto;
use crate::modules::order::domain::traits::OrderRepository;

pub struct ConfirmOrderUseCase<T: OrderRepository> {
    repository: T,
}

impl<T: OrderRepository> ConfirmOrderUseCase<T> {
    pub fn new(repository: T) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        dto: ConfirmOrderDto,
    ) -> Result<(), crate::modules::order::domain::errors::OrderError> {
        self.repository
            .confirm_order(dto.order_id, &dto.actor_id)
            .await
    }
}
