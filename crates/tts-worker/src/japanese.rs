use super::{pool, UsJob};
use tts_core::jp;
pub struct Factory {
    tables: std::sync::Arc<tts_rom::japanese::JpImage>,
}
impl Factory {
    pub fn new(image: tts_rom::japanese::JpImage) -> Self {
        Self {
            tables: std::sync::Arc::new(image),
        }
    }
}
struct Lane {
    tables: std::sync::Arc<tts_rom::japanese::JpImage>,
    mem: Box<[u8; jp::ENGINE_SIZE]>,
    text: [u8; 0x400],
    pcm: Vec<i16>,
}
impl pool::EngineFactory<UsJob> for Factory {
    fn start(&self) -> Result<Box<dyn pool::LaneEngine<UsJob>>, String> {
        Ok(Box::new(Lane {
            tables: self.tables.clone(),
            mem: Box::new([0; jp::ENGINE_SIZE]),
            text: [0; 0x400],
            pcm: vec![0; 4_000_000],
        }))
    }
}
impl pool::LaneEngine<UsJob> for Lane {
    fn render_once(&mut self, job: &UsJob) -> Result<Vec<i16>, pool::RenderFailure> {
        let (text, params) = match job {
            UsJob::Jp { text, params } => (text, params),
            UsJob::NoAudio => return Err(pool::RenderFailure::NoAudio),
            UsJob::EmptySing => return Ok(Vec::new()),
            _ => return Err(pool::RenderFailure::Engine("wrong ROM".into())),
        };
        let ctx = jp::voice::Context {
            sound: 0x607b10,
            voice: 0,
            thread: 1,
            sync: jp::sync::GuestSync {
                engine_base: 0,
                mutex_a: 0,
                mutex_b: 0,
            },
        };
        let mut voice = jp::voice::JpVoice::new(
            jp::Engine::new(&mut self.mem),
            self.tables.tables(),
            self.tables.preset(),
            &mut self.text,
            jp::PcmLog::new(&mut self.pcm),
            ctx,
        );
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            voice.speak(text, None, params, false)
        }))
        .map_err(|_| pool::RenderFailure::Engine("engine panic".into()))?
        .map_err(|e| pool::RenderFailure::Engine(format!("engine error: {e:?}")))?;
        let n = voice.pcm_len();
        if n == 0 {
            return Err(pool::RenderFailure::NoAudio);
        }
        Ok(self.pcm[..n].to_vec())
    }
    fn restart(&mut self) {
        self.mem.fill(0);
        self.text.fill(0);
    }
}

pub fn parameters(sliders: [u8; 5], intonation: u8) -> [(i32, i32); 6] {
    fn scale(value: u8, low: i32, high: i32) -> i32 {
        let fraction = value as f32 / 100.0;
        let delta = (high - low) as f32 * fraction;
        (low as f32 + delta) as i32
    }
    [
        (4, scale(sliders[0], 3500, 7000)),
        (3, scale(100 - sliders[1], 50, 180)),
        (6, scale(sliders[2], 80, 130)),
        (15, scale(sliders[3], 0, 400)),
        (14, scale(sliders[4], 0, 400)),
        (12, intonation as i32),
    ]
}
