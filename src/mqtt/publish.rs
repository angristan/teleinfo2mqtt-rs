use crate::teleinfo::parser::TeleinfoFrame;
use aimeqtt::client::Client;
use rppal::gpio::Gpio;
use std::thread;
use std::time::Duration;
use tokio::task;

const GPIO_PITINFO_GREEN_LED: u8 = 4;

pub fn publish_teleinfo(client: &Client, value: TeleinfoFrame) {
    let client_clone = (*client).clone();
    task::spawn(async move {
        let publish_res =
            client_clone.publish(format!("teleinfo/{}", value.adco), value.to_string());

        match publish_res {
            Ok(_) => {
                println!("Published MQTT message");

                let mut pin = Gpio::new()
                    .unwrap()
                    .get(GPIO_PITINFO_GREEN_LED)
                    .unwrap()
                    .into_output();

                pin.set_high();
                thread::sleep(Duration::from_millis(10));
                pin.set_low();
            }
            Err(e) => eprintln!("Error: {:?}", e),
        }
    });
}
