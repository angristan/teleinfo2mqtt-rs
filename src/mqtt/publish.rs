use aimeqtt::client::Client;
use tokio::task;

use crate::teleinfo::parser::TeleinfoFrame;

pub fn publish_teleinfo(client: &Client, value: TeleinfoFrame) {
    let client_clone = (*client).clone();
    task::spawn(async move {
        let publish_res =
            client_clone.publish(format!("teleinfo/{}", value.adco), value.to_string());

        match publish_res {
            Ok(_) => println!("Published"),
            Err(e) => println!("Error: {:?}", e),
        }
    });
}
