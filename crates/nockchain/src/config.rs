use std::path::PathBuf;
use std::time::Duration;

use clap::{arg, command, value_parser, ArgAction, Parser};
use nockchain_bitcoin_sync::BitcoinRPCConnection;

use crate::mining::MiningKeyConfig;

// TODO: command-line/configure
/** Path to read current node's identity from */
pub const IDENTITY_PATH: &str = ".nockchain_identity";

/** Path to read current node's peer ID from */
pub const PEER_ID_EXTENSION: &str = ".peer_id";

// TODO: command-line/configure
/** Extension for peer ID files */
pub const PEER_ID_FILE_EXTENSION: &str = "peerid";

// Libp2p multiaddrs don't support const construction, so we have to put strings literals and parse them at startup
/** Backbone nodes for our testnet */
pub const TESTNET_BACKBONE_NODES: &[&str] = &[];

// Libp2p multiaddrs don't support const construction, so we have to put strings literals and parse them at startup
// TODO: feature flag testnet/realnet
/** Backbone nodes for our realnet */
#[allow(dead_code)]
pub const REALNET_BACKBONE_NODES: &[&str] = &["/dnsaddr/nockchain-backbone.zorp.io"];

/** How often we should affirmatively ask other nodes for their heaviest chain */
pub const CHAIN_INTERVAL: Duration = Duration::from_secs(20);

/// The height of the bitcoin block that we want to sync our genesis block to
/// Currently, this is the height of an existing block for testing. It will be
/// switched to a future block for launch.
pub const GENESIS_HEIGHT: u64 = 897767;

/// Command line arguments
#[derive(Parser, Debug, Clone)]
#[command(name = "nockchain")]
pub struct NockchainCli {
    #[command(flatten)]
    pub nockapp_cli: nockapp::kernel::boot::Cli,
    #[arg(
        long,
        help = "npc socket path",
        default_value = ".socket/nockchain_npc.sock"
    )]
    pub npc_socket: String,
    #[arg(long, help = "Mine in-kernel", default_value = "false")]
    pub mine: bool,
    #[arg(
        long,
        help = "Pubkey to mine to (mutually exclusive with --mining-key-adv)"
    )]
    pub mining_pubkey: Option<String>,
    #[arg(
        long,
        help = "Advanced mining key configuration (mutually exclusive with --mining-pubkey). Format: share,m:key1,key2,key3",
        value_parser = value_parser!(MiningKeyConfig),
        num_args = 1..,
        value_delimiter = ',',
    )]
    pub mining_key_adv: Option<Vec<MiningKeyConfig>>,
    #[arg(long, help = "Watch for genesis block", default_value = "false")]
    pub genesis_watcher: bool,
    #[arg(long, help = "Mine genesis block", default_value = "false")]
    pub genesis_leader: bool,
    #[arg(long, help = "use fake genesis block", default_value = "false")]
    pub fakenet: bool,
    #[arg(long, help = "Genesis block message", default_value = "Hail Zorp")]
    pub genesis_message: String,
    #[arg(
        long,
        help = "URL for Bitcoin Core RPC",
        default_value = "http://100.98.183.39:8332"
    )]
    pub btc_node_url: String,
    #[arg(long, help = "Username for Bitcoin Core RPC")]
    pub btc_username: Option<String>,
    #[arg(long, help = "Password for Bitcoin Core RPC")]
    pub btc_password: Option<String>,
    #[arg(long, help = "Auth cookie path for Bitcoin Core RPC")]
    pub btc_auth_cookie: Option<String>,
    #[arg(long, short, help = "Initial peer", action = ArgAction::Append)]
    pub peer: Vec<String>,
    #[arg(long, short, help = "Force peer", action = ArgAction::Append)]
    pub force_peer: Vec<String>,
    #[arg(long, help = "Allowed peer IDs file")]
    pub allowed_peers_path: Option<String>,
    #[arg(long, help = "Don't dial default peers")]
    pub no_default_peers: bool,
    #[arg(long, help = "Bind address", action = ArgAction::Append)]
    pub bind: Vec<String>,
    #[arg(
        long,
        help = "Generate a new peer ID, discarding the existing one",
        default_value = "false"
    )]
    pub new_peer_id: bool,
    #[arg(long, help = "Maximum established incoming connections")]
    pub max_established_incoming: Option<u32>,
    #[arg(long, help = "Maximum established outgoing connections")]
    pub max_established_outgoing: Option<u32>,
    #[arg(long, help = "Maximum pending incoming connections")]
    pub max_pending_incoming: Option<u32>,
    #[arg(long, help = "Maximum pending outgoing connections")]
    pub max_pending_outgoing: Option<u32>,
    #[arg(long, help = "Maximum established connections")]
    pub max_established: Option<u32>,
    #[arg(long, help = "Maximum established connections per peer")]
    pub max_established_per_peer: Option<u32>,
    #[arg(long, help = "Maximum system memory percentage for connection limits")]
    pub max_system_memory_fraction: Option<f64>,
    #[arg(long, help = "Maximum process memory for connection limits (bytes)")]
    pub max_system_memory_bytes: Option<usize>,
}

impl NockchainCli {
    pub fn validate(&self) -> Result<(), String> {
        if self.mine && !(self.mining_pubkey.is_some() || self.mining_key_adv.is_some()) {
            return Err(
                "Cannot specify mine without either mining_pubkey or mining_key_adv".to_string(),
            );
        }

        if self.mining_pubkey.is_some() && self.mining_key_adv.is_some() {
            return Err(
                "Cannot specify both mining_pubkey and mining_key_adv at the same time".to_string(),
            );
        }

        if self.genesis_leader && self.genesis_watcher {
            return Err(
                "Cannot specify both genesis_leader and genesis_watcher at the same time"
                    .to_string(),
            );
        }

        if !self.fakenet && (self.genesis_watcher || self.genesis_leader) {
            if self.btc_node_url.is_empty() {
                return Err(
                    "Must specify --btc-node-url when using genesis_watcher or genesis_leader"
                        .to_string(),
                );
            }
            if self.btc_auth_cookie.is_none() {
                if self.btc_username.is_none() && self.btc_password.is_none() {
                    return Err("Must specify either --btc-username or --btc-password when using genesis_watcher or genesis_leader on livenet".to_string());
                }
            }
        }

        Ok(())
    }

    /// Helper function to create a BitcoinRPCConnection from CLI arguments
    pub fn create_bitcoin_connection(&self) -> BitcoinRPCConnection {
        let url = self.btc_node_url.clone();
        let height = GENESIS_HEIGHT;
        let auth = if let Some(username) = self.btc_username.clone() {
            let password = self.btc_password.clone().unwrap_or_else(|| {
                panic!(
                    "Panicked at {}:{} (git sha: {:?})",
                    file!(),
                    line!(),
                    option_env!("GIT_SHA")
                )
            });
            bitcoincore_rpc::Auth::UserPass(username, password)
        } else {
            let cookie_path_str = self.btc_auth_cookie.clone().unwrap_or_else(|| {
                panic!(
                    "Panicked at {}:{} (git sha: {:?})",
                    file!(),
                    line!(),
                    option_env!("GIT_SHA")
                )
            });
            let cookie_path = PathBuf::from(cookie_path_str);
            bitcoincore_rpc::Auth::CookieFile(cookie_path)
        };
        BitcoinRPCConnection::new(url, auth, height)
    }
}
