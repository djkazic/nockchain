#!/bin/bash
source .env

# Memory optimization environment variables
export RUST_LOG=${RUST_LOG:-info}
export MINIMAL_LOG_FORMAT
export MINING_PUBKEY

# Use mimalloc features
export MIMALLOC_LARGE_OS_PAGES=1
export MIMALLOC_RESERVE_HUGE_OS_PAGES=2

echo "Starting Nockchain miner with memory optimizations..."
echo "- Using MiMalloc allocator"
echo "- Kernel pooling enabled"
echo "- Large OS pages enabled"

# Monitor memory usage in background
(
    while true; do
        sleep 30
        MEM=$(ps aux | grep nockchain | grep -v grep | awk '{print $6/1024 " MB"}' | head -1)
        if [ ! -z "$MEM" ]; then
            echo "[Memory Monitor] Current usage: $MEM"
        fi
    done
) &
MONITOR_PID=$!

# Run the miner
nockchain --mining-pubkey ${MINING_PUBKEY} --mine

# Clean up
kill $MONITOR_PID 2>/dev/null
