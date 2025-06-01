use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod memory_pool;

fn main() {
    println!("Nockchain Optimized Miner v0.1.0");
    
    // Initialize memory pools
    memory_pool::init_memory_pools();
    
    // Set process priority (optional, requires root on Linux)
    #[cfg(target_os = "linux")]
    unsafe {
        libc::nice(-10); // Higher priority
    }
    
    // Get the existing miner code and run it
    // For now, this is a placeholder - you'll integrate with existing code
    run_existing_miner();
}

fn run_existing_miner() {
    // This is where you'll call your existing miner code
    // For now, let's add a simple test to verify memory pool works
    
    println!("Testing memory pool...");
    
    // Allocate and deallocate to test pool
    for i in 0..100 {
        let mut vec = memory_pool::PooledVec::new(1024);
        vec.as_mut_slice()[0] = i;
        // vec automatically returned to pool when dropped
    }
    
    println!("Memory pool test complete");
    
    // TODO: Call actual miner code here
    // nockchain::run_miner();
}
