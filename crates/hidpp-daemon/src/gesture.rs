use std::collections::HashMap;

/// Direction resolved from accumulated XY displacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Result of completing a gesture (button release).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureResult {
    /// Directional swipe detected.
    Direction(GestureDirection),
    /// Quick press+release with no significant movement.
    Tap,
}

/// State of an in-flight gesture for a single button.
struct ActiveGesture {
    /// Accumulated X displacement (positive = right).
    dx: i32,
    /// Accumulated Y displacement (positive = down).
    dy: i32,
    /// Whether the first rawXY event has been discarded.
    /// The MX Master 3S sends a phantom rawXY on button press
    /// (sensor buffer flush) that must be ignored.
    first_xy_discarded: bool,
}

/// Tracks gesture state for all active (held) gesture buttons.
///
/// Created per-connection. Dropped on disconnect, naturally resetting state.
pub struct GestureTracker {
    active: HashMap<u16, ActiveGesture>,
}

impl GestureTracker {
    pub fn new() -> Self {
        Self {
            active: HashMap::new(),
        }
    }

    /// Called when a gesture-configured button is pressed.
    pub fn button_pressed(&mut self, cid: u16) {
        self.active.insert(
            cid,
            ActiveGesture {
                dx: 0,
                dy: 0,
                first_xy_discarded: false,
            },
        );
    }

    /// Called when a gesture-configured button is released.
    /// Returns the gesture result if this button was being tracked.
    pub fn button_released(&mut self, cid: u16, threshold: i32) -> Option<GestureResult> {
        let gesture = self.active.remove(&cid)?;

        let abs_dx = gesture.dx.abs();
        let abs_dy = gesture.dy.abs();
        let magnitude = abs_dx.max(abs_dy);

        if magnitude < threshold {
            return Some(GestureResult::Tap);
        }

        let direction = if abs_dx > abs_dy {
            if gesture.dx > 0 {
                GestureDirection::Right
            } else {
                GestureDirection::Left
            }
        } else if gesture.dy > 0 {
            GestureDirection::Down
        } else {
            GestureDirection::Up
        };

        Some(GestureResult::Direction(direction))
    }

    /// Feed a rawXY event into active gestures. Returns true if consumed.
    ///
    /// rawXY events are not tagged with a CID — they apply to whichever
    /// gesture button(s) are currently held. In practice only one gesture
    /// button is held at a time.
    pub fn feed_raw_xy(&mut self, dx: i16, dy: i16) -> bool {
        if self.active.is_empty() {
            return false;
        }

        for gesture in self.active.values_mut() {
            if !gesture.first_xy_discarded {
                // Discard phantom sensor flush on button press.
                gesture.first_xy_discarded = true;
                continue;
            }
            gesture.dx += i32::from(dx);
            gesture.dy += i32::from(dy);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tap_no_movement() {
        let mut t = GestureTracker::new();
        t.button_pressed(195);
        let r = t.button_released(195, 50);
        assert_eq!(r, Some(GestureResult::Tap));
    }

    #[test]
    fn swipe_right() {
        let mut t = GestureTracker::new();
        t.button_pressed(195);
        t.feed_raw_xy(100, 0); // phantom — discarded
        t.feed_raw_xy(30, 5);
        t.feed_raw_xy(30, 5);
        let r = t.button_released(195, 50);
        assert_eq!(r, Some(GestureResult::Direction(GestureDirection::Right)));
    }

    #[test]
    fn swipe_left() {
        let mut t = GestureTracker::new();
        t.button_pressed(195);
        t.feed_raw_xy(0, 0); // phantom
        t.feed_raw_xy(-40, 5);
        t.feed_raw_xy(-40, 5);
        let r = t.button_released(195, 50);
        assert_eq!(r, Some(GestureResult::Direction(GestureDirection::Left)));
    }

    #[test]
    fn swipe_up() {
        let mut t = GestureTracker::new();
        t.button_pressed(195);
        t.feed_raw_xy(0, 0); // phantom
        t.feed_raw_xy(5, -60);
        let r = t.button_released(195, 50);
        assert_eq!(r, Some(GestureResult::Direction(GestureDirection::Up)));
    }

    #[test]
    fn swipe_down() {
        let mut t = GestureTracker::new();
        t.button_pressed(195);
        t.feed_raw_xy(0, 0); // phantom
        t.feed_raw_xy(-3, 80);
        let r = t.button_released(195, 50);
        assert_eq!(r, Some(GestureResult::Direction(GestureDirection::Down)));
    }

    #[test]
    fn phantom_event_discarded() {
        let mut t = GestureTracker::new();
        t.button_pressed(195);
        t.feed_raw_xy(500, 500); // phantom — big but discarded
        t.feed_raw_xy(0, 0); // real — zero movement
        let r = t.button_released(195, 50);
        assert_eq!(r, Some(GestureResult::Tap));
    }

    #[test]
    fn below_threshold_is_tap() {
        let mut t = GestureTracker::new();
        t.button_pressed(195);
        t.feed_raw_xy(5, 5); // phantom
        t.feed_raw_xy(10, 10); // small movement
        let r = t.button_released(195, 50);
        assert_eq!(r, Some(GestureResult::Tap));
    }

    #[test]
    fn release_without_press_returns_none() {
        let mut t = GestureTracker::new();
        let r = t.button_released(195, 50);
        assert_eq!(r, None);
    }

    #[test]
    fn feed_xy_without_active_gesture() {
        let mut t = GestureTracker::new();
        assert!(!t.feed_raw_xy(10, 10));
    }

    #[test]
    fn threshold_boundary() {
        let mut t = GestureTracker::new();
        t.button_pressed(195);
        t.feed_raw_xy(0, 0); // phantom
        t.feed_raw_xy(49, 0); // exactly at threshold - 1
        let r = t.button_released(195, 50);
        assert_eq!(r, Some(GestureResult::Tap));

        t.button_pressed(195);
        t.feed_raw_xy(0, 0); // phantom
        t.feed_raw_xy(50, 0); // exactly at threshold
        let r = t.button_released(195, 50);
        assert_eq!(r, Some(GestureResult::Direction(GestureDirection::Right)));
    }
}
