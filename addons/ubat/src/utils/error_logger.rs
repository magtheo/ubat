// In a new file: error_logger.rs
pub mod error_logger {
    use godot::prelude::*;
    use std::sync::Mutex;
    use std::collections::VecDeque;
    
    // Maximum number of errors to keep in history
    const MAX_ERROR_HISTORY: usize = 100;
    
    // Thread-safe error log
    lazy_static! {
        static ref ERROR_LOG: Mutex<VecDeque<String>> = Mutex::new(VecDeque::with_capacity(MAX_ERROR_HISTORY));
    }
    
    pub fn log_error(module: &str, message: &str) {
        let error_message = format!("[{}] {}", module, message);
        
        // Print to Godot console
        godot_error!("{}", error_message);
        
        // Add to error history
        if let Ok(mut log) = ERROR_LOG.lock() {
            // Add new error
            log.push_back(error_message);
            
            // Remove oldest if exceeding capacity
            if log.len() > MAX_ERROR_HISTORY {
                log.pop_front();
            }
        }
    }
    
    pub fn get_error_history() -> Vec<String> {
        if let Ok(log) = ERROR_LOG.lock() {
            return log.iter().cloned().collect();
        }
        Vec::new()
    }
}