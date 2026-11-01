use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

const WORKER_QUEUE_POLL: Duration = Duration::from_millis(250);
const STOP_JOIN: Duration = Duration::from_secs(5);
const READY_WAIT_MAX: f64 = 15.0;
const AUTOSCALE_TICK: Duration = Duration::from_secs(5);

pub const NO_AUDIO_TRACEBACK: &str = "Renderer returned no audio after restart";

#[derive(Debug, Clone, PartialEq)]
pub enum RenderFailure {
    NoAudio,
    Engine(String),
}

impl std::fmt::Display for RenderFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderFailure::NoAudio => f.write_str("Renderer returned no audio"),
            RenderFailure::Engine(e) => f.write_str(e),
        }
    }
}

pub trait LaneEngine<J> {
    fn render_once(&mut self, job: &J) -> Result<Vec<i16>, RenderFailure>;

    fn restart(&mut self);
}

pub trait EngineFactory<J>: Send + Sync + 'static {
    fn start(&self) -> Result<Box<dyn LaneEngine<J>>, String>;
}

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub rom: String,
    pub workers: usize,
    pub min_active: usize,
    pub render_timeout: f64,
    pub autoscale: bool,
    pub scale_down_idle: f64,
    pub idle_suspend: f64,
    pub stack_size: usize,

    pub wedged_exit_after: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct Rendered {
    pub samples: Vec<i16>,
    pub elapsed_ms: f64,
}

#[derive(Debug, Clone)]
pub struct LaneHealth {
    pub name: String,
    pub rom: String,
    pub running: bool,
    pub alive: bool,
    pub ready: bool,
    pub paused: bool,
    pub active_job_count: u32,
    pub idle_seconds: Option<f64>,
    pub last_render_ms: Option<f64>,
    pub resume_count: u64,
    pub restart_count: u64,
    pub render_retry_count: u64,
    pub last_recovery_reason: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PoolHealth {
    pub autoscale: bool,
    pub scale_down_idle: f64,
    pub rom: String,
    pub min_active: usize,
    pub running: usize,
    pub configured: usize,
    pub workers: Vec<LaneHealth>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Clone)]
struct WorkerVars {
    paused: bool,
    active_job_count: u32,
    last_activity_at: Instant,
    last_render_ms: Option<f64>,
    resume_count: u64,
    restart_count: u64,
    render_retry_count: u64,
    last_recovery_reason: Option<String>,
    last_error: Option<String>,
}

enum Msg<J> {
    Job { job_id: u64, payload: Arc<J> },
    Stop,
}

#[derive(Default)]
struct ExitFlag {
    exited: Mutex<bool>,
    cv: Condvar,
}

impl ExitFlag {
    fn set(&self) {
        *lock(&self.exited) = true;
        self.cv.notify_all();
    }
    fn get(&self) -> bool {
        *lock(&self.exited)
    }
    fn wait(&self, timeout: Duration) -> bool {
        let g = lock(&self.exited);
        let (g, _) = self
            .cv
            .wait_timeout_while(g, timeout, |e| !*e)
            .unwrap_or_else(|e| e.into_inner());
        *g
    }
}

struct Process<J> {
    id: u64,
    inbox: mpsc::Sender<Msg<J>>,
    exit: Arc<ExitFlag>,
    stop_requested: Arc<AtomicBool>,
}

struct LaneState<J> {
    process: Option<Process<J>>,
    next_process_id: u64,
    pending: HashMap<u64, mpsc::Sender<Result<Rendered, String>>>,
    ready: bool,
    startup_failed: bool,
    last_error: Option<String>,
    paused: bool,
    active_job_count: u32,
    last_activity_at: Option<Instant>,
    last_render_ms: Option<f64>,
    resume_count: u64,
    worker_restart_count: u64,
    process_restart_count: u64,
    render_retry_count: u64,
    last_recovery_reason: Option<String>,
    reserved_count: u32,
}

struct Settings<J> {
    factory: Arc<dyn EngineFactory<J>>,
    idle_suspend: f64,
    stack_size: usize,
    wedged_exit_after: Option<Duration>,
    next_job_id: AtomicU64,
}

