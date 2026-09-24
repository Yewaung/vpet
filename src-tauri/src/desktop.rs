//! Moves the pet window around the desktop. Works in physical pixels.

use vpet_core::rng::Rng;
use vpet_core::{Locomotion, Movement};

const GRAVITY: f64 = 2600.0;
const ARRIVE_EPS: f64 = 2.0;
const CURSOR_STOP_DIST: f64 = 70.0;
/// How fast a flyer climbs or descends (px/s).
const CLIMB_SPEED: f64 = 90.0;

/// Usable area of the monitor the pet is on (taskbar excluded).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

pub struct Motion {
    /// Window top-left.
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    vy: f64,
    target: Option<f64>,
    pause: f64,
    pub facing: i8,
    pub moving: bool,
    /// How this body gets around; set by the runtime every step.
    pub loco: Locomotion,
    /// Sleeping or sitting: flyers land.
    pub resting: bool,
    /// Visual lift above the ground line (hops, swim waves, flight).
    lift: f64,
    /// Flight altitude above the floor, and where it's heading.
    alt: f64,
    alt_target: f64,
    /// Gait phase for hops, waves and hovering.
    phase: f64,
    rng: Rng,
}

impl Motion {
    pub fn new(x: f64, y: f64, w: f64, h: f64, seed: u64) -> Self {
        Self {
            x,
            y,
            w,
            h,
            vy: 0.0,
            target: None,
            pause: 0.0,
            facing: 1,
            moving: false,
            loco: Locomotion::Walk,
            resting: false,
            lift: 0.0,
            alt: 0.0,
            alt_target: 0.0,
            phase: 0.0,
            rng: Rng::new(seed),
        }
    }

    /// Where the window's top edge would be with no lift.
    fn base(&self) -> f64 {
        self.y + self.lift
    }

    /// Highest a flyer goes above the taskbar.
    fn max_alt(&self, b: &Bounds) -> f64 {
        ((b.bottom - b.top) * 0.45).min(self.h * 2.5).max(0.0)
    }

    fn floor_y(&self, b: &Bounds) -> f64 {
        b.bottom - self.h
    }

    /// Dropped and falling. Flyers never fall: they just hover.
    pub fn falling(&self, b: &Bounds) -> bool {
        self.loco != Locomotion::Fly && self.base() < self.floor_y(b) - 0.5
    }

    /// Picks up / puts down the window directly (dragging).
    pub fn place(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
        self.lift = 0.0;
        self.alt = 0.0;
        self.vy = 0.0;
        self.target = None;
        self.moving = false;
    }

    /// True when the next frame needs smooth updates.
    pub fn active(&self, b: &Bounds, movement: Movement) -> bool {
        let flying = self.loco == Locomotion::Fly && (self.alt - self.wanted_alt()).abs() > 1.0;
        self.falling(b) || flying || self.moving || self.lift > 0.5 || (movement != Movement::Stay && self.pause <= 0.0)
    }

    fn wanted_alt(&self) -> f64 {
        if self.resting { 0.0 } else { self.alt_target }
    }

    /// Advances one frame. `speed` is px/s; `cursor` is the cursor position.
    pub fn step(&mut self, dt: f64, movement: Movement, speed: f64, cursor: Option<(f64, f64)>, b: &Bounds) {
        let floor = self.floor_y(b);
        let max_x = (b.right - self.w).max(b.left);
        let mut base = self.base();

        if self.loco == Locomotion::Fly {
            // Dropped mid-air: hover right there.
            if base < floor - 0.5 {
                self.alt = (self.alt + floor - base).min(self.max_alt(b).max(floor - base));
                self.alt_target = self.alt;
            }
            // Flying to the cursor means flying up to its height too.
            if movement == Movement::ApproachCursor {
                if let Some((_, cy)) = cursor {
                    self.alt_target = (floor + self.h * 0.5 - cy).clamp(0.0, self.max_alt(b));
                }
            }
            let want = self.wanted_alt();
            self.alt += (want - self.alt).clamp(-CLIMB_SPEED * dt, CLIMB_SPEED * dt);
            self.phase += dt * 3.0;
        } else if base < floor {
            self.vy += GRAVITY * dt;
            self.y = (base + self.vy * dt).min(floor);
            self.lift = 0.0;
            self.moving = false;
            return;
        }
        base = floor;
        self.vy = 0.0;
        self.x = self.x.clamp(b.left, max_x);

        let (target, speed) = match movement {
            Movement::Stay => {
                self.target = None;
                (None, speed)
            }
            Movement::ApproachCursor => {
                let t = cursor.and_then(|(cx, _)| {
                    let goal = (cx - self.w / 2.0).clamp(b.left, max_x);
                    ((goal - self.x).abs() > CURSOR_STOP_DIST).then_some(goal)
                });
                (t, speed * 1.2)
            }
            Movement::Wander | Movement::Explore => {
                let (reach, pace) = if movement == Movement::Explore { (600.0, 1.4) } else { (300.0, 1.0) };
                if self.pause > 0.0 {
                    self.pause -= dt;
                    (None, speed)
                } else {
                    if self.target.is_none() {
                        if self.loco == Locomotion::Fly {
                            self.alt_target = self.rng.range(0.2, 1.0) * self.max_alt(b);
                        }
                        let dir = if self.rng.next_f64() < 0.5 { -1.0 } else { 1.0 };
                        let dist = self.rng.range(reach * 0.3, reach);
                        let mut goal = self.x + dir * dist;
                        if goal < b.left || goal > max_x {
                            goal = self.x - dir * dist;
                        }
                        self.target = Some(goal.clamp(b.left, max_x));
                    }
                    (self.target, speed * pace)
                }
            }
        };

        self.moving = false;
        if let Some(t) = target {
            let dx = t - self.x;
            if dx.abs() <= ARRIVE_EPS {
                self.x = t;
                if matches!(movement, Movement::Wander | Movement::Explore) {
                    self.target = None;
                    self.pause = self.rng.range(1.0, 4.0);
                }
            } else {
                self.facing = if dx > 0.0 { 1 } else { -1 };
                self.x += dx.signum() * (speed * dt).min(dx.abs());
                self.moving = true;
            }
        }
        self.lift = self.gait(dt, speed);
        self.y = base - self.lift;
    }

