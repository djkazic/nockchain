#!/bin/bash
source .env

# Memory optimization environment variables
export RUST_LOG=${RUST_LOG:-info}
export MINIMAL_LOG_FORMAT
export MINING_PUBKEY

# Use mimalloc features
export MIMALLOC_LARGE_OS_PAGES=1
export MIMALLOC_RESERVE_HUGE_OS_PAGES=4

# Limit memory usage to prevent runaway allocation
ulimit -v 10485760  # 10GB virtual memory limit

# Set CPU governor to performance (requires root)
if [ "$EUID" -eq 0 ]; then
    echo performance | tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor > /dev/null
fi

# Pre-load optimized library
export LD_PRELOAD="./nockchain-optimized/target/release/libnockchain_optimized.so:$LD_PRELOAD"

echo "Starting Nockchain miner with memory optimizations..."
echo "Memory limit: 10GB"
echo "Using mimalloc with huge pages"

# Monitor memory usage in background
(
    while true; do
        sleep 30
        echo "Memory usage: $(ps aux | grep nockchain | grep -v grep | awk '{print $6/1024 " MB"}')"
    done
) &
MONITOR_PID=$!

# Run the miner
nockchain --mining-pubkey ${MINING_PUBKEY} --mine

# Clean up
kill $MONITOR_PID 2>/dev/null
