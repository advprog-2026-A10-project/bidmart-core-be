use lapin::{
    options::{BasicPublishOptions, ExchangeDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind,
};

const EXCHANGE: &str = "bidmart.auction";

/// Fire-and-forget AMQP publisher ke exchange `bidmart.auction`.
///
/// `Channel` dari lapin adalah `Clone` (Arc-backed), sehingga `AmqpPublisher`
/// bisa di-clone dan di-share across threads tanpa overhead.
///
/// Jika `APP_AMQP_URL` tidak di-set atau koneksi gagal saat startup,
/// seluruh modul berjalan normal tanpa notifikasi realtime (degraded mode).
#[derive(Clone)]
pub struct AmqpPublisher {
    channel: Channel,
}

impl AmqpPublisher {
    pub async fn connect(amqp_url: &str) -> Result<Self, lapin::Error> {
        let conn = Connection::connect(amqp_url, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await?;

        channel
            .exchange_declare(
                EXCHANGE,
                ExchangeKind::Topic,
                ExchangeDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        tracing::info!(exchange = EXCHANGE, "AMQP publisher ready");
        Ok(Self { channel })
    }

    /// Publish payload JSON ke exchange dengan routing key yang diberikan.
    /// Operasi ini non-blocking: di-spawn sebagai tokio task terpisah.
    /// Kegagalan publish hanya di-log, tidak mempengaruhi caller.
    pub fn publish(&self, routing_key: impl Into<String>, payload: serde_json::Value) {
        let channel = self.channel.clone();
        let routing_key = routing_key.into();

        tokio::spawn(async move {
            let body = match serde_json::to_vec(&payload) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(error = %e, routing_key, "AMQP serialize failed");
                    return;
                }
            };

            if let Err(e) = channel
                .basic_publish(
                    EXCHANGE,
                    &routing_key,
                    BasicPublishOptions::default(),
                    &body,
                    BasicProperties::default()
                        .with_content_type("application/json".into())
                        .with_delivery_mode(2), // persistent
                )
                .await
            {
                tracing::warn!(error = %e, routing_key, "AMQP publish failed");
            }
        });
    }
}