    /// Vertical offset for this body's way of moving.
    fn gait(&mut self, dt: f64, speed: f64) -> f64 {
        match self.loco {
            Locomotion::Fly => {
                let bob = if self.alt > 2.0 { self.phase.sin() * 3.0 } else { 0.0 };
                self.alt + bob
            }
            Locomotion::Hop => {
                // Each hop covers about half a body length.
                if self.moving || self.phase.sin().abs() > 0.05 {
                    let hop_len = (self.w * 0.5).max(20.0);
                    self.phase = (self.phase + dt * speed.max(40.0) / hop_len * std::f64::consts::PI) % std::f64::consts::TAU;
                } else {
                    self.phase = 0.0;
                }
                self.phase.sin().abs() * self.h * 0.14
            }
            Locomotion::Swim => {
                self.phase += dt * if self.moving { 5.0 } else { 1.5 };
                (1.0 + self.phase.sin()) * self.h * if self.moving { 0.04 } else { 0.012 }
            }
            Locomotion::Walk | Locomotion::Run | Locomotion::Crawl => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: Bounds = Bounds {
        left: 0.0,
        top: 0.0,
        right: 1920.0,
        bottom: 1040.0,
    };

    #[test]
    fn dropped_pet_falls_to_the_floor() {
        let mut m = Motion::new(500.0, 100.0, 200.0, 200.0, 1);
        for _ in 0..200 {
            m.step(1.0 / 60.0, Movement::Stay, 50.0, None, &B);
        }
        assert_eq!(m.y, 840.0);
        assert!(!m.falling(&B));
    }

    #[test]
    fn wander_stays_on_screen() {
        let mut m = Motion::new(0.0, 840.0, 200.0, 200.0, 3);
        for _ in 0..20_000 {
            m.step(1.0 / 30.0, Movement::Explore, 80.0, None, &B);
            assert!(m.x >= 0.0 && m.x <= 1720.0);
        }
    }

    #[test]
    fn flyers_take_off_and_land_to_rest() {
        let mut m = Motion::new(500.0, 840.0, 200.0, 200.0, 5);
        m.loco = Locomotion::Fly;
        for _ in 0..600 {
            m.step(1.0 / 30.0, Movement::Wander, 80.0, None, &B);
        }
        assert!(m.y < 840.0 - 10.0, "it flies above the taskbar");
        assert!(!m.falling(&B));
        m.resting = true;
        for _ in 0..600 {
            m.step(1.0 / 30.0, Movement::Stay, 80.0, None, &B);
        }
        assert!((m.y - 840.0).abs() < 0.5, "and lands to sleep");
    }

    #[test]
    fn dropped_flyer_hovers_instead_of_falling() {
        let mut m = Motion::new(500.0, 400.0, 200.0, 200.0, 5);
        m.loco = Locomotion::Fly;
        m.step(1.0 / 30.0, Movement::Stay, 80.0, None, &B);
        assert!(m.y < 500.0);
        assert!(!m.falling(&B));
    }

    #[test]
    fn hoppers_leave_the_ground_while_travelling() {
        let mut m = Motion::new(0.0, 840.0, 200.0, 200.0, 3);
        m.loco = Locomotion::Hop;
        let mut highest: f64 = 840.0;
        for _ in 0..300 {
            m.step(1.0 / 30.0, Movement::Explore, 80.0, None, &B);
            highest = highest.min(m.y);
            assert!(!m.falling(&B), "a hop is not a fall");
        }
        assert!(highest < 835.0);
    }

    #[test]
    fn approaches_cursor_then_stops() {
        let mut m = Motion::new(0.0, 840.0, 200.0, 200.0, 1);
        for _ in 0..2000 {
            m.step(1.0 / 30.0, Movement::ApproachCursor, 80.0, Some((1000.0, 500.0)), &B);
        }
        assert!((m.x + 100.0 - 1000.0).abs() <= CURSOR_STOP_DIST + 1.0);
        assert!(!m.moving);
        assert_eq!(m.facing, 1);
    }
}
