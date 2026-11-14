const WHEEL_STEP: f64 = 120.;
const FINGER_STEP: f64 = 40.;

#[derive(Debug, Default)]
pub struct TabScrollState {
    wheel: Accumulator,
    finger: Accumulator,
}

#[derive(Debug, Default)]
struct Accumulator {
    axis: Option<ScrollAxis>,
    amount: f64,
    last: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrollAxis {
    Horizontal,
    Vertical,
}

impl TabScrollState {
    pub fn wheel(&mut self, horizontal_v120: f64, vertical_v120: f64) -> i32 {
        self.finger.reset();
        self.wheel
            .accumulate(horizontal_v120, vertical_v120, WHEEL_STEP, i32::MAX)
    }

    pub fn finger(&mut self, horizontal: f64, vertical: f64) -> i32 {
        self.wheel.reset();
        self.finger.accumulate(horizontal, vertical, FINGER_STEP, 1)
    }

    pub fn reset(&mut self) {
        self.wheel.reset();
        self.finger.reset();
    }
}

impl Accumulator {
    fn accumulate(&mut self, horizontal: f64, vertical: f64, step: f64, max_steps: i32) -> i32 {
        let Some((axis, amount)) = dominant_axis(horizontal, vertical) else {
            self.reset();
            return 0;
        };

        let reversed = (self.last > 0. && amount < 0.) || (self.last < 0. && amount > 0.);
        if self.axis != Some(axis) || reversed {
            self.amount = 0.;
        }

        self.axis = Some(axis);
        self.last = amount;
        self.amount += amount;

        let steps = (self.amount / step).trunc() as i32;
        self.amount %= step;
        steps.clamp(-max_steps, max_steps)
    }

    fn reset(&mut self) {
        self.axis = None;
        self.amount = 0.;
        self.last = 0.;
    }
}

fn dominant_axis(horizontal: f64, vertical: f64) -> Option<(ScrollAxis, f64)> {
    if horizontal == 0. && vertical == 0. {
        None
    } else if horizontal.abs() > vertical.abs() {
        Some((ScrollAxis::Horizontal, horizontal))
    } else {
        Some((ScrollAxis::Vertical, vertical))
    }
}

#[cfg(test)]
mod tests {
    use super::TabScrollState;

    #[test]
    fn wheel_accumulates_v120_and_reversal_resets_remainder() {
        let mut state = TabScrollState::default();
        assert_eq!(state.wheel(0., 60.), 0);
        assert_eq!(state.wheel(0., 60.), 1);
        assert_eq!(state.wheel(0., 60.), 0);
        assert_eq!(state.wheel(0., -60.), 0);
        assert_eq!(state.wheel(0., -60.), -1);
    }

    #[test]
    fn wheel_axis_change_does_not_combine_remainders() {
        let mut state = TabScrollState::default();
        assert_eq!(state.wheel(60., 0.), 0);
        assert_eq!(state.wheel(0., 60.), 0);
        assert_eq!(state.wheel(0., 60.), 1);
    }

    #[test]
    fn finger_requires_threshold_and_emits_one_step_per_event() {
        let mut state = TabScrollState::default();
        assert_eq!(state.finger(0., 20.), 0);
        assert_eq!(state.finger(0., 19.), 0);
        assert_eq!(state.finger(0., 1.), 1);
        assert_eq!(state.finger(0., 400.), 1);
    }

    #[test]
    fn finger_stop_and_reversal_reset_remainder() {
        let mut state = TabScrollState::default();
        assert_eq!(state.finger(0., 30.), 0);
        assert_eq!(state.finger(0., 0.), 0);
        assert_eq!(state.finger(0., 30.), 0);
        assert_eq!(state.finger(0., -1.), 0);
        assert_eq!(state.finger(0., -39.), -1);
    }

    #[test]
    fn changing_source_or_reset_clears_remainder() {
        let mut state = TabScrollState::default();
        assert_eq!(state.wheel(0., 60.), 0);
        assert_eq!(state.finger(0., 20.), 0);
        assert_eq!(state.wheel(0., 60.), 0);
        state.reset();
        assert_eq!(state.wheel(0., 60.), 0);
    }
}
