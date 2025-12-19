// use crate::trackpad;
use cidre::cg;
use core_graphics;
use objc2;
use objc2_app_kit;
// use objc2_application_services;
use crate::utils::{max, max_x, max_y, min, min_x, min_y};

pub const MAXIMUM_MOMENTUM_SPEED: cg::Float = 9000.0;
pub const TRACKPAD_VELOCITY_GAIN: cg::Float = 0.95;
pub const GLIDE_DECAY_PER_SECOND: cg::Float = 6.5;
pub const MINIMUM_GLIDE_VELOCITY: cg::Float = 220.0;
pub const GLIDE_STOP_SPEED_FACTOR: cg::Float = 0.45;

enum VelocitySource {
    Pointer,
    Trackpad,
}

struct State {
    position: cg::Point,
    previous_position: cg::Point,
    last_input_delta: cg::Vector,
    velocity: cg::Vector,
    is_gliding: bool,
    velocity_source: VelocitySource,
}

pub struct Engine {
    state: State,
    last_physical_mouse_position: cg::Point,
    desktop_bounds: cg::Rect,
}

impl Engine {
    pub fn new() -> Self {
        Engine {
            state: State {
                position: cg::Point { x: 0.0, y: 0.0 },
                previous_position: cg::Point { x: 0.0, y: 0.0 },
                last_input_delta: cg::Vector { dx: 0.0, dy: 0.0 },
                velocity: cg::Vector { dx: 0.0, dy: 0.0 },
                is_gliding: false,
                velocity_source: VelocitySource::Pointer,
            },
            last_physical_mouse_position: cg::Point { x: 0.0, y: 0.0 },
            desktop_bounds: cg::Rect::null(),
        }
    }

    pub fn set_gliding(&mut self, value: bool) {
        self.state.is_gliding = value;
    }

    pub fn begin_touch(&mut self, position: cg::Point) {
        self.state.position = position;
        self.state.previous_position = position;
        self.state.last_input_delta = cg::Vector { dx: 0.0, dy: 0.0 };
        self.state.velocity = cg::Vector { dx: 0.0, dy: 0.0 };
        self.set_gliding(false);
    }

    pub fn handle_touch(
        &mut self,
        physical_position: cg::Point,
        delta_time: cg::Float,
        normalized_trackpad_velocity: Option<cg::Vector>,
    ) {
        let delta_pos = cg::Point {
            x: physical_position.x - self.last_physical_mouse_position.x,
            y: physical_position.y - self.last_physical_mouse_position.y,
        };
        self.last_physical_mouse_position = physical_position;
        self.state.previous_position = self.state.position;

        let pointer_velocity = cg::Vector {
            dx: delta_pos.x / delta_time,
            dy: delta_pos.y / delta_time,
        };

        let mut velocity = pointer_velocity;
        let mut source: VelocitySource = VelocitySource::Pointer;
        if let Some(trackpad_velocity) =
            self.trackpad_velocity_in_pixels(&normalized_trackpad_velocity)
        {
            if Self::magnitude(&trackpad_velocity) > Self::magnitude(&pointer_velocity) {
                velocity = trackpad_velocity;
                source = VelocitySource::Trackpad;
            }
        }
        self.state.velocity = velocity;
        self.state.velocity_source = source;
        self.state.position.x += delta_pos.x;
        self.state.position.y += delta_pos.y;
        self.state.last_input_delta = cg::Vector {
            dx: delta_pos.x,
            dy: delta_pos.y,
        };

        self.clamp_position_to_desktop();

        if self.state.is_gliding {
            self.set_gliding(false);
        }
    }

    pub fn apply_momentum(&mut self, delta_time: cg::Float) {
        let decay_factor = max(0.0, 1.0 - GLIDE_DECAY_PER_SECOND * delta_time);
        self.state.velocity.dx *= decay_factor;
        self.state.velocity.dy *= decay_factor;

        let momentum_delta = cg::Vector {
            dx: self.state.velocity.dx * delta_time,
            dy: self.state.velocity.dy * delta_time,
        };

        self.state.previous_position = self.state.position;
        self.state.position.x += momentum_delta.dx;
        self.state.position.y += momentum_delta.dy;
        self.state.last_input_delta = momentum_delta;

        self.clamp_position_to_desktop();
        self.sync_to_virtual_position();

        let speed = Self::magnitude(&self.state.velocity);
        if speed < MINIMUM_GLIDE_VELOCITY * GLIDE_STOP_SPEED_FACTOR {
            self.set_gliding(false);
            self.state.velocity = cg::Vector { dx: 0.0, dy: 0.0 };
            self.sync_to_virtual_position();
        }
    }

    fn magnitude(vector: &cg::Vector) -> cg::Float {
        (vector.dx * vector.dx + vector.dy * vector.dy).sqrt()
    }

    pub fn update_desktop_bounds(&mut self, bounds: cg::Rect) {
        self.desktop_bounds = bounds;
        self.clamp_position_to_desktop();
    }

    pub fn sync_to_virtual_position(&mut self) {
        let target = self.state.position;
        let mtm = objc2::MainThreadMarker::new().expect("must be on the main thread");
        if let Some(screen) = objc2_app_kit::NSScreen::mainScreen(mtm) {
            let display_id = unsafe { core_graphics::display::CGMainDisplayID() };
            let local_x = target.x - screen.frame().min().x;
            let local_y_from_bottom = target.y - screen.frame().min().y;
            let local_y = screen.frame().size.height - local_y_from_bottom;
            let _error = unsafe {
                core_graphics::display::CGDisplayMoveCursorToPoint(
                    display_id,
                    core_graphics::display::CGPoint {
                        x: local_x,
                        y: local_y,
                    },
                )
            };
        } else {
            return;
        }
    }

    pub fn sync_state(&mut self, physical_position: cg::Point) {
        self.state.position = physical_position;
        self.state.previous_position = physical_position;
        self.state.last_input_delta = cg::Vector { dx: 0.0, dy: 0.0 };
        self.last_physical_mouse_position = physical_position;
    }

    pub fn clamp_position_to_desktop(&mut self) {
        if self.desktop_bounds == cg::Rect::null() {
            return;
        }
        self.state.position.x = min(
            max(self.state.position.x, min_x(&self.desktop_bounds)),
            max_x(&self.desktop_bounds),
        );
        self.state.position.y = min(
            max(self.state.position.y, min_y(&self.desktop_bounds)),
            max_y(&self.desktop_bounds),
        );
    }

    fn trackpad_velocity_in_pixels(
        &mut self,
        normalized_velocity: &Option<cg::Vector>,
    ) -> Option<cg::Vector> {
        if self.desktop_bounds == cg::Rect::null() {
            return None;
        }
        if let Some(normalized_velocity) = normalized_velocity {
            let scaled = cg::Vector {
                dx: normalized_velocity.dx
                    * self.desktop_bounds.size.width
                    * TRACKPAD_VELOCITY_GAIN,
                dy: normalized_velocity.dy
                    * self.desktop_bounds.size.height
                    * TRACKPAD_VELOCITY_GAIN,
            };
            return Some(Self::clamped_velocity(&scaled, MAXIMUM_MOMENTUM_SPEED));
        } else {
            return None;
        }
    }

    fn clamped_velocity(vector: &cg::Vector, max_magnitude: cg::Float) -> cg::Vector {
        let magnitude = Self::magnitude(vector);
        if magnitude > max_magnitude && magnitude > 0.0 {
            let scale = max_magnitude / magnitude;
            return cg::Vector {
                dx: vector.dx * scale,
                dy: vector.dy * scale,
            };
        } else {
            return *vector;
        }
    }
}
