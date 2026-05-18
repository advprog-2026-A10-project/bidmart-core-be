use crate::modules::order::application::dto::ListOrdersDto;
use crate::modules::order::domain::entities::Order;
use crate::modules::order::domain::traits::OrderRepository;

pub struct ListOrdersUseCase<T: OrderRepository> {
    repository: T,
}

impl<T: OrderRepository> ListOrdersUseCase<T> {
    pub fn new(repository: T) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        dto: ListOrdersDto,
    ) -> Result<Vec<Order>, crate::modules::order::domain::errors::OrderError> {
        self.repository
            .list_orders(&dto.role, dto.user_id.as_deref(), dto.stage)
            .await
    }
}
