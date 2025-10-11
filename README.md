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
use async_capture::{AsyncCapture, Capture, Active};
use tokio::runtime::Runtime;

fn main() {
    // Create a Tokio runtime
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Open the default network device
        let cap = Capture::from_device("eth0")
            .unwrap()
            .open()
            .unwrap();

        // Wrap it in an async capture
        let async_cap = AsyncCapture::new(cap);

        // Await packets
        while let Some(packet) = async_cap.next_packet().await {
            println!("Captured packet with {} bytes", packet.data.len());
        }
    });
}
```

---

## Notes

* `AsyncCapture` internally spawns a dedicated thread for polling packets.
* Uses a Tokio `Mutex` and `UnboundedReceiver` for async packet delivery.
* Suitable for async network monitoring applications.

---

