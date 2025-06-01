use std::sync::Mutex;
use std::collections::VecDeque;
use bumpalo::Bump;

// Global memory pools for different sizes
lazy_static::lazy_static! {
    static ref SMALL_POOL: Mutex<VecDeque<Vec<u64>>> = Mutex::new(VecDeque::new());
    static ref MEDIUM_POOL: Mutex<VecDeque<Vec<u64>>> = Mutex::new(VecDeque::new());
    static ref LARGE_POOL: Mutex<VecDeque<Vec<u64>>> = Mutex::new(VecDeque::new());
    static ref BUMP_ALLOCATOR: Mutex<Bump> = Mutex::new(Bump::with_capacity(1 << 28)); // 256MB
}

pub struct PooledVec {
    data: Vec<u64>,
    size_class: SizeClass,
}

#[derive(Clone, Copy)]
enum SizeClass {
    Small,  // <= 1024 elements
    Medium, // <= 65536 elements  
    Large,  // > 65536 elements
}

impl PooledVec {
    pub fn new(size: usize) -> Self {
        let size_class = match size {
            0..=1024 => SizeClass::Small,
            1025..=65536 => SizeClass::Medium,
            _ => SizeClass::Large,
        };

        let data = match size_class {
            SizeClass::Small => {
                SMALL_POOL.lock().unwrap()
                    .pop_front()
                    .map(|mut v| { v.clear(); v.resize(size, 0); v })
                    .unwrap_or_else(|| vec![0u64; size])
            }
            SizeClass::Medium => {
                MEDIUM_POOL.lock().unwrap()
                    .pop_front()
                    .map(|mut v| { v.clear(); v.resize(size, 0); v })
                    .unwrap_or_else(|| vec![0u64; size])
            }
            SizeClass::Large => {
                LARGE_POOL.lock().unwrap()
                    .pop_front()
                    .map(|mut v| { v.clear(); v.resize(size, 0); v })
                    .unwrap_or_else(|| vec![0u64; size])
            }
        };

        PooledVec { data, size_class }
    }

    pub fn as_slice(&self) -> &[u64] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [u64] {
        &mut self.data
    }
}

impl Drop for PooledVec {
    fn drop(&mut self) {
        // Return vector to pool when dropped
        let data = std::mem::take(&mut self.data);
        
        match self.size_class {
            SizeClass::Small => {
                let mut pool = SMALL_POOL.lock().unwrap();
                if pool.len() < 100 {  // Keep max 100 small vecs
                    pool.push_back(data);
                }
            }
            SizeClass::Medium => {
                let mut pool = MEDIUM_POOL.lock().unwrap();
                if pool.len() < 50 {   // Keep max 50 medium vecs
                    pool.push_back(data);
                }
            }
            SizeClass::Large => {
                let mut pool = LARGE_POOL.lock().unwrap();
                if pool.len() < 10 {   // Keep max 10 large vecs
                    pool.push_back(data);
                }
            }
        }
    }
}

// Bump allocator for temporary allocations
pub struct TempAllocator<'a> {
    bump: &'a Bump,
}

impl<'a> TempAllocator<'a> {
    pub fn new() -> Self {
        let bump = &*BUMP_ALLOCATOR.lock().unwrap();
        // Safety: We're careful to not hold this reference across allocations
        let bump = unsafe { &*(bump as *const Bump) };
        TempAllocator { bump }
    }

    pub fn alloc_slice(&self, size: usize) -> &'a mut [u64] {
        self.bump.alloc_slice_fill_copy(size, 0u64)
    }

    pub fn reset() {
        BUMP_ALLOCATOR.lock().unwrap().reset();
    }
}

// Pre-warm the pools
pub fn init_memory_pools() {
    println!("Initializing memory pools...");
    
    // Pre-allocate some vectors
    {
        let mut small = SMALL_POOL.lock().unwrap();
        for _ in 0..20 {
            small.push_back(vec![0u64; 1024]);
        }
    }
    
    {
        let mut medium = MEDIUM_POOL.lock().unwrap();
        for _ in 0..10 {
            medium.push_back(vec![0u64; 65536]);
        }
    }
    
    println!("Memory pools initialized");
}
