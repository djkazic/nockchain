use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub mod memory_pool;
pub mod streaming_prover;

// Re-export main functions
pub use memory_pool::init_memory_pools;
pub use streaming_prover::{StreamingProver, report_memory_usage};

/// Initialize the optimized miner library
#[no_mangle]
pub extern "C" fn init_optimized_miner() {
    init_memory_pools();
    println!("Nockchain optimized miner initialized");
}

/// Get version string
#[no_mangle]
pub extern "C" fn get_version() -> *const u8 {
    b"nockchain-optimized-v0.1.0\0".as_ptr()
}
