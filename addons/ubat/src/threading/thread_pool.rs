use rayon::ThreadPoolBuilder;
use std::sync::{Arc, RwLock, OnceLock};
use num_cpus;
use godot::prelude::*;

// A wrapper around Rayon's ThreadPool that provides a clean interface for our terrain generation
pub struct ThreadPool {
    pool: rayon::ThreadPool,
    num_threads: usize,
}

impl ThreadPool {
    // Create a new ThreadPool with the specified number of threads
    // If size is 0, it will use num_cpus::get() to determine the optimal number
    pub fn new(size: usize) -> ThreadPool {
        let num_threads = if size > 0 { size } else { num_cpus::get() };
        
        // Create a Rayon thread pool with the specified number of threads
        let pool = ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .expect("Failed to build Rayon thread pool");
        
        godot_print!("Created thread pool with {} threads", num_threads);
        
        ThreadPool { 
            pool,
            num_threads,
        }
    }

    // Execute a job in the thread pool
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.pool.spawn(f);
    }
    
    // Execute a closure and wait for it to complete (blocking)
    pub fn execute_wait<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        
        self.pool.spawn(move || {
            let result = f();
            tx.send(result).expect("Failed to send result via channel");
        });
        
        rx.recv().expect("Failed to receive result from thread")
    }
    
    // Get the number of threads in the pool
    pub fn num_threads(&self) -> usize {
        self.num_threads
    }
    
    // Execute a parallel task on a slice of data (using Rayon's parallel iterator)
    pub fn par_execute<T, F, R>(&self, data: &[T], f: F) -> Vec<R>
        where
            T: Send + Sync,
            F: Fn(&T) -> R + Send + Sync,
            R: Send,
        {
            use rayon::prelude::*;
            self.pool.install(|| {
                data.par_iter().map(f).collect()
            })
        }

    
    // Create a shared thread-local context that can be used by worker threads
    pub fn with_thread_local_context<T, F>(&self, create_context: F) -> ThreadLocalContext<T>
    where
        T: Send + 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        let contexts = (0..self.num_threads)
            .map(|_| create_context())
            .collect::<Vec<_>>();
        ThreadLocalContext {
            contexts: Arc::new(contexts),
        }
    }

}

// Helper struct to manage thread-local contexts
pub struct ThreadLocalContext<T> {
    contexts: Arc<Vec<T>>,
}

impl<T> ThreadLocalContext<T> 
where
    T: Send + 'static
{
    // Get a reference to the context for the current thread
    pub fn get(&self, thread_id: usize) -> &T {
        &self.contexts[thread_id % self.contexts.len()]
    }
}

// Global thread pool for easy access
static GLOBAL_THREAD_POOL: OnceLock<Arc<RwLock<ThreadPool>>> = OnceLock::new();

// Initialize the global thread pool with the specified number of threads
pub fn initialize_global_pool(num_threads: usize) -> Result<(), &'static str> {
    let pool = ThreadPool::new(num_threads);
    let pool_arc = Arc::new(RwLock::new(pool));

    // set returns Ok if the OnceLock was empty, Err if it was already populated.
    GLOBAL_THREAD_POOL.set(pool_arc)
        .map_err(|_| "Global thread pool already initialized")
}

// Get a reference to the global thread pool
pub fn global_thread_pool() -> Option<Arc<RwLock<ThreadPool>>> {
    // .get() returns Option<&T>, so we clone the Arc if it exists.
    GLOBAL_THREAD_POOL.get().cloned()
}

// Call this instead of global_thread_pool() if you want lazy init.
// Be careful: the first call determines the size.
pub fn get_or_init_global_pool() -> Arc<RwLock<ThreadPool>> {
    GLOBAL_THREAD_POOL.get_or_init(|| {
        godot_print!("Lazily initializing global thread pool with default threads.");
        let pool = ThreadPool::new(0); // 0 uses num_cpus
        Arc::new(RwLock::new(pool))
    }).clone() // Clone the Arc for the caller
}
