use async_stream::stream;
use futures_util::stream::Stream;
use std::{io, time::Duration};
use tracing::{event, instrument, Level};

#[instrument]
pub fn serial_stream(port_device: String) -> impl Stream<Item = Vec<u8>> {
    stream! {
        let port = serialport::new(port_device, 1200)
            .timeout(Duration::from_millis(1000)).data_bits(serialport::DataBits::Seven)
            .open();

        event!(Level::INFO, "Opening serial port");

        match port {
            Ok(mut port) => {
                let mut serial_buf: Vec<u8> = vec![0; 1];
                loop {
                    match port.read(serial_buf.as_mut_slice()) {
                        Ok(t) => {
                            yield serial_buf[..t].to_vec()
                        },
                        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                        Err(e) => {
                            event!(Level::ERROR, "Failed to read from serial port. Error: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                event!(Level::ERROR, "Failed to open port. Error: {:?}", e);
                ::std::process::exit(1);
            }
        }
    }
}
