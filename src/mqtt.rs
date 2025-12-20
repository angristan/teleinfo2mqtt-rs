use crate::teleinfo::parser::{SensorMeta, TeleinfoFrame, SENSOR_METADATA};
use aimeqtt::client::{Client, ClientError, PublishOptions};
use serde_json::json;
use tracing::{event, instrument, Level};

/// Publishes Home Assistant MQTT Discovery payloads for all TeleInfo sensors
#[instrument(skip(client))]
pub async fn publish_discovery(
    client: &Client,
    adco: &str,
    discovery_prefix: &str,
) -> Result<(), ClientError> {
    event!(Level::INFO, "Publishing Home Assistant discovery");

    let device = json!({
        "identifiers": [format!("linky_{}", adco)],
        "name": format!("Linky {}", adco),
        "manufacturer": "Enedis",
        "model": "Linky"
    });

    for sensor in SENSOR_METADATA {
        publish_sensor_discovery(client, adco, discovery_prefix, sensor, &device).await?;
    }

    Ok(())
}

async fn publish_sensor_discovery(
    client: &Client,
    adco: &str,
    discovery_prefix: &str,
    sensor: &SensorMeta,
    device: &serde_json::Value,
) -> Result<(), ClientError> {
    let unique_id = format!("linky_{}_{}", adco, sensor.key.to_lowercase());
    let config_topic = format!(
        "{}/sensor/linky_{}/{}/config",
        discovery_prefix,
        adco,
        sensor.key.to_lowercase()
    );

    let mut payload = json!({
        "name": sensor.name,
        "unique_id": unique_id,
        "state_topic": format!("teleinfo/{}", adco),
        "value_template": format!("{{{{ value_json.{}.value }}}}", sensor.key),
        "device": device,
    });

    if let Some(dc) = sensor.device_class {
        payload["device_class"] = json!(dc);
    }
    if let Some(unit) = sensor.unit {
        payload["unit_of_measurement"] = json!(unit);
    }
    if let Some(sc) = sensor.state_class {
        payload["state_class"] = json!(sc);
    }

    event!(Level::DEBUG, topic = %config_topic, "Publishing discovery for {}", sensor.key);

    client
        .publish(
            config_topic,
            payload.to_string(),
            PublishOptions::new().retain(),
        )
        .await
}

#[instrument(skip(client))]
pub async fn publish_teleinfo(client: &Client, value: &TeleinfoFrame) -> Result<(), ClientError> {
    event!(Level::INFO, "Publishing teleinfo frame to MQTT");

    client
        .publish(
            format!("teleinfo/{}", value.adco),
            value.to_string(),
            PublishOptions::new(),
        )
        .await
}
