use embassy_time::Instant;
pub struct SimpleRngU64 {
    seed: u64,
}

impl SimpleRngU64 {
    pub fn new() -> Self {
        // Use the current time as initial seed
        let now = Instant::now();
        let seed = now.as_micros();
        Self { seed }
    }

    // Seed update
    pub fn next_u64(&mut self) -> u64 {
        const A: u64 = 1664525;
        const C: u64 = 1013904223;
        self.seed = self.seed.wrapping_mul(A).wrapping_add(C);
        self.seed
    }

    // Linear congruential generator implementation
    pub fn generate_from_range(&mut self, from: u64, to: u64) -> u64 {
        if from >= to {
            return from;
        }
        from + (self.next_u64() % (to - from + 1))
    }
}
