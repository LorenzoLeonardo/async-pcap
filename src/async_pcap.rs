use pcap::Active;
use pcap::Capture;
use pcap::PacketHeader;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

/// Represents an owned packet captured from the network.
///
/// This struct contains the packet header and the packet data as a `Vec<u8>`,
/// allowing it to be safely stored and sent across threads.
#[derive(Debug, Clone)]
pub struct Packet {
    pub header: PacketHeader,
    pub data: Vec<u8>,
}

/// An asynchronous wrapper around a `pcap::Capture<Active>`.
///
/// This allows capturing packets in a non-blocking, async context using Tokio.
/// Internally, it spawns a dedicated thread to poll packets and sends them
/// through a channel that can be awaited asynchronously.
pub struct AsyncCapture {
    rx: Mutex<UnboundedReceiver<Packet>>,
}

impl AsyncCapture {
    /// Creates a new asynchronous capture from an active `pcap` capture.
    ///
    /// # Arguments
    ///
    /// * `cap` - A `Capture<Active>` object representing the pcap device.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use async_pcap::{AsyncCapture, Capture, Active};
    ///
    /// let mut cap = Capture::from_device("eth0").unwrap().open().unwrap();
    /// let async_cap = AsyncCapture::new(cap);
    /// ```
    pub fn new(mut cap: Capture<Active>) -> Self {
        let (tx, rx) = unbounded_channel::<Packet>();

        // Spawn a dedicated thread to poll packets
        std::thread::spawn(move || {
            while let Ok(packet) = cap.next_packet() {
                // Copy the data into owned Vec<u8>
                let owned = Packet {
                    header: packet.header.clone(),
                    data: packet.data.to_vec(),
                };
                // Ignore send errors if receiver is dropped
                let _ = tx.send(owned);
            }
        });

        Self { rx: Mutex::new(rx) }
    }

    /// Asynchronously retrieves the next captured packet.
    ///
    /// Returns `None` if the sender has been dropped (e.g., the capture thread exited).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tokio::runtime::Runtime;
    /// use async_pcap::AsyncCapture;
    ///
    /// let rt = Runtime::new().unwrap();
    /// rt.block_on(async {
    ///     if let Some(packet) = async_cap.next_packet().await {
    ///         println!("Got packet with {} bytes", packet.data.len());
    ///     }
    /// });
    /// ```
    pub async fn next_packet(&self) -> Option<Packet> {
        let mut rx = self.rx.lock().await;
        rx.recv().await
    }
}
