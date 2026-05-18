use crate::modules::order::application::dto::UpdateShippingDto;
use crate::modules::order::domain::traits::OrderRepository;

pub struct UpdateShippingUseCase<T: OrderRepository> {
    repository: T,
}

impl<T: OrderRepository> UpdateShippingUseCase<T> {
    pub fn new(repository: T) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        dto: UpdateShippingDto,
    ) -> Result<(), crate::modules::order::domain::errors::OrderError> {
        self.repository
            .update_shipping_status(dto.order_id, &dto.status, dto.tracking.as_deref())
            .await
    }
}
