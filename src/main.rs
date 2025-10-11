use async_pcap::AsyncCapture;
use pcap::Capture;
use pcap::Device;

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

    let res = cap.next_packet().await;
    println!("{res:?}");
}
