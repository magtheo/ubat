use std::net::{TcpListener, TcpStream, SocketAddr};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use serde::{Serialize, Deserialize};
use bincode;

// Enum to represent different network events
#[derive(Debug)]
pub enum NetworkEvent {
    Connected(PeerId),
    Disconnected(PeerId),
    DataReceived {
        peer_id: PeerId,
        payload: Vec<u8>,
    },
    ConnectionError(ConnectionError),
}

// Unique identifier for network peers
type PeerId = String;

// Possible network modes
#[derive(Debug, Clone)]
pub enum NetworkMode {
    Standalone,
    Host,
    Client,
}

// Connection configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub mode: NetworkMode,
    pub port: u16,
    pub max_connections: usize,
    pub server_address: Option<String>,
}

// Custom error type for network operations
#[derive(Debug)]
pub enum ConnectionError {
    ConnectionFailed,
    SendError,
    ReceiveError,
    InvalidMessage,
}

// Network message wrapper for type-safe serialization
#[derive(Serialize, Deserialize)]
struct NetworkMessage<T> {
    message_type: String,
    payload: T,
}

// Primary Network Handler Structure
pub struct NetworkHandler {
    // Current network mode
    mode: NetworkMode,

    // Connection details
    config: NetworkConfig,

    // Active peer connections
    peers: Arc<Mutex<HashMap<PeerId, TcpStream>>>,

    // Event channel for network events
    event_sender: mpsc::Sender<NetworkEvent>,
    event_receiver: mpsc::Receiver<NetworkEvent>,

    // Listener for incoming connections (for host mode)
    listener: Option<TcpListener>,
}

impl NetworkHandler {
    // Create a new network handler
    pub fn new(config: NetworkConfig) -> Result<Self, ConnectionError> {
        let (event_sender, event_receiver) = mpsc::channel();

        let mut handler = Self {
            mode: config.mode.clone(),
            config,
            peers: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
            event_receiver,
            listener: None,
        };

        // Initialize based on network mode
        handler.initialize_mode()?;

        Ok(handler)
    }

    // Initialize networking based on mode
    fn initialize_mode(&mut self) -> Result<(), ConnectionError> {
        match self.mode {
            NetworkMode::Host => self.start_host_mode(),
            NetworkMode::Client => self.start_client_mode(),
            NetworkMode::Standalone => Ok(()),
        }
    }

    // Start host mode - listen for incoming connections
    fn start_host_mode(&mut self) -> Result<(), ConnectionError> {
        let address = format!("0.0.0.0:{}", self.config.port);
        let listener = TcpListener::bind(&address)
            .map_err(|_| ConnectionError::ConnectionFailed)?;
        
        let peers = Arc::clone(&self.peers);
        let event_sender = self.event_sender.clone();

        // Spawn connection acceptance thread
        thread::spawn(move || {
            for incoming in listener.incoming() {
                match incoming {
                    Ok(stream) => {
                        let peer_id = Self::generate_peer_id();
                        
                        // Add to peers
                        let mut peers_lock = peers.lock().unwrap();
                        peers_lock.insert(peer_id.clone(), stream.try_clone().unwrap());

                        // Send connection event
                        event_sender.send(NetworkEvent::Connected(peer_id)).unwrap();
                    }
                    Err(e) => {
                        // Handle connection errors
                        eprintln!("Connection error: {}", e);
                    }
                }
            }
        });

        self.listener = Some(listener);
        Ok(())
    }

    // Start client mode - connect to host
    fn start_client_mode(&mut self) -> Result<(), ConnectionError> {
        let server_address = self.config.server_address
            .as_ref()
            .ok_or(ConnectionError::ConnectionFailed)?;

        let stream = TcpStream::connect(server_address)
            .map_err(|_| ConnectionError::ConnectionFailed)?;

        let peer_id = Self::generate_peer_id();
        
        // Add server connection to peers
        let mut peers = self.peers.lock().unwrap();
        peers.insert(peer_id.clone(), stream);

        // Send connection event
        self.event_sender
            .send(NetworkEvent::Connected(peer_id))
            .map_err(|_| ConnectionError::ConnectionFailed)?;

        Ok(())
    }

    // Send a message to a specific peer
    pub fn send_to_peer<T: Serialize>(
        &self, 
        peer_id: &PeerId, 
        message_type: &str, 
        payload: &T
    ) -> Result<(), ConnectionError> {
        let message = NetworkMessage {
            message_type: message_type.to_string(),
            payload,
        };

        let serialized = bincode::serialize(&message)
            .map_err(|_| ConnectionError::SendError)?;

        let mut peers = self.peers.lock().unwrap();
        if let Some(stream) = peers.get_mut(peer_id) {
            stream.write_all(&serialized)
                .map_err(|_| ConnectionError::SendError)?;
        }

        Ok(())
    }

    // Generate a unique peer identifier
    fn generate_peer_id() -> PeerId {
        // In a real implementation, use a more robust method
        uuid::Uuid::new_v4().to_string()
    }

    // Process incoming network events
    pub fn poll_events(&self) -> Option<NetworkEvent> {
        self.event_receiver.try_recv().ok()
    }
}

// // Demonstration of usage
// fn demonstrate_network_handler() {
//     // Host configuration
//     let host_config = NetworkConfig {
//         mode: NetworkMode::Host,
//         port: 7878,
//         max_connections: 64,
//         server_address: None,
//     };

//     // Client configuration
//     let client_config = NetworkConfig {
//         mode: NetworkMode::Client,
//         port: 0,
//         max_connections: 1,
//         server_address: Some("127.0.0.1:7878".to_string()),
//     };

//     // Create network handlers
//     let host_handler = NetworkHandler::new(host_config).unwrap();
//     let client_handler = NetworkHandler::new(client_config).unwrap();

//     // Poll for events
//     while let Some(event) = host_handler.poll_events() {
//         match event {
//             NetworkEvent::Connected(peer_id) => {
//                 println!("New peer connected: {}", peer_id);
//             }
//             NetworkEvent::Disconnected(peer_id) => {
//                 println!("Peer disconnected: {}", peer_id);
//             }
//             _ => {}
//         }
//     }
// }