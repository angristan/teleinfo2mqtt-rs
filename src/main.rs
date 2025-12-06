use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use rppal::gpio::Gpio;
use std::env;
use std::time::Duration;
use tracing::{event, Level};

mod mqtt;
mod serial;
mod teleinfo;

const GPIO_PITINFO_GREEN_LED: u8 = 4;

#[tokio::main]
async fn main() {
    let log_level: tracing::Level = match env::var("LOG_LEVEL") {
        Ok(level) => match level.to_lowercase().as_str() {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        },
        Err(_) => tracing::Level::INFO,
    };

    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(true)
        .with_max_level(log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mqtt_host = env::var("MQTT_HOST").expect("$MQTT_HOST is not set");
    let mqtt_port = match env::var("MQTT_PORT") {
        Ok(port) => port
            .parse::<u16>()
            .expect("$MQTT_PORT is not a valid port number"),
        Err(_) => 1883,
    };
    let mqtt_user = env::var("MQTT_USER");
    let mqtt_pass = env::var("MQTT_PASS");
    let serial_device = match env::var("SERIAL_PORT") {
        Ok(port) => port,
        Err(_) => "/dev/ttyS0".to_string(),
    };

    let mut aimeqtt_options =
        aimeqtt::client::ClientOptions::new(mqtt_host, mqtt_port).with_keep_alive(60);
    if mqtt_user.is_ok() && mqtt_pass.is_ok() {
        aimeqtt_options = aimeqtt_options.with_credentials(mqtt_user.unwrap(), mqtt_pass.unwrap());
    }

    let client = aimeqtt::client::new(aimeqtt_options).await;
    event!(Level::DEBUG, "MQTT client created");

    let serial_stream = serial::serial_stream(serial_device);
    pin_mut!(serial_stream);

    let teleinfo_raw_frames_stream = teleinfo::stream::ascii_to_frames(serial_stream);
    pin_mut!(teleinfo_raw_frames_stream);

    let teleinfo_parsed_frames_stream =
        teleinfo::stream::frame_to_teleinfo(teleinfo_raw_frames_stream);
    pin_mut!(teleinfo_parsed_frames_stream);

    let mut led_pin = Gpio::new()
        .expect("Failed to initialize GPIO")
        .get(GPIO_PITINFO_GREEN_LED)
        .expect("Failed to get GPIO pin")
        .into_output();

    while let Some(value) = teleinfo_parsed_frames_stream.next().await {
        match mqtt::publish_teleinfo(&client, &value).await {
            Ok(_) => {
                led_pin.set_high();
                tokio::time::sleep(Duration::from_millis(10)).await;
                led_pin.set_low();
            }
            Err(e) => {
                event!(Level::ERROR, error = ?e, "Error while publishing teleinfo frame to MQTT");
            }
        }
    }
}
