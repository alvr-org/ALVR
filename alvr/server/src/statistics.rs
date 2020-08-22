

use std::time::Instant;

struct Statistics {
    last_update_instant: Instant,
}

impl Statistics {
    pub fn new() -> Self {
        Self {
            last_update_instant: Instant::now(),
            
        }
    }
}