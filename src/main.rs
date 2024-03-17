use futures_util::pin_mut;
use futures_util::stream::StreamExt;

mod serial;
mod teleinfo;

#[tokio::main]
async fn main() {
    let serial_stream = serial::stream::serial_stream();
    pin_mut!(serial_stream);

    let teleinfo_raw_frames_stream = teleinfo::stream::ascii_to_frames(serial_stream);
    pin_mut!(teleinfo_raw_frames_stream);

    let teleinfo_parsed_frames_stream =
        teleinfo::stream::frame_to_teleinfo(teleinfo_raw_frames_stream);
    pin_mut!(teleinfo_parsed_frames_stream);

    while let Some(value) = teleinfo_parsed_frames_stream.next().await {
        println!("=====================");
        println!("{:?}", value);
    }
}
