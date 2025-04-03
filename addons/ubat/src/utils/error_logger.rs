use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use chrono::{DateTime, Utc};
use godot::prelude::*;

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorSeverity {
    Warning,
    Error,
    Critical,
}

/// Detailed error log entry
#[derive(Debug, Clone)]
pub struct ErrorLogEntry {
    pub timestamp: DateTime<Utc>,
    pub module: String,
    pub message: String,
    pub severity: ErrorSeverity,
    pub context: Option<String>,
}

/// Thread-safe error logging system
pub struct ErrorLogger {
    log: Arc<Mutex<VecDeque<ErrorLogEntry>>>,
    max_log_size: usize,
}

impl ErrorLogger {
    /// Create a new error logger
    pub fn new(max_log_size: usize) -> Self {
        ErrorLogger {
            log: Arc::new(Mutex::new(VecDeque::new())),
            max_log_size,
        }
    }

    /// Log an error with optional context
    pub fn log_error(
        &self, 
        module: &str, 
        message: &str, 
        severity: ErrorSeverity,
        context: Option<String>
    ) {
        let entry = ErrorLogEntry {
            timestamp: Utc::now(),
            module: module.to_string(),
            message: message.to_string(),
            severity,
            context,
        };

        // Safely add entry to log
        if let Ok(mut log) = self.log.lock() {
            // Maintain maximum log size
            if log.len() >= self.max_log_size {
                log.pop_front();
            }
            log.push_back(entry);
        }

        // Log to Godot's console based on severity
        match severity {
            ErrorSeverity::Warning => godot_warn!("[{}] {}", module, message),
            ErrorSeverity::Error => godot_error!("[{}] {}", module, message),
            ErrorSeverity::Critical => godot_error!("CRITICAL [{}] {}", module, message),
        }
    }

    /// Get all error logs
    pub fn get_logs(&self) -> Vec<ErrorLogEntry> {
        if let Ok(log) = self.log.lock() {
            log.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Get logs filtered by severity
    pub fn get_logs_by_severity(&self, severity: ErrorSeverity) -> Vec<ErrorLogEntry> {
        if let Ok(log) = self.log.lock() {
            log.iter()
                .filter(|entry| entry.severity == severity)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Clear all logs
    pub fn clear_logs(&self) {
        if let Ok(mut log) = self.log.lock() {
            log.clear();
        }
    }

    /// Get the number of log entries
    pub fn log_count(&self) -> usize {
        if let Ok(log) = self.log.lock() {
            log.len()
        } else {
            0
        }
    }
}

/// Global error logger for terrain system
/// This can be initialized once and shared across the terrain components
pub struct TerrainErrorLogger {
    logger: Arc<ErrorLogger>,
}

impl TerrainErrorLogger {
    /// Create a new global terrain error logger
    pub fn new() -> Self {
        TerrainErrorLogger {
            logger: Arc::new(ErrorLogger::new(100)), // Default max log size of 100 entries
        }
    }

    /// Get a shareable reference to the logger
    pub fn get_logger(&self) -> Arc<ErrorLogger> {
        Arc::clone(&self.logger)
    }

    /// Convenience method to log terrain-related errors
    pub fn log(&self, module: &str, message: &str, severity: ErrorSeverity) {
        self.logger.log_error(module, message, severity, None);
    }

    /// Log error with additional context
    pub fn log_with_context(
        &self, 
        module: &str, 
        message: &str, 
        severity: ErrorSeverity, 
        context: String
    ) {
        self.logger.log_error(module, message, severity, Some(context));
    }
}

/// Example usage in Godot integration
#[derive(GodotClass)]
#[class(base=Node)]
pub struct TerrainErrorReporter {
    #[base]
    base: Base<Node>,
    error_logger: Arc<ErrorLogger>,
}

#[godot_api]
impl INode for TerrainErrorReporter {
    fn init(base: Base<Node>) -> Self {
        let global_error_logger = TerrainErrorLogger::new();
        
        TerrainErrorReporter {
            base,
            error_logger: global_error_logger.get_logger(),
        }
    }

    fn ready(&mut self) {
        // Example of logging an initialization message
        self.error_logger.log_error(
            "TerrainErrorReporter", 
            "Terrain error reporting system initialized", 
            ErrorSeverity::Warning, 
            None
        );
    }
}

#[godot_api]
impl TerrainErrorReporter {
    /// Fetch and return error logs as a Godot Dictionary
    #[func]
    pub fn get_error_logs(&self) -> Dictionary {
        let mut error_dict = Dictionary::new();
        
        if let Ok(logs) = self.error_logger.log.lock() {
            for (index, entry) in logs.iter().enumerate() {
                let mut log_entry = Dictionary::new();
                log_entry.insert("timestamp", entry.timestamp.to_rfc3339());
                log_entry.insert("module", entry.module.clone());
                log_entry.insert("message", entry.message.clone());
                log_entry.insert("severity", match entry.severity {
                    ErrorSeverity::Warning => "warning",
                    ErrorSeverity::Error => "error",
                    ErrorSeverity::Critical => "critical",
                });
                
                if let Some(context) = &entry.context {
                    log_entry.insert("context", context.clone());
                }
                
                error_dict.insert(index as i64, log_entry);
            }
        }
        
        error_dict
    }

    /// Clear all error logs
    #[func]
    pub fn clear_logs(&mut self) {
        self.error_logger.clear_logs();
    }
}