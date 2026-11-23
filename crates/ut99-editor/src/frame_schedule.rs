use std::time::{Duration, Instant};

const DEFAULT_REFRESH_MILLIHERTZ: u32 = 60_000;
const DEFAULT_FRAME_INTERVAL_NANOS: u64 = 16_666_667;
const MIN_REFRESH_MILLIHERTZ: u32 = 30_000;
const MAX_REFRESH_MILLIHERTZ: u32 = 360_000;

/// Coalesces repaint deadlines and paces continuously requested frames to the active monitor refresh rate.
pub(crate) struct FrameSchedule {
    last_frame: Instant,
    repaint_at: Option<Instant>,
    not_before: Option<Instant>,
    refresh_rate_millihertz: u32,
    min_frame_interval: Duration,
}
impl FrameSchedule {
    pub fn new(now: Instant) -> Self {
        Self {
            last_frame: now,
            repaint_at: None,
            not_before: None,
            refresh_rate_millihertz: DEFAULT_REFRESH_MILLIHERTZ,
            min_frame_interval: Duration::from_nanos(DEFAULT_FRAME_INTERVAL_NANOS),
        }
    }
    pub fn set_refresh_rate_millihertz(&mut self, millihertz: Option<u32>) {
        match millihertz {
            Some(mhz) => {
                let clamped = mhz.clamp(MIN_REFRESH_MILLIHERTZ, MAX_REFRESH_MILLIHERTZ);
                self.refresh_rate_millihertz = clamped;
                self.min_frame_interval =
                    Duration::from_nanos((1_000_000_000_000u64 / clamped as u64).max(1));
            }
            None => {
                self.refresh_rate_millihertz = DEFAULT_REFRESH_MILLIHERTZ;
                self.min_frame_interval = Duration::from_nanos(DEFAULT_FRAME_INTERVAL_NANOS);
            }
        }
    }
    pub fn refresh_rate_hz(&self) -> f32 {
        self.refresh_rate_millihertz as f32 / 1000.0
    }
    #[cfg(test)]
    pub fn min_frame_interval(&self) -> Duration {
        self.min_frame_interval
    }
    pub fn request(&mut self, deadline: Instant) {
        self.repaint_at = Some(self.repaint_at.map_or(deadline, |old| old.min(deadline)));
    }
    pub fn frame_started(&mut self, now: Instant) {
        self.last_frame = now;
        self.repaint_at = None;
    }
    pub fn defer_until(&mut self, deadline: Instant) { self.not_before = Some(deadline); }
    pub fn resume(&mut self) { self.not_before = None; }
    pub fn deadline(&self) -> Option<Instant> {
        self.repaint_at.map(|deadline| {
            deadline
                .max(self.last_frame + self.min_frame_interval)
                .max(self.not_before.unwrap_or(deadline))
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_delay_survives_input_and_redraw_requests() {
        let now = Instant::now();
        let mut schedule = FrameSchedule::new(now);
        let retry = now + Duration::from_secs(2);
        schedule.defer_until(retry);
        schedule.request(now);
        schedule.frame_started(now);
        schedule.request(now);
        assert_eq!(schedule.deadline(), Some(retry));
        schedule.resume();
        assert!(schedule.deadline().unwrap() < retry);
    }
    #[test]
    fn idle_has_no_frame_deadline() {
        let schedule = FrameSchedule::new(Instant::now());
        assert_eq!(schedule.deadline(), None);
    }
    #[test]
    fn requests_coalesce_without_postponing_earlier_work() {
        let now = Instant::now();
        let mut schedule = FrameSchedule::new(now);
        schedule.request(now + Duration::from_secs(5));
        schedule.request(now + Duration::from_secs(1));
        schedule.request(now + Duration::from_secs(3));
        assert_eq!(schedule.deadline(), Some(now + Duration::from_secs(1)));
    }
    #[test]
    fn immediate_animation_is_throttled_and_completed_frame_clears_requests() {
        let now = Instant::now();
        let mut schedule = FrameSchedule::new(now);
        schedule.request(now);
        assert_eq!(schedule.deadline(), Some(now + Duration::from_nanos(16_666_667)));
        schedule.frame_started(now + Duration::from_millis(17));
        assert_eq!(schedule.deadline(), None);
    }
    #[test]
    fn adapts_frame_interval_to_monitor_refresh_rate_60_120_240_and_clamps_outliers() {
        let now = Instant::now();
        let mut schedule = FrameSchedule::new(now);
        assert!((schedule.refresh_rate_hz() - 60.0).abs() < 1e-3);
        assert_eq!(schedule.min_frame_interval(), Duration::from_nanos(16_666_667));

        schedule.set_refresh_rate_millihertz(Some(120_000));
        assert!((schedule.refresh_rate_hz() - 120.0).abs() < 1e-3);
        assert_eq!(schedule.min_frame_interval(), Duration::from_nanos(8_333_333));
        schedule.request(now);
        assert_eq!(schedule.deadline(), Some(now + Duration::from_nanos(8_333_333)));

        schedule.set_refresh_rate_millihertz(Some(240_000));
        assert!((schedule.refresh_rate_hz() - 240.0).abs() < 1e-3);
        assert_eq!(schedule.min_frame_interval(), Duration::from_nanos(4_166_666));
        schedule.frame_started(now);
        schedule.request(now);
        assert_eq!(schedule.deadline(), Some(now + Duration::from_nanos(4_166_666)));

        schedule.set_refresh_rate_millihertz(Some(1_000_000));
        assert!((schedule.refresh_rate_hz() - 360.0).abs() < 1e-3);
        schedule.set_refresh_rate_millihertz(Some(10_000));
        assert!((schedule.refresh_rate_hz() - 30.0).abs() < 1e-3);
        schedule.set_refresh_rate_millihertz(None);
        assert!((schedule.refresh_rate_hz() - 60.0).abs() < 1e-3);
    }
    #[test]
    fn idle_remains_none_at_high_refresh_rates() {
        let now = Instant::now();
        let mut schedule = FrameSchedule::new(now);
        schedule.set_refresh_rate_millihertz(Some(240_000));
        assert_eq!(schedule.deadline(), None);
        schedule.request(now);
        schedule.frame_started(now + Duration::from_millis(5));
        assert_eq!(schedule.deadline(), None);
    }
}

#[cfg(all(test, target_os = "windows"))]
mod windows_tests {
    use super::*;
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::platform::windows::EventLoopBuilderExtWindows;
    use winit::window::WindowId;

    #[test]
    #[ignore = "runs a native Windows event loop; run serially"]
    fn waiting_loop_sleeps_and_accepts_background_wakeup() {
        struct Probe { until: Instant, visits: usize, woke: bool }
        impl ApplicationHandler<()> for Probe {
            fn resumed(&mut self, _: &ActiveEventLoop) {}
            fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
            fn user_event(&mut self, _: &ActiveEventLoop, _: ()) { self.woke = true; }
            fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
                self.visits += 1;
                if Instant::now() >= self.until { event_loop.exit(); }
                else { event_loop.set_control_flow(ControlFlow::WaitUntil(self.until)); }
            }
        }
        let event_loop = EventLoop::<()>::with_user_event().with_any_thread(true).build().unwrap();
        let proxy = event_loop.create_proxy();
        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(75));
            proxy.send_event(()).unwrap();
        });
        let mut probe = Probe { until: Instant::now() + Duration::from_millis(250), visits: 0, woke: false };
        event_loop.run_app(&mut probe).unwrap();
        sender.join().unwrap();
        eprintln!("Wait-loop visits in 250 ms: {}", probe.visits);
        assert!(probe.woke, "background event must wake a waiting loop");
        assert!(probe.visits < 12, "idle loop must not poll every frame");
    }
}
