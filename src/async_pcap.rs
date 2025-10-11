use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use pcap::{Active, Capture, Error, PacketHeader};
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
    Packet(Result<Packet, Error>),
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
    stop_flag: Arc<AtomicBool>,
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
        let stop_flag = Arc::new(AtomicBool::new(false));
        let handle = AsyncCaptureHandle {
            tx: tx.clone(),
            stop_flag,
        };

        std::thread::spawn(move || {
            loop {
                let res = cap.next_packet();

                let owned = res.map(|packet| Packet {
                    header: *packet.header,
                    data: packet.data.to_vec(),
                });
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
    /// Returns `Some(Result<Packet, Error>)` if a packet is received,
    /// or `None` if the capture has stopped.
    pub async fn next_packet(&self) -> Option<Result<Packet, Error>> {
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
        self.stop_flag.store(true, Ordering::Relaxed);
        let _ = self.tx.send(PacketOrStop::Stop);
    }
}
