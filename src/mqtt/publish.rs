use rumqttc::{AsyncClient, QoS};
use tokio::task;

use crate::teleinfo::parser::TeleinfoFrame;

pub fn publish_teleinfo(client: &AsyncClient, value: TeleinfoFrame) {
    let client_clone = client.clone();
    task::spawn(async move {
        let publish_res = client_clone
            .publish(
                format!("teleinfo/{}", value.adco),
                QoS::AtMostOnce,
                false,
                value.to_string(),
            )
            .await;

        match publish_res {
            Ok(_) => println!("Published"),
            Err(e) => println!("Error: {:?}", e),
        }
    });
}
