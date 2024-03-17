use super::parser;
use super::parser::TeleinfoFrame;
use async_stream::stream;
use futures_util::stream::Stream;
use futures_util::stream::StreamExt;

pub fn ascii_to_frames<S: Stream<Item = Vec<u8>>>(ascii_stream: S) -> impl Stream<Item = String> {
    let mut ascii_stream = Box::pin(ascii_stream);
    stream! {
        let mut teleinfo_buffer: Vec<String> = Vec::new();
        while let Some(value) = ascii_stream.next().await {

            let teleinfo = std::str::from_utf8(&value);
            match teleinfo {
                Ok(teleinfo) => {
                        if teleinfo_buffer.len() >= 4 {
                            let mut i = 0;
                            for c in teleinfo_buffer[teleinfo_buffer.len()-4..].join("").chars() {
                                if c == 'A' {
                                    if teleinfo_buffer[teleinfo_buffer.len()-4..].join("").chars().skip(i).take(4).collect::<String>() == "ADCO" {
                                        yield teleinfo_buffer[..teleinfo_buffer.len()-4].join("");
                                        teleinfo_buffer = teleinfo_buffer[teleinfo_buffer.len()-4..].to_vec();
                                    }
                                }
                                i += 1;
                            }
                        }


                    teleinfo_buffer.push(teleinfo.to_string());
                }
                // Err(e) => eprint!("{:?}", e),
                Err(_) => (),
            }
        }
    }
}

pub fn frame_to_teleinfo<S: Stream<Item = String>>(
    frame_stream: S,
) -> impl Stream<Item = TeleinfoFrame> {
    let mut frame_stream = Box::pin(frame_stream);
    stream! {
        while let Some(value) = frame_stream.next().await {
            let teleinfo = parser::parse_teleinfo(&value);
            match teleinfo {
                Ok(teleinfo) => {
                    yield teleinfo;
                }
                Err(e) => eprintln!("{:?}", e),
            }
        }
    }
}
