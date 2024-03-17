use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use rumqttc::{AsyncClient, MqttOptions};
use std::{env, time::Duration};
use tokio::task;

mod mqtt;
mod serial;
mod teleinfo;

#[tokio::main]
async fn main() {
    let mqtt_host = env::var("MQTT_HOST").expect("$MQTT_HOST is not set");
    let mqtt_port = match env::var("MQTT_PORT") {
        Ok(port) => port
            .parse::<u16>()
            .expect("$MQTT_PORT is not a valid port number"),
        Err(_) => 1883,
    };
    let mqtt_user = env::var("MQTT_USER");
    let mqtt_pass = env::var("MQTT_PASS");
    let serial_port = match env::var("SERIAL_PORT") {
        Ok(port) => port,
        Err(_) => "/dev/ttyS0".to_string(),
    };

    let mut mqttoptions = MqttOptions::new("teleinfo2mqtt-rs", mqtt_host, mqtt_port);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    if mqtt_user.is_ok() && mqtt_pass.is_ok() {
        mqttoptions.set_credentials(mqtt_user.unwrap(), mqtt_pass.unwrap());
    }

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // We need to keep polling the event loop to make it progress
    task::spawn(async move { while let Ok(_) = eventloop.poll().await {} });

    let serial_stream = serial::stream::serial_stream(serial_port);
    pin_mut!(serial_stream);

    let teleinfo_raw_frames_stream = teleinfo::stream::ascii_to_frames(serial_stream);
    pin_mut!(teleinfo_raw_frames_stream);

    let teleinfo_parsed_frames_stream =
        teleinfo::stream::frame_to_teleinfo(teleinfo_raw_frames_stream);
    pin_mut!(teleinfo_parsed_frames_stream);

    while let Some(value) = teleinfo_parsed_frames_stream.next().await {
        println!("=====================");
        println!("{:?}", value);

        mqtt::publish::publish_teleinfo(&client, value);
    }
}
