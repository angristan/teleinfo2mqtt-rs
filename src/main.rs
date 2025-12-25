use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use rppal::gpio::Gpio;
use std::env;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{event, Level};

mod mqtt;
mod serial;
mod teleinfo;

const GPIO_PITINFO_GREEN_LED: u8 = 4;
const DEFAULT_MAX_POWER_VA: u32 = 6000;

#[derive(Debug, Clone, Copy, PartialEq)]
enum LedMode {
    Frame, // Blink once per frame sent
    Power, // Blink rate based on power consumption
}

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
    let discovery_prefix =
        env::var("HA_DISCOVERY_PREFIX").unwrap_or_else(|_| "homeassistant".to_string());
    let led_mode = match env::var("LED_MODE") {
        Ok(mode) => match mode.to_lowercase().as_str() {
            "power" => LedMode::Power,
            _ => LedMode::Frame,
        },
        Err(_) => LedMode::Frame,
    };
    let max_power_va: u32 = env::var("LED_MAX_POWER_VA")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_POWER_VA);

    event!(Level::INFO, ?led_mode, max_power_va, "LED configuration");

    let aimeqtt_options = aimeqtt::client::ClientOptions::new()
        .with_broker_host(mqtt_host)
        .with_broker_port(mqtt_port)
        .with_keep_alive(60);
    let aimeqtt_options = if mqtt_user.is_ok() && mqtt_pass.is_ok() {
        aimeqtt_options.with_credentials(mqtt_user.unwrap(), mqtt_pass.unwrap())
    } else {
        aimeqtt_options
    };

    let client = aimeqtt::client::new(aimeqtt_options).await;
    event!(Level::DEBUG, "MQTT client created");

    let serial_stream = serial::serial_stream(serial_device);
    pin_mut!(serial_stream);

    let teleinfo_raw_frames_stream = teleinfo::stream::ascii_to_frames(serial_stream);
    pin_mut!(teleinfo_raw_frames_stream);

    let teleinfo_parsed_frames_stream =
        teleinfo::stream::frame_to_teleinfo(teleinfo_raw_frames_stream);
    pin_mut!(teleinfo_parsed_frames_stream);

    // Shared power value for LED blinking task (only used in Power mode)
    let current_power = if led_mode == LedMode::Power {
        Some(Arc::new(AtomicU32::new(0)))
    } else {
        None
    };

    // LED pin for Frame mode (owned by main loop)
    let mut led_pin_frame = if led_mode == LedMode::Frame {
        Some(
            Gpio::new()
                .expect("Failed to initialize GPIO")
                .get(GPIO_PITINFO_GREEN_LED)
                .expect("Failed to get GPIO pin")
                .into_output(),
        )
    } else {
        None
    };

    // Spawn LED blinking task for Power mode
    if let Some(ref power_arc) = current_power {
        let led_power = Arc::clone(power_arc);
        tokio::spawn(async move {
            let mut led_pin = Gpio::new()
                .expect("Failed to initialize GPIO")
                .get(GPIO_PITINFO_GREEN_LED)
                .expect("Failed to get GPIO pin")
                .into_output();

            loop {
                let power = led_power.load(Ordering::Relaxed);
                let ratio = (power as f32 / max_power_va as f32).min(1.0);

                // Higher power = shorter duration (200ms -> 10ms)
                let on_duration_ms = (200.0 - ratio * 190.0) as u64;
                // Higher power = shorter interval (2000ms -> 100ms)
                let off_duration_ms = (1800.0 - ratio * 1710.0) as u64;

                event!(Level::DEBUG, power_va = power, ratio = %format!("{:.2}", ratio), on_ms = on_duration_ms, off_ms = off_duration_ms, "LED blink");

                led_pin.set_high();
                tokio::time::sleep(Duration::from_millis(on_duration_ms)).await;
                led_pin.set_low();
                tokio::time::sleep(Duration::from_millis(off_duration_ms)).await;
            }
        });
    }

    let mut discovery_sent = false;

    while let Some(value) = teleinfo_parsed_frames_stream.next().await {
        // Publish Home Assistant discovery on first frame
        if !discovery_sent {
            match mqtt::publish_discovery(&client, &value.adco, &discovery_prefix).await {
                Ok(_) => {
                    event!(Level::INFO, "Published Home Assistant MQTT discovery");
                    discovery_sent = true;
                }
                Err(e) => {
                    event!(Level::ERROR, error = ?e, "Failed to publish discovery");
                }
            }
        }

        // Update current power for LED blinking rate (Power mode)
        if let Some(ref power_arc) = current_power {
            if let Ok(papp) = value.papp.parse::<u32>() {
                power_arc.store(papp, Ordering::Relaxed);
            }
        }

        match mqtt::publish_teleinfo(&client, &value).await {
            Ok(_) => {
                // Blink LED on successful publish (Frame mode)
                if let Some(ref mut led_pin) = led_pin_frame {
                    led_pin.set_high();
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    led_pin.set_low();
                }
            }
            Err(e) => {
                event!(Level::ERROR, error = ?e, "Error while publishing teleinfo frame to MQTT");
            }
        }
    }
}
