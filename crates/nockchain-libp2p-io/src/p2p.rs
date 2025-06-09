use std::convert::Infallible;
use std::error::Error;

use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use libp2p::identity::Keypair;
use libp2p::multiaddr::Multiaddr;
use libp2p::request_response::{self, cbor, ResponseChannel};
use libp2p::swarm::behaviour::toggle::Toggle;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{
    allow_block_list, connection_limits, identify, kad, memory_connection_limits, ping, PeerId,
    Swarm,
};
use nockapp::NockAppError;
use tracing::{debug, error, trace};

use crate::config::LibP2PConfig;
use crate::nc::*;

#[derive(Debug)]
pub enum SwarmAction {
    SendResponse {
        channel: ResponseChannel<NockchainResponse>,
        response: NockchainResponse,
    },
    SendRequest {
        peer_id: PeerId,
        request: NockchainRequest,
    },
    BlockPeer {
        peer_id: PeerId,
    },
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "NockchainEvent")]
/** Composed [NetworkBehaviour] implementation for Nockchain */
pub struct NockchainBehaviour {
    /// Allows nodes to connect via just IP and port and exchange pubkeys
    identify: identify::Behaviour,
    /// Connectivity testing
    ping: ping::Behaviour,
    /// Peer discovery via a DHT
    pub kad: kad::Behaviour<kad::store::MemoryStore>,
    /// Peer banning
    pub allow_block_list: allow_block_list::Behaviour<allow_block_list::BlockedPeers>,
    /// Peer whitelisting
    pub allow_peers: Toggle<allow_block_list::Behaviour<allow_block_list::AllowedPeers>>,
    /// Connection limiting
    connection_limits: connection_limits::Behaviour,
    /// Memory connection limits
    memory_connection_limits: Toggle<memory_connection_limits::Behaviour>,
    /// Peer store for tracking peer information (including addresses)
    pub peer_store: libp2p::peer_store::Behaviour<libp2p::peer_store::memory_store::MemoryStore>,
    /// Actual comms
    pub request_response: cbor::Behaviour<NockchainRequest, NockchainResponse>,
}

impl NockchainBehaviour {
    fn pre_new(
        libp2p_config: LibP2PConfig,
        allowed: Option<allow_block_list::Behaviour<allow_block_list::AllowedPeers>>,
        limits: connection_limits::ConnectionLimits,
        memory_limits: Option<memory_connection_limits::Behaviour>,
    ) -> impl FnOnce(&libp2p::identity::Keypair) -> Self {
        move |keypair: &libp2p::identity::Keypair| {
            let peer_id = libp2p::identity::PeerId::from_public_key(&keypair.public());

            let identify_config = identify::Config::new(
                libp2p_config.identify_protocol_version.clone(),
                keypair.public(),
            )
            .with_interval(libp2p_config.identify_interval())
            .with_hide_listen_addrs(true); // Only send externally confirmed addresses so we don't send loopback addresses
            let identify_behaviour = identify::Behaviour::new(identify_config);

            let memory_store = kad::store::MemoryStore::new(peer_id);

            let kad_config = kad::Config::new(libp2p::StreamProtocol::new(
                LibP2PConfig::kad_protocol_version(),
            ));
            let kad_behaviour = kad::Behaviour::with_config(peer_id, memory_store, kad_config);

            let request_response_config = request_response::Config::default()
                .with_max_concurrent_streams(
                    libp2p_config.request_response_max_concurrent_streams(),
                )
                .with_request_timeout(libp2p_config.request_response_timeout());

            let request_response_behaviour = cbor::Behaviour::new(
                [(
                    libp2p::StreamProtocol::new(LibP2PConfig::req_res_protocol_version()),
                    request_response::ProtocolSupport::Full,
                )],
                request_response_config,
            );
            let connection_limits_behaviour = connection_limits::Behaviour::new(limits);
            let memory_connection_limits =
                Toggle::<memory_connection_limits::Behaviour>::from(memory_limits);

            let allow_peers =
                Toggle::<allow_block_list::Behaviour<allow_block_list::AllowedPeers>>::from(
                    allowed,
                );
            let peer_store_config = libp2p::peer_store::memory_store::Config::default();
            let record_capacity = libp2p_config.peer_store_record_capacity;
            let peer_store_config = peer_store_config.set_record_capacity(record_capacity);
            let peer_store_memory =
                libp2p::peer_store::memory_store::MemoryStore::new(peer_store_config);

            let peer_store_behaviour = libp2p::peer_store::Behaviour::new(peer_store_memory);
            NockchainBehaviour {
                ping: ping::Behaviour::default(),
                identify: identify_behaviour,
                kad: kad_behaviour,
                allow_block_list: allow_block_list::Behaviour::default(),
                allow_peers,
                request_response: request_response_behaviour,
                connection_limits: connection_limits_behaviour,
                memory_connection_limits,
                peer_store: peer_store_behaviour,
            }
        }
    }
}

