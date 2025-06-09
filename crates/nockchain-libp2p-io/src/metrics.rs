use gnort::*;

metrics_struct![
    NockchainP2PMetrics,
    (gossip_acked, "nockchain-libp2p-io.gossip_acked", Count),
    (gossip_nacked, "nockchain-libp2p-io.gossip_nacked", Count),
    (gossip_erred, "nockchain-libp2p-io.gossip_erred", Count),
    (gossip_dropped, "nockchain-libp2p-io.gossip_dropped", Count),
    (requests_peeked_some, "nockchain-libp2p-io.requests_peeked_some", Count),
    (requests_peeked_none, "nockchain-libp2p-io.requests_peeked_none", Count),
    (requests_erred, "nockchain-libp2p-io.requests_erred", Count),
    (requests_dropped, "nockchain-libp2p-io.requests_dropped", Count),
    (responses_acked, "nockchain-libp2p-io.responses_acked", Count),
    (responses_nacked, "nockchain-libp2p-io.responses_nacked", Count),
    (responses_erred, "nockchain-libp2p-io.responses_erred", Count),
    (responses_dropped, "nockchain-libp2p-io.responses_dropped", Count),
    (block_request_cache_hits, "nockchain-libp2p-io.block_request_cache_hits", Count),
    (tx_request_cache_hits, "nockchain-libp2p-io.tx_request_cache_hits", Count),
    (block_seen_cache_hits, "nockchain-libp2p-io.block_seen_cache_hits", Count),
    (tx_seen_cache_hits, "nockchain-libp2p-io.tx_seen_cache_hits", Count),
    (block_request_cache_misses, "nockchain-libp2p-io.block_request_cache_misses", Count),
    (block_request_cache_negative, "nockchain-libp2p-io.block_request_cache_negative", Count),
    (tx_request_cache_misses, "nockchain-libp2p-io.tx_request_cache_misses", Count),
    (block_seen_cache_misses, "nockchain-libp2p-io.block_seen_cache_misses", Count),
    (tx_seen_cache_misses, "nockchain-libp2p-io.tx_seen_cache_misses", Count),
    (highest_block_height_seen, "nockchain-libp2p-io.highest_block_height_seen", Gauge),
    (peer_count, "nockchain-libp2p-io.peer_count", Gauge),
    // Peer connection health
    (peer_connections_established, "nockchain-libp2p-io.peer_connections_established", Count),
    (peer_connections_closed, "nockchain-libp2p-io.peer_connections_closed", Count),
    (peer_connection_failures, "nockchain-libp2p-io.peer_connection_failures", Count),
    (kademlia_bootstrap_attempts, "nockchain-libp2p-io.kademlia_bootstrap_attempts", Count),
    (kademlia_bootstrap_failures, "nockchain-libp2p-io.kademlia_bootstrap_failures", Count),
    (active_peer_connections, "nockchain-libp2p-io.active_peer_connections", Gauge),
    // Block sync progress
    (blocks_requested_by_height, "nockchain-libp2p-io.blocks_requested_by_height", Count),
    (blocks_received_by_height, "nockchain-libp2p-io.blocks_received_by_height", Count),
    (block_request_timeouts, "nockchain-libp2p-io.block_request_timeouts", Count),
    (last_block_height_received, "nockchain-libp2p-io.last_block_height_received", Gauge),
    // Request/response patterns
    (
        request_response_active_streams, "nockchain-libp2p-io.request_response_active_streams",
        Gauge
    ),
    (peer_request_rate_limited, "nockchain-libp2p-io.peer_request_rate_limited", Count),
    (request_failed, "nockchain-libp2p-io.request_failed", Count),
    (response_failed_not_dropped, "nockchain-libp2p-io.response_failed_not_dropped", Count),
    (response_dropped, "nockchain-libp2p-io.response_dropped", Count)
];
