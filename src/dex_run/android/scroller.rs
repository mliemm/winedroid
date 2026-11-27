use super::*;

const SCROLLER: &str = "Landroid/widget/Scroller;";
const DEFAULT_DURATION: i32 = 250;
const NB_SAMPLES: usize = 100;
const INFLEXION: f32 = 0.35;
const START_TENSION: f32 = 0.5;
const END_TENSION: f32 = 1.0;
const SCROLL_FRICTION: f32 = 0.015;
const GRAVITY_EARTH: f32 = 9.806_65;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScrollerMode {
    Scroll,
    Fling,
}

// cite: frameworks-base Scroller.java#mMode, mStartX, mFinalX, mCurrX, mStartTime, mDuration, mFinished, mFlywheel, mVelocity, mCurrVelocity, mDistance, mFlingFriction, mDeceleration, mPpi, mPhysicalCoeff
pub(in crate::dex_run) struct ScrollerState {
    interpolator: DexValue,
    mode: ScrollerMode,
    start: (i32, i32),
    end: (i32, i32),
    min: (i32, i32),
    max: (i32, i32),
    current: (i32, i32),
    delta: (f32, f32),
    start_time: i64,
    duration: i32,
    duration_reciprocal: f32,
    finished: bool,
    flywheel: bool,
    velocity: f32,
    current_velocity: f32,
    distance: i32,
    fling_friction: f32,
    deceleration: f32,
    ppi: f32,
    physical_coefficient: f32,
}

// cite: frameworks-base Scroller.java#computeDeceleration
fn compute_deceleration(ppi: f32, friction: f32) -> f32 {
    GRAVITY_EARTH * 39.37 * ppi * friction
}

// cite: frameworks-base Scroller.java#SPLINE_POSITION, SPLINE_TIME, P1, P2
fn spline_position() -> &'static [f32; NB_SAMPLES + 1] {
    static SPLINE: std::sync::OnceLock<[f32; NB_SAMPLES + 1]> = std::sync::OnceLock::new();
    SPLINE.get_or_init(|| {
        let p1 = START_TENSION * INFLEXION;
        let p2 = 1.0 - END_TENSION * (1.0 - INFLEXION);
        let mut position = [0.0f32; NB_SAMPLES + 1];
        let mut x_min = 0.0f32;
        for (index, slot) in position.iter_mut().take(NB_SAMPLES).enumerate() {
            let alpha = index as f32 / NB_SAMPLES as f32;
            let mut x_max = 1.0f32;
            let (x, coef) = loop {
                let x = x_min + (x_max - x_min) / 2.0;
                let coef = 3.0 * x * (1.0 - x);
                let tx = coef * ((1.0 - x) * p1 + x * p2) + x * x * x;
                if (tx - alpha).abs() < 1e-5 {
                    break (x, coef);
                }
                if tx > alpha {
                    x_max = x;
                } else {
                    x_min = x;
                }
            };
            *slot = coef * ((1.0 - x) * START_TENSION + x) + x * x * x;
        }
        position[NB_SAMPLES] = 1.0;
        position
    })
}

// cite: frameworks-base Scroller.java#ViscousFluidInterpolator, viscousFluid, getInterpolation
fn viscous_fluid(x: f32) -> f32 {
    let x = x * 8.0;
    if x < 1.0 {
        x - (1.0 - (-x).exp())
    } else {
        let start = 0.367_879_44_f32;
        let x = 1.0 - (1.0 - x).exp();
        start + x * (1.0 - start)
    }
}

fn viscous_fluid_interpolation(input: f32) -> f32 {
    let normalize = 1.0 / viscous_fluid(1.0);
    let offset = 1.0 - normalize * viscous_fluid(1.0);
    let interpolated = normalize * viscous_fluid(input);
    if interpolated > 0.0 {
        interpolated + offset
    } else {
        interpolated
    }
}

fn java_round(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}

fn float_argument(value: &DexValue) -> Option<f32> {
    match value {
        DexValue::Float(value) => Some(*value),
        DexValue::Int(bits) => Some(f32::from_bits(*bits as u32)),
        _ => None,
    }
}

impl ScrollerState {
    // cite: frameworks-base Scroller.java#getSplineDeceleration, getSplineFlingDuration, getSplineFlingDistance, DECELERATION_RATE
    fn spline_deceleration(&self, velocity: f32) -> f64 {
        (f64::from(INFLEXION) * f64::from(velocity.abs())
            / f64::from(self.fling_friction * self.physical_coefficient))
        .ln()
    }

