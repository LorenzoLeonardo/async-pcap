# async-pcap

`async-pcap` is a Rust library that provides an asynchronous wrapper around `pcap` packet captures. It allows you to capture network packets in a non-blocking, async context using Tokio. Internally, it spawns a dedicated thread to poll packets and delivers them via an asynchronous channel.

---

## Features

* Async-friendly wrapper around `pcap::Capture<Active>`.
* Returns owned packet data (`Vec<u8>`) with packet metadata.
* Safe to use in multi-threaded Tokio contexts.
* Simple API: `AsyncCapture::new()` and `next_packet().await`.

---

## Usage

```rust
use async_pcap::{AsyncCapture, Capture, Device};
use tokio::runtime::Runtime;

fn main() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Open the network device
        let device = Device::lookup().unwrap().unwrap();
        let cap = Capture::from_device(device)
            .unwrap()
            .promisc(true)
            .snaplen(65535)
            .timeout(500)
            .immediate_mode(true)
            .open()
            .unwrap();

        // Create the async capture and handle
        let (async_cap, handle) = AsyncCapture::new(cap);

        // Clone the handle to stop from another task
        let handle_clone = handle.clone();

        // Spawn a task to stop capture after 5 seconds
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            handle_clone.stop(); // This sends the Stop signal
            println!("Capture stopped from another task");
        });

        // Read packets asynchronously
        while let Some(packet) = async_cap.next_packet().await {
            println!("Captured packet: {} bytes", packet.data.len());
        }

        println!("Capture loop exited cleanly");
    });
}
```

---

## Notes

* `AsyncCapture` internally spawns a dedicated thread for polling packets.
* Uses a Tokio `Mutex` and `UnboundedReceiver` for async packet delivery.
* Suitable for async network monitoring applications.

---