pub struct Lane<J> {
    name: String,
    rom: String,

    lifecycle: Mutex<()>,
    state: Mutex<LaneState<J>>,
    ready_cv: Condvar,
    settings: Arc<Settings<J>>,
}

impl<J: Send + Sync + 'static> Lane<J> {
    fn new(name: String, rom: String, settings: Arc<Settings<J>>) -> Self {
        Self {
            name,
            rom,
            lifecycle: Mutex::new(()),
            state: Mutex::new(LaneState {
                process: None,
                next_process_id: 0,
                pending: HashMap::new(),
                ready: false,
                startup_failed: false,
                last_error: None,
                paused: false,
                active_job_count: 0,
                last_activity_at: None,
                last_render_ms: None,
                resume_count: 0,
                worker_restart_count: 0,
                process_restart_count: 0,
                render_retry_count: 0,
                last_recovery_reason: None,
                reserved_count: 0,
            }),
            ready_cv: Condvar::new(),
            settings,
        }
    }

    pub fn start(self: &Arc<Self>) {
        let _life = lock(&self.lifecycle);
        self.start_locked();
    }

    fn start_locked(self: &Arc<Self>) {
        if self.is_alive() {
            return;
        }
        let id = {
            let mut st = lock(&self.state);
            st.ready = false;
            st.startup_failed = false;
            st.last_error = None;
            st.paused = false;
            st.active_job_count = 0;
            st.last_activity_at = None;
            st.last_render_ms = None;
            st.resume_count = 0;
            st.worker_restart_count = 0;
            st.render_retry_count = 0;
            st.last_recovery_reason = None;
            st.next_process_id += 1;
            st.next_process_id
        };
        let (inbox, rx) = mpsc::channel();
        let exit = Arc::new(ExitFlag::default());
        let stop_requested = Arc::new(AtomicBool::new(false));

        lock(&self.state).process = Some(Process {
            id,
            inbox,
            exit: exit.clone(),
            stop_requested: stop_requested.clone(),
        });
        let lane = self.clone();
        let spawned = std::thread::Builder::new()
            .name(self.name.clone())
            .stack_size(self.settings.stack_size)
            .spawn({
                let exit = exit.clone();
                move || worker_loop(lane, id, rx, exit, stop_requested)
            });
        if let Err(e) = spawned {
            exit.set();
            let mut st = lock(&self.state);
            st.startup_failed = true;
            st.last_error = Some(format!("engine error: lane thread: {e}"));
            st.ready = true;
            self.ready_cv.notify_all();
        }
    }

    pub fn stop(self: &Arc<Self>) {
        let _life = lock(&self.lifecycle);
        self.stop_locked();
    }

    fn stop_locked(self: &Arc<Self>) {
        let handles = {
            let st = lock(&self.state);
            st.process.as_ref().map(|p| {
                p.stop_requested.store(true, Ordering::Relaxed);
                let _ = p.inbox.send(Msg::Stop);
                p.exit.clone()
            })
        };
        if let Some(exit) = handles {
            if !exit.wait(STOP_JOIN) {
                eprintln!("[ttsmodachi-worker:{}] killed while rendering", self.name);
                if let Some(limit) = self.settings.wedged_exit_after {
                    let name = self.name.clone();
                    std::thread::spawn(move || {
                        if !exit.wait(limit) {
                            eprintln!("[ttsmodachi-worker:{name}] render wedged, exiting");
                            std::process::exit(70);
                        }
                    });
                }
            }
        }
        let pending: Vec<_> = {
            let mut st = lock(&self.state);
            st.ready = false;
            st.process = None;
            st.paused = false;
            st.active_job_count = 0;
            st.reserved_count = 0;
            st.pending.drain().map(|(_, tx)| tx).collect()
        };
        for tx in pending {
            let _ = tx.send(Err(format!("Renderer worker {} stopped", self.name)));
        }
    }

    pub fn restart(self: &Arc<Self>) {
        let _life = lock(&self.lifecycle);
        lock(&self.state).process_restart_count += 1;
        self.stop_locked();
        self.start_locked();
    }

    fn wait_ready(&self, timeout: f64) -> bool {
        let g = lock(&self.state);
        let (g, _) = self
            .ready_cv
            .wait_timeout_while(g, Duration::from_secs_f64(timeout.max(0.0)), |st| !st.ready)
            .unwrap_or_else(|e| e.into_inner());
        g.ready
    }

    pub fn render(self: &Arc<Self>, payload: Arc<J>, timeout: f64) -> Result<Rendered, String> {
        if !self.is_alive() {
            self.start();
        }
        let wait = timeout.min(READY_WAIT_MAX);
        if !self.wait_ready(wait) {
            return Err(format!("Renderer worker {} is not ready", self.name));
        }
        if self.startup_failed() {
            self.restart();
            if !self.wait_ready(wait) {
                return Err(format!(
                    "Renderer worker {} is not ready after restart",
                    self.name
                ));
            }
            if self.startup_failed() {
                let err = lock(&self.state).last_error.clone();
                return Err(format!(
                    "Renderer worker {} failed startup: {}",
                    self.name,
                    err.as_deref().unwrap_or("None"),
                ));
            }
        }
        let job_id = self.settings.next_job_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        {
            let mut st = lock(&self.state);
            let Some(inbox) = st.process.as_ref().map(|p| p.inbox.clone()) else {
                return Err(format!(
                    "Renderer worker {} has no command queue",
                    self.name
                ));
            };
            st.pending.insert(job_id, tx);
            let _ = inbox.send(Msg::Job { job_id, payload });
        }
        match rx.recv_timeout(Duration::from_secs_f64(timeout.max(0.0))) {
            Ok(result) => result,
            Err(_) => {
                lock(&self.state).pending.remove(&job_id);
                self.restart();
                Err(format!("Renderer worker {} timed out", self.name))
            }
        }
    }

    pub fn pending_count(&self) -> usize {
        lock(&self.state).pending.len()
    }

    pub fn load_count(&self) -> usize {
        let st = lock(&self.state);
        st.pending.len() + st.reserved_count as usize + st.active_job_count as usize
    }

    fn reserve(&self) {
        lock(&self.state).reserved_count += 1;
    }

    fn release_reservation(&self) {
        let mut st = lock(&self.state);
        st.reserved_count = st.reserved_count.saturating_sub(1);
    }

    pub fn is_alive(&self) -> bool {
        lock(&self.state)
            .process
            .as_ref()
            .is_some_and(|p| !p.exit.get())
    }

    fn startup_failed(&self) -> bool {
        lock(&self.state).startup_failed
    }

    fn paused(&self) -> bool {
        lock(&self.state).paused
    }

    fn idle_seconds(&self) -> Option<f64> {
        lock(&self.state)
            .last_activity_at
            .map(|t| t.elapsed().as_secs_f64())
    }

    fn health(&self) -> LaneHealth {
        let st = lock(&self.state);
        let running = st.process.is_some();
        LaneHealth {
            name: self.name.clone(),
            rom: self.rom.clone(),
            running,
            alive: st.process.as_ref().is_some_and(|p| !p.exit.get()),
            ready: st.ready,
            paused: st.paused,
            active_job_count: st.active_job_count,
            idle_seconds: st.last_activity_at.map(|t| t.elapsed().as_secs_f64()),
            last_render_ms: st.last_render_ms,
            resume_count: st.resume_count,
            restart_count: st.process_restart_count + st.worker_restart_count,
            render_retry_count: st.render_retry_count,
            last_recovery_reason: st.last_recovery_reason.clone(),
            last_error: st.last_error.clone(),
        }
    }

    fn with_current<R>(&self, process: u64, f: impl FnOnce(&mut LaneState<J>) -> R) -> Option<R> {
        let mut st = lock(&self.state);
        if st.process.as_ref().is_some_and(|p| p.id == process) {
            Some(f(&mut st))
        } else {
            None
        }
    }

    fn publish(&self, process: u64, vars: &WorkerVars) {
        self.with_current(process, |st| apply_state(st, vars));
    }

    fn on_ready(&self, process: u64, vars: &WorkerVars) {
        self.with_current(process, |st| {
            st.startup_failed = false;
            apply_state(st, vars);
            st.ready = true;
        });
        self.ready_cv.notify_all();
    }

    fn on_startup_error(self: &Arc<Self>, process: u64, error: String) {
        let restart = self.with_current(process, |st| {
            st.startup_failed = true;
            st.last_error = Some(error);
            if st.process_restart_count < 2 {
                st.ready = false;
                true
            } else {
                st.ready = true;
                false
            }
        });
        self.ready_cv.notify_all();
        if restart == Some(true) {
            let lane = self.clone();
            std::thread::spawn(move || lane.restart());
        }
    }

    fn on_result(
        &self,
        process: u64,
        job_id: u64,
        result: Result<Rendered, String>,
        vars: &WorkerVars,
    ) {
        let reply = self
            .with_current(process, |st| {
                apply_state(st, vars);
                st.pending.remove(&job_id)
            })
            .flatten();
        if let Some(tx) = reply {
            let _ = tx.send(result);
        }
    }

    fn on_exit(&self, process: u64, stop_requested: bool) {
        if stop_requested {
            return;
        }
        let pending: Vec<_> = self
            .with_current(process, |st| st.pending.drain().map(|(_, tx)| tx).collect())
            .unwrap_or_default();
        for tx in pending {
            let _ = tx.send(Err(format!("Renderer worker {} exited", self.name)));
        }
    }
}

