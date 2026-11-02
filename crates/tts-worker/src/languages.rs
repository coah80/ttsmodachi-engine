use super::{pool, sparse, Shared, UsFactory, UsJob};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Once, OnceLock};

pub struct Language {
    pub id: &'static str,
    name: &'static str,
    mode: u8,
    presets: [[u8; 6]; 2],
}

pub struct LanguagePool {
    pool: Arc<pool::RendererPool<UsJob>>,
    started: Once,
}

impl LanguagePool {
    pub fn render(&self, job: Arc<UsJob>) -> Result<pool::Rendered, String> {
        self.started.call_once(|| self.pool.start());
        self.pool.render(job)
    }

    pub fn health(&self) -> pool::PoolHealth {
        self.pool.health()
    }
}

type Pools = HashMap<String, Arc<LanguagePool>>;

struct SourceFactory {
    path: PathBuf,
    region: &'static str,
    mode: u8,
    prepared: OnceLock<Result<Arc<dyn pool::EngineFactory<UsJob>>, String>>,
}

impl pool::EngineFactory<UsJob> for SourceFactory {
    fn start(&self) -> Result<Box<dyn pool::LaneEngine<UsJob>>, String> {
        let prepared = self.prepared.get_or_init(|| {
            if self.region == "US" {
                us_factory(tts_rom::BootImage::load(&self.path)?)
            } else if self.region == "EU" || self.region == "KR" {
                let rom = if self.region == "EU" {
                    tts_core::rom_region::Rom::Eu
                } else {
                    tts_core::rom_region::Rom::Kr
                };
                native_factory(
                    tts_rom::regional_boot::load(&self.path, rom, self.mode)?,
                    rom,
                )
            } else {
                let dump = tts_rom::image::Dump::open(&self.path)?;
                let image = tts_rom::japanese::JpImage::load(&dump)?;
                Ok(Arc::new(super::japanese::Factory::new(image)))
            }
        });
        prepared.as_ref().map_err(Clone::clone)?.start()
    }
}

fn us_factory(image: tts_rom::BootImage) -> Result<Arc<dyn pool::EngineFactory<UsJob>>, String> {
    native_factory(image, tts_core::rom_region::Rom::Us)
}

fn native_factory(
    image: tts_rom::BootImage,
    rom: tts_core::rom_region::Rom,
) -> Result<Arc<dyn pool::EngineFactory<UsJob>>, String> {
    let size = image.arena.len();
    let boot = image.boot;
    let store = sparse::BootState::create(size).map_err(|e| format!("boot memory: {e}"))?;
    for (index, page) in image.arena.chunks(4096).enumerate() {
        if page.iter().any(|b| *b != 0) {
            store
                .write_at((index * 4096) as u64, page)
                .map_err(|e| format!("boot memory write: {e}"))?;
        }
    }
    drop(image);
    let boot_fd = store.fd_private();
    let snap =
        sparse::Mmap::shared_ro(boot_fd, size).map_err(|e| format!("boot memory mapping: {e}"))?;
    Ok(Arc::new(UsFactory {
        rom,
        boot_store: store,
        size,
        boot,
        snap: Arc::new(snap),
    }))
}

pub fn load(paths: &[PathBuf]) -> Result<(Pools, Vec<Language>), String> {
    let mut pools = HashMap::new();
    let mut languages = Vec::new();
    for path in paths {
        let dump = tts_rom::image::Dump::open(path)?;
        dump.ensure_decrypted()?;
        let (provided, region) = match dump.title_id {
            0x000400000008c300 => (vec![Language { id: "useng", name: "us english", mode: 1, presets: dump.us_presets()? }], "US"),
            0x000400000008c400 => {
                let presets = dump.eu_presets()?;
                let provided = [("eueng", "eu english"), ("fr", "french"), ("de", "german"), ("it", "italian"), ("es", "spanish")].into_iter().enumerate().map(|(i,(id,name))| Language {id,name,mode:i as u8 + 2,presets:presets[i]}).collect();
                (provided,"EU")
            }
            0x000400000008c500 => {
                dump.verify_jp(&dump.code()?)?;
                (vec![Language {id:"jp",name:"japanese",mode:0,presets:[[50,50,50,50,50,1];2]}],"JP")
            }
            0x0004000000120c00 => (vec![Language{id:"kr",name:"korean",mode:7,presets:dump.kr_presets()?}],"KR"),
            _ => return Err("wrong region or game: use a supported Tomodachi Life US Rev 1, EU Rev 1, KR original release or JP Rev 2 dump".into()),
        };
        if provided.iter().any(|l| pools.contains_key(l.id)) {
            return Err(format!("duplicate region: pass only one {region} dump"));
        }
        for language in provided {
            let factory = Arc::new(SourceFactory {
                path: path.clone(),
                region,
                mode: language.mode,
                prepared: OnceLock::new(),
            });
            let pool = pool::RendererPool::new(
                super::pool_config(if region == "EU" { language.id } else { region }),
                factory,
            );
            pools.insert(
                language.id.to_owned(),
                Arc::new(LanguagePool {
                    pool,
                    started: Once::new(),
                }),
            );
            languages.push(language);
        }
    }
    languages.sort_by_key(|l| super::LANG_TO_ID.iter().position(|id| *id == l.id));
    Ok((pools, languages))
}

pub fn body(languages: &[Language]) -> String {
    let rows: Vec<_> = languages
        .iter()
        .map(|language| {
            let presets: Vec<_> = language
                .presets
                .iter()
                .map(|preset| {
                    let fields: Vec<_> =
                        ["pitch", "speed", "quality", "tone", "accent", "intonation"]
                            .iter()
                            .zip(preset)
                            .map(|(name, value)| format!("{}:{value}", super::py_json_str(name)))
                            .collect();
                    format!("{{{}}}", fields.join(","))
                })
                .collect();
            format!(
                "{{\"id\":{},\"name\":{},\"defaults\":{{\"boy\":{},\"girl\":{}}}}}",
                super::py_json_str(language.id),
                super::py_json_str(language.name),
                presets[0],
                presets[1]
            )
        })
        .collect();
    format!("{{\"languages\":[{}]}}", rows.join(","))
}

pub fn require(stream: &mut std::net::TcpStream, shared: &Shared, lang: &str, head: bool) -> bool {
    if shared.pools.contains_key(lang) {
        return true;
    }
    let needed = match lang {
        "useng" => "a decrypted US Rev 1 dump",
        "jp" => "a decrypted JP Rev 2 dump",
        "eueng" | "fr" | "de" | "it" | "es" => "a decrypted EU Rev 1 dump",
        "kr" => "a decrypted KR original release dump",
        _ => "a supported dump",
    };
    super::err_detail(
        stream,
        400,
        &format!(
            "Language {lang} needs {needed}. Available: {}",
            shared
                .languages
                .iter()
                .map(|l| l.id)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        head,
    );
    false
}