/// # Create a swarm and set it to listen
///
/// This function initializes a libp2p swarm with the provided keypair and binding addresses.
/// It configures the swarm to listen on specified multiaddresses and sets up the behavior for network interactions.
///
/// # Arguments
/// * `keypair` - The keypair for the node's identity
/// * `bind` - A vector of multiaddresses specifying the network interfaces to bind to
///
/// # Returns
/// A Result containing the Swarm instance or an error if any operation fails
pub fn start_swarm(
    libp2p_config: LibP2PConfig,
    keypair: Keypair,
    bind: Vec<Multiaddr>,
    allowed: Option<allow_block_list::Behaviour<allow_block_list::AllowedPeers>>,
    limits: connection_limits::ConnectionLimits,
    memory_limits: Option<memory_connection_limits::Behaviour>,
) -> Result<Swarm<NockchainBehaviour>, Box<dyn Error>> {
    let (resolver_config, resolver_opts) =
        if let Ok(sys) = hickory_resolver::system_conf::read_system_conf() {
            debug!("resolver configs and opts: {:?}", sys);
            sys
        } else {
            (ResolverConfig::cloudflare(), ResolverOpts::default())
        };

    let max_idle_timeout_millisecs = libp2p_config.max_idle_timeout_millisecs();
    let keep_alive_interval = libp2p_config.keep_alive_interval();
    let handshake_timeout = libp2p_config.handshake_timeout();
    let connection_timeout = libp2p_config.connection_timeout();
    let swarm_idle_timeout = libp2p_config.swarm_idle_timeout();
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_quic_config(|mut cfg| {
            cfg.max_idle_timeout = max_idle_timeout_millisecs;
            cfg.keep_alive_interval = keep_alive_interval;
            cfg.handshake_timeout = handshake_timeout;
            cfg
        })
        .with_dns_config(resolver_config, resolver_opts)
        .with_behaviour(NockchainBehaviour::pre_new(
            libp2p_config, allowed, limits, memory_limits,
        ))?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(swarm_idle_timeout))
        .with_connection_timeout(connection_timeout)
        .build();

    for bind_addr in bind {
        swarm.listen_on(bind_addr.clone()).map_err(|e| {
            error!("Failed to listen on {bind_addr:?}: {e}");
            e
        })?;
    }
    Ok(swarm)
}

// TODO: We need to box identify::Event but we are on stable so no boxed patterns.
#[derive(Debug)]
#[allow(dead_code)]
#[allow(clippy::large_enum_variant)]
/** Events that can be emitted by the swarm running [NockchainBehaviour] */
pub enum NockchainEvent {
    /// Received or sent identify message
    Identify(identify::Event),
    /// Received or failed ping
    Ping(ping::Event),
    /// DHT state changes
    Kad(kad::Event),
    /// Request or response received from peer
    RequestResponse(request_response::Event<NockchainRequest, NockchainResponse>),
    /// Peer store events
    PeerStore(libp2p::peer_store::memory_store::Event),
}

impl From<identify::Event> for NockchainEvent {
    fn from(event: identify::Event) -> Self {
        Self::Identify(event)
    }
}

impl From<ping::Event> for NockchainEvent {
    fn from(event: ping::Event) -> Self {
        Self::Ping(event)
    }
}

impl From<kad::Event> for NockchainEvent {
    fn from(event: kad::Event) -> Self {
        Self::Kad(event)
    }
}

impl From<Infallible> for NockchainEvent {
    fn from(i: Infallible) -> Self {
        match i {}
    }
}

impl From<request_response::Event<NockchainRequest, NockchainResponse>> for NockchainEvent {
    fn from(event: request_response::Event<NockchainRequest, NockchainResponse>) -> Self {
        Self::RequestResponse(event)
    }
}

impl From<libp2p::peer_store::memory_store::Event> for NockchainEvent {
    fn from(event: libp2p::peer_store::memory_store::Event) -> Self {
        Self::PeerStore(event)
    }
}

///** Handler for "identify" messages */
//#[instrument(skip(swarm))]
pub fn identify_received(
    swarm: &mut Swarm<NockchainBehaviour>,
    peer_id: PeerId,
    info: libp2p::identify::Info,
) -> Result<(), NockAppError> {
    swarm.add_external_address(info.observed_addr.clone());
    let us = *swarm.local_peer_id();
    let kad = &mut swarm.behaviour_mut().kad;
    trace!("identify received for peer {}", peer_id);
    trace!("Adding address {} for us: {}", info.observed_addr, us);
    kad.add_address(&us, info.observed_addr);
    for addr in info.listen_addrs {
        trace!("Adding address {} for peer {}", addr, peer_id);
        kad.add_address(&peer_id, addr);
    }
    Ok(())
}