fn apply_state<J>(st: &mut LaneState<J>, vars: &WorkerVars) {
    st.paused = vars.paused;
    st.active_job_count = vars.active_job_count;
    st.last_activity_at = Some(vars.last_activity_at);
    st.last_render_ms = vars.last_render_ms;
    st.resume_count = vars.resume_count;
    st.worker_restart_count = vars.restart_count;
    st.render_retry_count = vars.render_retry_count;
    st.last_recovery_reason = vars.last_recovery_reason.clone();
    st.last_error = vars.last_error.clone();
}

struct ExitGuard<J: Send + Sync + 'static> {
    lane: Arc<Lane<J>>,
    process: u64,
    exit: Arc<ExitFlag>,
    stop_requested: Arc<AtomicBool>,
}

impl<J: Send + Sync + 'static> Drop for ExitGuard<J> {
    fn drop(&mut self) {
        self.exit.set();
        self.lane
            .on_exit(self.process, self.stop_requested.load(Ordering::Relaxed));
    }
}

fn worker_loop<J: Send + Sync + 'static>(
    lane: Arc<Lane<J>>,
    process: u64,
    rx: mpsc::Receiver<Msg<J>>,
    exit: Arc<ExitFlag>,
    stop_requested: Arc<AtomicBool>,
) {
    let _guard = ExitGuard {
        lane: lane.clone(),
        process,
        exit,
        stop_requested: stop_requested.clone(),
    };
    let mut vars = WorkerVars {
        paused: false,
        active_job_count: 0,
        last_activity_at: Instant::now(),
        last_render_ms: None,
        resume_count: 0,
        restart_count: 0,
        render_retry_count: 0,
        last_recovery_reason: None,
        last_error: None,
    };
    let mut engine = match lane.settings.factory.start() {
        Ok(engine) => {
            vars.last_activity_at = Instant::now();
            lane.on_ready(process, &vars);
            Some(engine)
        }
        Err(error) => {
            lane.on_startup_error(process, format!("engine error: {error}"));
            None
        }
    };
    loop {
        match rx.recv_timeout(WORKER_QUEUE_POLL) {
            Err(mpsc::RecvTimeoutError::Timeout) => {
                maybe_suspend(&lane, process, &mut vars);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) | Ok(Msg::Stop) => break,
            Ok(Msg::Job { job_id, payload }) => {
                run_job(&lane, process, &mut engine, &mut vars, job_id, &payload);
            }
        }
    }
}