    fn deceleration_rate() -> f64 {
        0.78f64.ln() / 0.9f64.ln()
    }

    fn spline_fling_duration(&self, velocity: f32) -> i32 {
        let decel_minus_one = Self::deceleration_rate() - 1.0;
        (1000.0 * (self.spline_deceleration(velocity) / decel_minus_one).exp()) as i32
    }

    fn spline_fling_distance(&self, velocity: f32) -> f64 {
        let rate = Self::deceleration_rate();
        f64::from(self.fling_friction * self.physical_coefficient)
            * (rate / (rate - 1.0) * self.spline_deceleration(velocity)).exp()
    }

    fn time_passed(&self, now: i64) -> i32 {
        (now - self.start_time) as i32
    }

    // cite: frameworks-base Scroller.java#getCurrVelocity
    fn current_velocity(&self, now: i64) -> f32 {
        if self.mode == ScrollerMode::Fling {
            self.current_velocity
        } else {
            self.velocity - self.deceleration * self.time_passed(now) as f32 / 2000.0
        }
    }

    // cite: frameworks-base Scroller.java#fling
    fn fling(
        &mut self,
        now: i64,
        start: (i32, i32),
        velocity: (i32, i32),
        min: (i32, i32),
        max: (i32, i32),
    ) {
        let (mut velocity_x, mut velocity_y) = (velocity.0 as f32, velocity.1 as f32);
        if self.flywheel && !self.finished {
            let old_velocity = self.current_velocity(now);
            let dx = (self.end.0 - self.start.0) as f32;
            let dy = (self.end.1 - self.start.1) as f32;
            let hypotenuse = dx.hypot(dy);
            let old_x = dx / hypotenuse * old_velocity;
            let old_y = dy / hypotenuse * old_velocity;
            if velocity_x.signum() == old_x.signum() && velocity_y.signum() == old_y.signum() {
                velocity_x += old_x;
                velocity_y += old_y;
            }
            velocity_x = velocity_x.trunc();
            velocity_y = velocity_y.trunc();
        }

        self.mode = ScrollerMode::Fling;
        self.finished = false;
        let speed = velocity_x.hypot(velocity_y);
        self.velocity = speed;
        self.duration = self.spline_fling_duration(speed);
        self.start_time = now;
        self.start = start;
        let coefficient_x = if speed == 0.0 {
            1.0
        } else {
            velocity_x / speed
        };
        let coefficient_y = if speed == 0.0 {
            1.0
        } else {
            velocity_y / speed
        };
        let total_distance = self.spline_fling_distance(speed);
        self.distance = (total_distance * f64::from(speed.signum())) as i32;
        self.min = min;
        self.max = max;
        let end_x = start.0 + (total_distance * f64::from(coefficient_x) + 0.5).floor() as i32;
        let end_y = start.1 + (total_distance * f64::from(coefficient_y) + 0.5).floor() as i32;
        self.end = (end_x.min(max.0).max(min.0), end_y.min(max.1).max(min.1));
    }
}

impl DexProgram {
    // cite: frameworks-base Scroller.java#Scroller, startScroll, computeScrollOffset, abortAnimation, forceFinished, isFinished, extendDuration, timePassed, setFinalX, setFinalY, setFriction, isScrollingInDirection
    pub(in crate::dex_run) fn scroller_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.class_descriptor != SCROLLER || self.guest_overrides(method, invocation) {
            return None;
        }
        let invocation = invocation?;
        let arguments = invocation.arguments.as_slice();
        let scroller = arguments.first()?;
        if !dex_value_is_object_like(scroller) {
            return None;
        }
        let key = map_identity_key(scroller);
        let now = self.runtime.borrow().main_thread_uptime_millis;
        let descriptor = method.proto.descriptor();

