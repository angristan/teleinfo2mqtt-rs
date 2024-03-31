use crate::teleinfo::parser::TeleinfoFrame;
use aimeqtt::client::{Client, ClientError};
use tracing::{event, instrument, Level};

#[instrument(skip(client))]
pub async fn publish_teleinfo(client: &Client, value: &TeleinfoFrame) -> Result<(), ClientError> {
    event!(Level::INFO, "Publishing teleinfo frame to MQTT");

    client
        .publish(format!("teleinfo/{}", value.adco), value.to_string())
        .await
}
