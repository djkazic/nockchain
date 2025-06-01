use std::sync::Arc;
use tokio::sync::Mutex;
use tempfile::TempDir;
use nockapp::kernel::form::Kernel;
use nockapp::kernel::checkpoint::JamPaths;
use tracing::info;

pub struct KernelPool {
    kernel: Arc<Mutex<Option<(Kernel, TempDir, JamPaths)>>>,
}

impl KernelPool {
    pub fn new() -> Self {
        Self {
            kernel: Arc::new(Mutex::new(None)),
        }
    }
    
    pub async fn get_or_create<F, Fut>(
        &self, 
        create_fn: F
    ) -> Result<(Kernel, TempDir, JamPaths), Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(Kernel, TempDir, JamPaths), Box<dyn std::error::Error + Send + Sync>>>,
    {
        let mut kernel_opt = self.kernel.lock().await;
        
        if let Some(kernel_data) = kernel_opt.take() {
            info!("Reusing existing kernel from pool");
            Ok(kernel_data)
        } else {
            info!("Creating new kernel");
            create_fn().await
        }
    }
    
    pub async fn return_kernel(&self, kernel_data: (Kernel, TempDir, JamPaths)) {
        let mut kernel_opt = self.kernel.lock().await;
        if kernel_opt.is_none() {
            info!("Returning kernel to pool");
            *kernel_opt = Some(kernel_data);
        }
    }
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_KERNEL_POOL: KernelPool = KernelPool::new();
}