fn maybe_suspend<J: Send + Sync + 'static>(
    lane: &Arc<Lane<J>>,
    process: u64,
    vars: &mut WorkerVars,
) {
    let idle_suspend = lane.settings.idle_suspend;
    if vars.active_job_count > 0 || idle_suspend <= 0.0 || vars.paused {
        return;
    }
    if vars.last_activity_at.elapsed().as_secs_f64() < idle_suspend {
        return;
    }
    vars.paused = true;
    lane.publish(process, vars);
}

fn run_job<J: Send + Sync + 'static>(
    lane: &Arc<Lane<J>>,
    process: u64,
    engine: &mut Option<Box<dyn LaneEngine<J>>>,
    vars: &mut WorkerVars,
    job_id: u64,
    payload: &J,
) {
    let started = Instant::now();
    vars.active_job_count += 1;
    vars.last_activity_at = Instant::now();
    let result = resume_emulator(lane, process, engine, vars)
        .and_then(|()| render_with_recovery(lane, process, engine, vars, payload));
    match result {
        Ok(samples) => {
            vars.last_error = None;
            let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
            lane.on_result(
                process,
                job_id,
                Ok(Rendered {
                    samples,
                    elapsed_ms,
                }),
                vars,
            );
        }
        Err(traceback) => {
            vars.last_error = Some(traceback.clone());
            lane.on_result(process, job_id, Err(traceback), vars);
        }
    }
    vars.active_job_count = vars.active_job_count.saturating_sub(1);
    vars.last_render_ms = Some(started.elapsed().as_secs_f64() * 1000.0);
    vars.last_activity_at = Instant::now();
    lane.publish(process, vars);
}

