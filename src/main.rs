use async_pcap::{AsyncCapture, Capture, Device};

#[tokio::main]
async fn main() {
    let device = Device::lookup().unwrap().unwrap();
    let cap = Capture::from_device(device)
        .unwrap()
        .promisc(true)
        .snaplen(65535)
        .timeout(500)
        .immediate_mode(true)
        .open()
        .unwrap();
    let cap = AsyncCapture::new(cap);

    while let Some(packet) = cap.next_packet().await {
        println!("Captured packet with {} bytes", packet.data.len());
    }
}
