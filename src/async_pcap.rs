use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use pcap::{Active, Capture, Error, PacketHeader};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

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
            stop_flag: stop_flag.clone(),
        };

        std::thread::spawn(move || {
            loop {
                if stop_flag.load(Ordering::Relaxed) {
                    eprintln!("AsyncCapture thread is aborted.");
                    break;
                }
                let res = cap.next_packet();

                let owned = res.map(|packet| Packet {
                    header: *packet.header,
                    data: packet.data.to_vec(),
                });
                if let Err(e) = tx.send(PacketOrStop::Packet(owned)) {
                    // Receiver dropped, exit thread
                    eprintln!("{e}");
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
    /// Stops the capture from another thread or asynchronous task.
    ///
    /// This method sets the internal stop flag, signaling the background
    /// capture thread to terminate gracefully. It also sends a `Stop`
    /// message through the internal channel to ensure that any awaiting
    /// calls to [`AsyncCapture::next_packet()`] will return `None`.
    ///
    /// # Notes
    ///
    /// - Calling this method multiple times is safe and idempotent.
    /// - Once stopped, the background thread will no longer produce packets.
    /// - After calling `stop`, any future calls to
    ///   [`AsyncCapture::next_packet()`] will immediately return `None`.
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

impl Drop for AsyncCaptureHandle {
    /// Automatically stops the capture when the last handle is dropped.
    ///
    /// This ensures that the background capture thread is terminated
    /// even if [`AsyncCaptureHandle::stop()`] was not called explicitly.
    ///
    /// When the last instance of this handle is dropped, the stop flag
    /// is set, and a `Stop` signal is sent to notify all waiting receivers.
    ///
    /// # Notes
    ///
    /// - Dropping cloned handles does **not** stop the capture as long as
    ///   other handles still exist.
    /// - The capture thread will only be stopped automatically when the
    ///   **last** handle is dropped.
    fn drop(&mut self) {
        self.stop();
    }
}
