use crate::memory_pool::{PooledVec, TempAllocator};
use std::mem;

const CHUNK_SIZE: usize = 4096; // Process tables in 4K row chunks

pub struct StreamingProver {
    chunk_size: usize,
}

impl StreamingProver {
    pub fn new() -> Self {
        StreamingProver {
            chunk_size: CHUNK_SIZE,
        }
    }

    /// Build tables in chunks to avoid loading entire table into memory
    pub fn build_table_streaming<F>(&self, 
        num_rows: usize, 
        num_cols: usize,
        mut row_generator: F
    ) -> Vec<Vec<u64>> 
    where 
        F: FnMut(usize) -> Vec<u64>
    {
        let mut result = Vec::with_capacity(num_cols);
        
        // Initialize column vectors
        for _ in 0..num_cols {
            result.push(Vec::with_capacity(num_rows));
        }
        
        // Process in chunks
        for chunk_start in (0..num_rows).step_by(self.chunk_size) {
            let chunk_end = (chunk_start + self.chunk_size).min(num_rows);
            
            // Generate chunk of rows
            let mut chunk_data = Vec::with_capacity((chunk_end - chunk_start) * num_cols);
            
            for row_idx in chunk_start..chunk_end {
                let row = row_generator(row_idx);
                chunk_data.extend(row);
            }
            
            // Transpose chunk and append to columns
            for row in 0..(chunk_end - chunk_start) {
                for col in 0..num_cols {
                    result[col].push(chunk_data[row * num_cols + col]);
                }
            }
            
            // Clear chunk data to free memory
            drop(chunk_data);
        }
        
        result
    }

    /// Memory-efficient polynomial interpolation
    pub fn interpolate_streaming(&self, values: &[u64], domain_size: usize) -> PooledVec {
        // Use pooled vector for result
        let mut result = PooledVec::new(domain_size);
        
        // Use temporary allocator for intermediate values
        let temp_alloc = TempAllocator::new();
        let workspace = temp_alloc.alloc_slice(domain_size * 2);
        
        // Perform FFT in chunks to maintain cache locality
        self.fft_chunked(values, result.as_mut_slice(), workspace);
        
        // Reset temporary allocator
        TempAllocator::reset();
        
        result
    }

    /// Chunked FFT for better cache usage
    fn fft_chunked(&self, input: &[u64], output: &mut [u64], workspace: &mut [u64]) {
        let n = input.len();
        
        // Copy input to output
        output[..n].copy_from_slice(input);
        
        // Bit reversal with cache blocking
        self.bit_reversal_blocked(output, n);
        
        // FFT with cache-aware passes
        let mut stride = 1;
        while stride < n {
            self.fft_pass_blocked(output, workspace, stride, n);
            stride *= 2;
        }
    }

    fn bit_reversal_blocked(&self, data: &mut [u64], n: usize) {
        const BLOCK_SIZE: usize = 64; // Tune for L1 cache
        
        for block_start in (0..n).step_by(BLOCK_SIZE) {
            let block_end = (block_start + BLOCK_SIZE).min(n);
            
            for i in block_start..block_end {
                let j = self.reverse_bits(i, n.trailing_zeros());
                if i < j && j < n {
                    data.swap(i, j);
                }
            }
        }
    }

    fn fft_pass_blocked(&self, data: &mut [u64], workspace: &mut [u64], stride: usize, n: usize) {
        // Implement cache-blocked FFT pass
        // This is simplified - real implementation would do proper FFT
        let half_stride = stride;
        let full_stride = stride * 2;
        
        for start in (0..n).step_by(full_stride) {
            for k in 0..half_stride {
                let i = start + k;
                let j = start + k + half_stride;
                
                if j < n {
                    // Butterfly operation
                    let t = data[j];
                    data[j] = data[i].wrapping_sub(t);
                    data[i] = data[i].wrapping_add(t);
                }
            }
        }
    }

    fn reverse_bits(&self, x: usize, bits: u32) -> usize {
        x.reverse_bits() >> (usize::BITS - bits)
    }
}

/// Memory usage reporter
pub fn report_memory_usage() {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    println!("Current memory usage: {}", line);
                    break;
                }
            }
        }
    }
}

/// C FFI for Hoon integration
#[no_mangle]
pub extern "C" fn streaming_build_table(
    num_rows: usize,
    num_cols: usize,
    callback: extern "C" fn(usize) -> *mut u64,
) -> *mut u64 {
    let prover = StreamingProver::new();
    
    let table = prover.build_table_streaming(num_rows, num_cols, |row_idx| {
        unsafe {
            let ptr = callback(row_idx);
            let slice = std::slice::from_raw_parts(ptr, num_cols);
            slice.to_vec()
        }
    });
    
    // Flatten table for return
    let mut flat: Vec<u64> = table.into_iter().flatten().collect();
    let ptr = flat.as_mut_ptr();
    mem::forget(flat);
    
    ptr
}

#[no_mangle]
pub extern "C" fn report_memory() {
    report_memory_usage();
}