        if method.name == "<init>" {
            let (interpolator, flywheel) = match (descriptor.as_str(), arguments) {
                ("(Landroid/content/Context;)V", [_, _]) => (DexValue::Null, true),
                (
                    "(Landroid/content/Context;Landroid/view/animation/Interpolator;)V",
                    [_, _, interpolator],
                ) => (interpolator.clone(), true),
                (
                    "(Landroid/content/Context;Landroid/view/animation/Interpolator;Z)V",
                    [_, _, interpolator, DexValue::Int(flywheel)],
                ) => (interpolator.clone(), *flywheel != 0),
                _ => return None,
            };
            let ppi = BOOTSTRAP_DISPLAY_DENSITY * 160.0;
            let state = ScrollerState {
                interpolator: if dex_value_is_object_like(&interpolator) {
                    interpolator
                } else {
                    DexValue::Null
                },
                mode: ScrollerMode::Scroll,
                start: (0, 0),
                end: (0, 0),
                min: (0, 0),
                max: (0, 0),
                current: (0, 0),
                delta: (0.0, 0.0),
                start_time: 0,
                duration: 0,
                duration_reciprocal: 0.0,
                finished: true,
                flywheel,
                velocity: 0.0,
                current_velocity: 0.0,
                distance: 0,
                fling_friction: SCROLL_FRICTION,
                deceleration: compute_deceleration(ppi, SCROLL_FRICTION),
                ppi,
                physical_coefficient: compute_deceleration(ppi, 0.84),
            };
            self.runtime.borrow_mut().scrollers.insert(key, state);
            return Some(DexExecutionResult::Void);
        }

        if method.name == "computeScrollOffset" && descriptor == "()Z" {
            return self.scroller_compute_scroll_offset(&key, now);
        }

