use crate::modules::order::application::dto::GetOrderDto;
use crate::modules::order::domain::traits::OrderRepository;

pub struct GetOrderUseCase<T: OrderRepository> {
    repository: T,
}

impl<T: OrderRepository> GetOrderUseCase<T> {
    pub fn new(repository: T) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        dto: GetOrderDto,
    ) -> Result<
        crate::modules::order::domain::entities::Order,
        crate::modules::order::domain::errors::OrderError,
    > {
        self.repository.get_order(dto.order_id).await
    }
}