fn resume_emulator<J: Send + Sync + 'static>(
    lane: &Arc<Lane<J>>,
    process: u64,
    engine: &mut Option<Box<dyn LaneEngine<J>>>,
    vars: &mut WorkerVars,
) -> Result<(), String> {
    if engine.is_none() {
        let started = lane
            .settings
            .factory
            .start()
            .map_err(|e| format!("engine error: {e}"))?;
        *engine = Some(started);
        vars.paused = false;
        vars.last_activity_at = Instant::now();
        vars.last_error = None;
        return Ok(());
    }
    if !vars.paused {
        return Ok(());
    }
    vars.paused = false;
    vars.resume_count += 1;
    vars.last_activity_at = Instant::now();
    vars.last_error = None;
    lane.publish(process, vars);
    Ok(())
}

fn render_with_recovery<J: Send + Sync + 'static>(
    lane: &Arc<Lane<J>>,
    process: u64,
    engine: &mut Option<Box<dyn LaneEngine<J>>>,
    vars: &mut WorkerVars,
    payload: &J,
) -> Result<Vec<i16>, String> {
    let engine = engine.as_mut().expect("resume_emulator started it");
    let first = match engine.render_once(payload) {
        Ok(samples) => return Ok(samples),
        Err(first) => first,
    };
    let reason = format!("render failed before retry: {first}");
    vars.last_recovery_reason = Some(reason.clone());
    vars.render_retry_count += 1;
    eprintln!("[ttsmodachi-worker:{}] {reason}", lane.name);

    vars.restart_count += 1;
    vars.last_recovery_reason = Some(reason);
    engine.restart();
    vars.paused = false;
    vars.last_activity_at = Instant::now();
    vars.last_error = None;
    lane.publish(process, vars);
    engine
        .render_once(payload)
        .map_err(|second| match (&first, &second) {
            (RenderFailure::NoAudio, RenderFailure::NoAudio) => NO_AUDIO_TRACEBACK.to_string(),
            _ => second.to_string(),
        })
}

fn is_worker_recoverable_error(message: &str) -> bool {
    let message = message.to_lowercase();
    [
        "failed startup",
        "is not ready",
        "timed out",
        "renderer returned no audio",
        "citra",
        "emulator",
        "engine error",
        "engine panic",
    ]
    .iter()
    .any(|phrase| message.contains(phrase))
}

pub struct RendererPool<J> {
    rom: String,
    lanes: Vec<Arc<Lane<J>>>,
    min_active: usize,
    render_timeout: f64,
    autoscale: bool,
    scale_down_idle: f64,