        let mut runtime = self.runtime.borrow_mut();
        let state = runtime.scrollers.get_mut(&key)?;
        let int = |index: usize| match arguments.get(index)? {
            DexValue::Int(value) => Some(*value),
            _ => None,
        };
        Some(match (method.name.as_str(), descriptor.as_str()) {
            ("isFinished", "()Z") => DexExecutionResult::Int(i32::from(state.finished)),
            ("forceFinished", "(Z)V") => {
                state.finished = int(1)? != 0;
                DexExecutionResult::Void
            }
            ("getDuration", "()I") => DexExecutionResult::Int(state.duration),
            ("getCurrX", "()I") => DexExecutionResult::Int(state.current.0),
            ("getCurrY", "()I") => DexExecutionResult::Int(state.current.1),
            ("getStartX", "()I") => DexExecutionResult::Int(state.start.0),
            ("getStartY", "()I") => DexExecutionResult::Int(state.start.1),
            ("getFinalX", "()I") => DexExecutionResult::Int(state.end.0),
            ("getFinalY", "()I") => DexExecutionResult::Int(state.end.1),
            ("getCurrVelocity", "()F") => {
                DexExecutionResult::Value(DexValue::Float(state.current_velocity(now)))
            }
            ("timePassed", "()I") => DexExecutionResult::Int(state.time_passed(now)),
            ("startScroll", "(IIII)V") | ("startScroll", "(IIIII)V") => {
                let duration = if arguments.len() == 6 {
                    int(5)?
                } else {
                    DEFAULT_DURATION
                };
                let (start_x, start_y, dx, dy) = (int(1)?, int(2)?, int(3)?, int(4)?);
                state.mode = ScrollerMode::Scroll;
                state.finished = false;
                state.duration = duration;
                state.start_time = now;
                state.start = (start_x, start_y);
                state.end = (start_x.wrapping_add(dx), start_y.wrapping_add(dy));
                state.delta = (dx as f32, dy as f32);
                state.duration_reciprocal = 1.0 / duration as f32;
                DexExecutionResult::Void
            }
            ("fling", "(IIIIIIII)V") => {
                state.fling(
                    now,
                    (int(1)?, int(2)?),
                    (int(3)?, int(4)?),
                    (int(5)?, int(7)?),
                    (int(6)?, int(8)?),
                );
                DexExecutionResult::Void
            }
            ("abortAnimation", "()V") => {
                state.current = state.end;
                state.finished = true;
                DexExecutionResult::Void
            }
            ("extendDuration", "(I)V") => {
                state.duration = state.time_passed(now) + int(1)?;
                state.duration_reciprocal = 1.0 / state.duration as f32;
                state.finished = false;
                DexExecutionResult::Void
            }
            ("setFinalX", "(I)V") => {
                state.end.0 = int(1)?;
                state.delta.0 = (state.end.0 - state.start.0) as f32;
                state.finished = false;
                DexExecutionResult::Void
            }
            ("setFinalY", "(I)V") => {
                state.end.1 = int(1)?;
                state.delta.1 = (state.end.1 - state.start.1) as f32;
                state.finished = false;
                DexExecutionResult::Void
            }
            ("setFriction", "(F)V") => {
                let friction = float_argument(arguments.get(1)?)?;
                state.deceleration = compute_deceleration(state.ppi, friction);
                state.fling_friction = friction;
                DexExecutionResult::Void
            }
            ("isScrollingInDirection", "(FF)Z") => {
                let (x, y) = (
                    float_argument(arguments.get(1)?)?,
                    float_argument(arguments.get(2)?)?,
                );
                let scrolling = !state.finished
                    && x.signum() == ((state.end.0 - state.start.0) as f32).signum()
                    && y.signum() == ((state.end.1 - state.start.1) as f32).signum();
                DexExecutionResult::Int(i32::from(scrolling))
            }
            _ => return None,
        })
    }

    // cite: frameworks-base Scroller.java#computeScrollOffset
    fn scroller_compute_scroll_offset(&self, key: &str, now: i64) -> Option<DexExecutionResult> {
        let (mode, finished, time_passed, duration, interpolator, reciprocal) = {
            let runtime = self.runtime.borrow();
            let state = runtime.scrollers.get(key)?;
            (
                state.mode,
                state.finished,
                state.time_passed(now),
                state.duration,
                state.interpolator.clone(),
                state.duration_reciprocal,
            )
        };
        if finished {
            return Some(DexExecutionResult::Int(0));
        }
        if time_passed >= duration {
            let mut runtime = self.runtime.borrow_mut();
            let state = runtime.scrollers.get_mut(key)?;
            state.current = state.end;
            state.finished = true;
            return Some(DexExecutionResult::Int(1));
        }

        match mode {
            ScrollerMode::Scroll => {
                let input = time_passed as f32 * reciprocal;
                let progress = if matches!(interpolator, DexValue::Null) {
                    viscous_fluid_interpolation(input)
                } else {
                    self.scroller_guest_interpolation(&interpolator, input)?
                };
                let mut runtime = self.runtime.borrow_mut();
                let state = runtime.scrollers.get_mut(key)?;
                state.current = (
                    state.start.0 + java_round(progress * state.delta.0),
                    state.start.1 + java_round(progress * state.delta.1),
                );
            }
            ScrollerMode::Fling => {
                let mut runtime = self.runtime.borrow_mut();
                let state = runtime.scrollers.get_mut(key)?;
                let t = time_passed as f32 / duration as f32;
                let index = (NB_SAMPLES as f32 * t) as usize;
                let (mut distance_coefficient, mut velocity_coefficient) = (1.0f32, 0.0f32);
                if index < NB_SAMPLES {
                    let spline = spline_position();
                    let t_inf = index as f32 / NB_SAMPLES as f32;
                    let t_sup = (index + 1) as f32 / NB_SAMPLES as f32;
                    velocity_coefficient = (spline[index + 1] - spline[index]) / (t_sup - t_inf);
                    distance_coefficient = spline[index] + (t - t_inf) * velocity_coefficient;
                }
                state.current_velocity =
                    velocity_coefficient * state.distance as f32 / duration as f32 * 1000.0;
                let x = state.start.0
                    + java_round(distance_coefficient * (state.end.0 - state.start.0) as f32);
                let y = state.start.1
                    + java_round(distance_coefficient * (state.end.1 - state.start.1) as f32);
                state.current = (
                    x.min(state.max.0).max(state.min.0),
                    y.min(state.max.1).max(state.min.1),
                );
                if state.current == state.end {
                    state.finished = true;
                }
            }
        }

        Some(DexExecutionResult::Int(1))
    }

    // cite: frameworks-base TimeInterpolator.java#getInterpolation
    fn scroller_guest_interpolation(&self, interpolator: &DexValue, input: f32) -> Option<f32> {
        let class_descriptor = value_class_descriptor(interpolator)?;
        let method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: "getInterpolation".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "FF".to_owned(),
                return_type: "F".to_owned(),
                parameters: vec!["F".to_owned()],
            },
        };
        let arguments = [interpolator.clone(), DexValue::Float(input)];
        let result = match self
            .resolve_exact_method_in_class_hierarchy(class_descriptor, &method)
            .ok()?
        {
            Some(resolved) => self
                .execute_method(None, &resolved, Some(&arguments), 0)
                .ok()?,
            None => self.invoke_builtin_method(
                &method,
                Some(&DexInvocation {
                    kind: crate::dex_interpreter::DexInvokeKind::Interface,
                    range: false,
                    method_index: 0,
                    argument_registers: Default::default(),
                    arguments: arguments.into_iter().collect(),
                }),
            )?,
        };
        match result {
            DexExecutionResult::Value(value) => float_argument(&value),
            DexExecutionResult::Int(bits) => Some(f32::from_bits(bits as u32)),
            _ => None,
        }
    }
}
