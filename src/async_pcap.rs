use pcap::{Active, Capture, PacketHeader};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

/// Represents a network packet with its header and raw data.
#[derive(Debug, Clone)]
pub struct Packet {
    /// Packet header information provided by pcap
    pub header: PacketHeader,
    /// Raw packet data
    pub data: Vec<u8>,
}

/// An asynchronous wrapper around a `pcap::Capture`.
///  
/// `AsyncCapture` owns the receiver side of a channel that receives
/// captured packets or a stop signal. It allows async code to
/// `await` new packets without blocking a thread.
pub struct AsyncCapture {
    rx: Mutex<UnboundedReceiver<PacketOrStop>>,
}

/// Enum used internally to represent either a captured packet
/// or a stop signal to terminate the capture.
enum PacketOrStop {
    /// A captured packet
    Packet(Packet),
    /// Signal that capture has stopped
    Stop,
}

/// Handle to control the asynchronous capture.
///  
/// `AsyncCaptureHandle` allows stopping the capture from another
/// thread or async task.
#[derive(Clone)]
pub struct AsyncCaptureHandle {
    tx: UnboundedSender<PacketOrStop>,
}

impl AsyncCapture {
    /// Creates a new asynchronous capture from a `pcap::Capture<Active>`.
    ///
    /// Spawns a background thread that reads packets and sends them
    /// through a channel for async consumption.
    ///
    /// Returns a tuple of `(AsyncCapture, AsyncCaptureHandle)`.
    pub fn new(mut cap: Capture<Active>) -> (Self, AsyncCaptureHandle) {
        let (tx, rx) = unbounded_channel::<PacketOrStop>();
        let handle = AsyncCaptureHandle { tx: tx.clone() };

        std::thread::spawn(move || {
            while let Ok(packet) = cap.next_packet() {
                let owned = Packet {
                    header: *packet.header,
                    data: packet.data.to_vec(),
                };
                if tx.send(PacketOrStop::Packet(owned)).is_err() {
                    // Receiver dropped, exit thread
                    break;
                }
            }
            // Send a Stop message when capture thread ends
            let _ = tx.send(PacketOrStop::Stop);
        });

        (Self { rx: Mutex::new(rx) }, handle)
    }

    /// Waits for the next packet asynchronously.
    ///
    /// Returns `Some(Packet)` if a packet is received,
    /// or `None` if the capture has stopped.
    pub async fn next_packet(&self) -> Option<Packet> {
        let mut rx = self.rx.lock().await;
        match rx.recv().await {
            Some(PacketOrStop::Packet(pkt)) => Some(pkt),
            Some(PacketOrStop::Stop) | None => None,
        }
    }
}

impl AsyncCaptureHandle {
    /// Stops the capture from another thread or async task.
    ///
    /// Sends a stop signal to the capture thread, causing
    /// `AsyncCapture::next_packet()` to return `None`.
    pub fn stop(&self) {
        let _ = self.tx.send(PacketOrStop::Stop);
    }
}
