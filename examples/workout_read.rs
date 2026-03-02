use anyhow::Context;
use tokio_serial::SerialPortBuilderExt;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port_name = "/dev/ttyUSB0";
    let baud_rate = 9600;

    let device = tokio_serial::new(port_name, baud_rate)
        .timeout(Duration::from_secs(1))
        .open_native_async()
        .context("Failed to open port")?;


    todo!()
}
