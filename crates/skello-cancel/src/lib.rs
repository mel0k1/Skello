//! A shared cancel flag for long flash operations. The UI asks the
//! running operation to stop; every protocol loop checks the flag
//! between chunks and aborts with its `Cancelled` error. One global
//! flag fits the app: the operations are mutually exclusive (a single
//! busy gate owns the USB port).

use std::sync::atomic::{AtomicBool, Ordering};

static CANCELLED: AtomicBool = AtomicBool::new(false);

/// Clears the flag; every long operation calls this when it starts.
pub fn reset() {
    CANCELLED.store(false, Ordering::SeqCst);
}

/// Sets the flag; the running operation stops at the next chunk.
pub fn request() {
    CANCELLED.store(true, Ordering::SeqCst);
}

/// True once the stop button was pressed for the running operation.
pub fn is_requested() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    #[test]
    fn resets_after_request() {
        super::reset();
        assert!(!super::is_requested());
        super::request();
        assert!(super::is_requested());
        super::reset();
        assert!(!super::is_requested());
    }
}
