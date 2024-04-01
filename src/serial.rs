use async_stream::stream;
use futures_util::stream::Stream;
use rppal::uart::{Parity, Uart};
use std::time::Duration;
use tracing::{event, instrument, Level};

#[instrument]
pub fn serial_stream(serial_device: String) -> impl Stream<Item = Vec<u8>> {
    let baud_rate = 1200;
    let data_bits = 7;
    let parity = Parity::None;
    let stop_bits = 1;

    let mut uart_device = Uart::with_path(serial_device, baud_rate, parity, data_bits, stop_bits)
        .expect("Failed to open UART");
    uart_device
        .set_read_mode(1, Duration::default())
        .expect("Failed to set read mode");

    event!(Level::INFO, ?uart_device, "Opened UART device");

    let mut buffer = [0u8; 1];
    stream! {
        loop {
            match uart_device.read(&mut buffer) {
                Ok(bytes_read) => {
                    if bytes_read > 0 {
                        yield buffer.to_vec();
                    }
                }
                Err(e) => {
                    event!(Level::ERROR, "Error reading from UART: {}", e);
                    break;
                }
            }
        }
    }
}