    next_index: Mutex<usize>,
    autoscale_stop: Arc<(Mutex<bool>, Condvar)>,
}

impl<J: Send + Sync + 'static> RendererPool<J> {
    pub fn new(cfg: PoolConfig, factory: Arc<dyn EngineFactory<J>>) -> Arc<Self> {
        let settings = Arc::new(Settings {
            factory,
            idle_suspend: cfg.idle_suspend,
            stack_size: cfg.stack_size,
            wedged_exit_after: cfg.wedged_exit_after,
            next_job_id: AtomicU64::new(1),
        });
        let lanes = (0..cfg.workers)
            .map(|i| {
                let name = format!("{}-{}", cfg.rom, i + 1);
                Arc::new(Lane::new(name, cfg.rom.clone(), settings.clone()))
            })
            .collect();
        Arc::new(Self {
            rom: cfg.rom,
            lanes,
            min_active: cfg.min_active,
            render_timeout: cfg.render_timeout,
            autoscale: cfg.autoscale,
            scale_down_idle: cfg.scale_down_idle,
            next_index: Mutex::new(0),
            autoscale_stop: Arc::new((Mutex::new(false), Condvar::new())),
        })
    }

    pub fn start(self: &Arc<Self>) {
        let _pool = lock(&self.next_index);
        let started = if self.autoscale {
            self.min_active_count()
        } else {
            self.lanes.len()
        };
        for lane in &self.lanes[..started] {
            lane.start();
        }
        if self.autoscale {
            let pool = Arc::downgrade(self);
            let stop = self.autoscale_stop.clone();
            std::thread::spawn(move || loop {
                let g = lock(&stop.0);
                let (g, _) = stop
                    .1
                    .wait_timeout_while(g, AUTOSCALE_TICK, |s| !*s)
                    .unwrap_or_else(|e| e.into_inner());
                if *g {
                    return;
                }
                drop(g);
                match pool.upgrade() {
                    Some(pool) => pool.scale_down_idle_lanes(),
                    None => return,
                }
            });
        }
    }

    pub fn stop(&self) {
        *lock(&self.autoscale_stop.0) = true;
        self.autoscale_stop.1.notify_all();
        for lane in &self.lanes {
            lane.stop();
        }
    }

    pub fn render(&self, payload: Arc<J>) -> Result<Rendered, String> {
        if self.lanes.is_empty() {
            return Err(format!(
                "No renderer worker configured for ROM {}",
                self.rom
            ));
        }
        let mut attempted = HashSet::new();
        let mut errors: Vec<String> = Vec::new();
        for _ in 0..self.lanes.len() {
            let chosen = {
                let mut next_index = lock(&self.next_index);
                self.ensure_capacity();
                self.choose_lane(&mut next_index, &attempted)
            };
            let Some(index) = chosen else { break };
            attempted.insert(index);
            let lane = &self.lanes[index];
            lane.reserve();
            let result = lane.render(payload.clone(), self.render_timeout);
            let error = match result {
                Ok(rendered) => {
                    lane.release_reservation();
                    return Ok(rendered);
                }
                Err(error) => error,
            };
            errors.push(format!("{}: {error}", lane.name));
            if lane.startup_failed() || is_worker_recoverable_error(&error) {
                lane.restart();
            }
            lane.release_reservation();
        }
        let detail = if errors.is_empty() {
            "no workers were available".to_string()
        } else {
            errors[errors.len().saturating_sub(3)..].join(" | ")
        };
        Err(format!(
            "All renderer workers failed for ROM {}: {detail}",
            self.rom
        ))
    }

    fn min_active_count(&self) -> usize {
        self.min_active.min(self.lanes.len()).max(1)
    }

    fn ensure_capacity(&self) {
        if !self.autoscale {
            for lane in &self.lanes {
                lane.start();
            }
            return;
        }
        let mut running: Vec<&Arc<Lane<J>>> = self.lanes.iter().filter(|l| l.is_alive()).collect();
        let min_active = self.min_active_count();
        while running.len() < min_active {
            let Some(lane) = self.lanes.iter().find(|l| !l.is_alive()) else {
                break;
            };
            lane.start();
            running.push(lane);
        }
        if running.is_empty() {
            return;
        }
        let running_load: usize = running.iter().map(|l| l.load_count()).sum();
        if running_load < running.len() {
            return;
        }
        if let Some(lane) = self.lanes.iter().find(|l| !l.is_alive()) {
            lane.start();
        }
    }

    fn scale_down_idle_lanes(&self) {
        let _pool = lock(&self.next_index);
        let min_active = self.min_active_count();
        let running: Vec<&Arc<Lane<J>>> = self.lanes.iter().filter(|l| l.is_alive()).collect();
        if running.len() <= min_active {
            return;
        }
        let mut idle_candidates: Vec<(f64, bool, &Arc<Lane<J>>)> = running
            .into_iter()
            .filter_map(|lane| {
                let idle = lane.idle_seconds()?;
                if lane.load_count() > 0 || idle < self.scale_down_idle {
                    return None;
                }
                Some((idle, lane.paused(), lane))
            })
            .collect();

        idle_candidates.sort_by(|a, b| {
            (!a.1)
                .cmp(&!b.1)
                .then(b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal))
        });
        for (_, _, lane) in idle_candidates {
            if self.lanes.iter().filter(|l| l.is_alive()).count() <= min_active {
                break;
            }
            lane.stop();
        }
    }

    fn choose_lane(&self, next_index: &mut usize, attempted: &HashSet<usize>) -> Option<usize> {
        let count = self.lanes.len();
        let mut available: Vec<usize> = (0..count)
            .filter(|i| {
                !attempted.contains(i)
                    && !self.lanes[*i].startup_failed()
                    && self.lanes[*i].is_alive()
            })
            .collect();
        if available.is_empty() {
            available = (0..count).filter(|i| !attempted.contains(i)).collect();
        }
        let pending: Vec<(usize, usize)> = available
            .iter()
            .map(|&i| (i, self.lanes[i].pending_count()))
            .collect();
        let lowest = pending.iter().map(|&(_, p)| p).min()?;
        let mut candidates: Vec<usize> = pending
            .iter()
            .filter(|&&(_, p)| p == lowest)
            .map(|&(i, _)| i)
            .collect();
        let warm: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|&i| !self.lanes[i].paused())
            .collect();
        if !warm.is_empty() {
            candidates = warm;
        }
        let start = *next_index % count;
        for offset in 0..count {
            let index = (start + offset) % count;
            if candidates.contains(&index) {
                *next_index = (index + 1) % count;
                return Some(index);
            }
        }
        None
    }

    pub fn health(&self) -> PoolHealth {
        let workers: Vec<LaneHealth> = self.lanes.iter().map(|l| l.health()).collect();
        PoolHealth {
            autoscale: self.autoscale,
            scale_down_idle: self.scale_down_idle,
            rom: self.rom.clone(),
            min_active: self.min_active,
            running: workers.iter().filter(|w| w.alive).count(),
            configured: workers.len(),
            workers,
        }
    }
}

pub struct ExecutorSlots {
    state: Mutex<(u64, u64)>,
    cv: Condvar,
    permits: u64,
}

pub struct ExecutorSlot<'a>(&'a ExecutorSlots);

impl ExecutorSlots {
    pub fn new(permits: usize) -> Self {
        Self {
            state: Mutex::new((0, 0)),
            cv: Condvar::new(),
            permits: permits.max(1) as u64,
        }
    }

    pub fn acquire(&self) -> ExecutorSlot<'_> {
        let mut g = lock(&self.state);
        let ticket = g.0;
        g.0 += 1;
        let _g = self
            .cv
            .wait_while(g, |s| ticket >= s.1 + self.permits)
            .unwrap_or_else(|e| e.into_inner());
        ExecutorSlot(self)
    }
}

impl Drop for ExecutorSlot<'_> {
    fn drop(&mut self) {
        lock(&self.0.state).1 += 1;
        self.0.cv.notify_all();
    }
}
