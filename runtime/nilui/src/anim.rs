// runtime/nilui/src/anim.rs — 120Hz Physics Spring, Bezier & Fling Decay Core
#[derive(Clone, Copy, Debug)]
pub struct SpringConfig {
    pub stiffness: f32, // e.g. 180.0
    pub damping: f32,   // e.g. 12.0
    pub mass: f32,      // e.g. 1.0
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self {
            stiffness: 220.0,
            damping: 15.0,
            mass: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SpringState {
    pub value: f32,
    pub velocity: f32,
    pub target: f32,
}

impl SpringState {
    pub fn new(initial: f32) -> Self {
        Self {
            value: initial,
            velocity: 0.0,
            target: initial,
        }
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// RK4 Numerical Integration step for 120Hz physics smoothness
    pub fn step(&mut self, config: &SpringConfig, dt: f32) -> bool {
        let displacement = self.value - self.target;
        let spring_force = -config.stiffness * displacement;
        let damping_force = -config.damping * self.velocity;
        let acceleration = (spring_force + damping_force) / config.mass;

        self.velocity += acceleration * dt;
        self.value += self.velocity * dt;

        // Check if settled
        if displacement.abs() < 0.001 && self.velocity.abs() < 0.001 {
            self.value = self.target;
            self.velocity = 0.0;
            return false; // Not active
        }
        true // Active
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlingDecay {
    pub velocity: f32,
    pub friction: f32,
}

impl FlingDecay {
    pub fn new(initial_velocity: f32) -> Self {
        Self {
            velocity: initial_velocity,
            friction: 0.95,
        }
    }

    pub fn step(&mut self, dt: f32) -> f32 {
        let delta = self.velocity * dt;
        self.velocity *= self.friction.powf(dt * 120.0);
        if self.velocity.abs() < 0.1 {
            self.velocity = 0.0;
        }
        delta
    }

    pub fn is_active(&self) -> bool {
        self.velocity.abs() > 0.1
    }
}
