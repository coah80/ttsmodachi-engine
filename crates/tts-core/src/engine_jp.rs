//! Japanese text-to-PCM request adapter over the existing kana engine.
//! UTF-8 kana requests are converted to the ROM engine's Shift-JIS katakana.
//! No state, voice tables, text workspace or PCM storage is allocated here.
//!
//! The request path lives in [`crate::jp::voice`]; this module keeps the
//! UTF-8 entry point and the corpus/acceptance tests over it.
use crate::jp;
use crate::jp::voice;

pub(crate) use voice::{encode_text, Context, Error};

/// UTF-8 text request over the [`voice`] path. The frontend and the engine
/// both run in Rust; voice tables, text workspace, engine state and output
/// remain caller owned.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_text(
    engine: &mut jp::Engine<'_>,
    tables: &jp::Tables<'_>,
    presets: &[u8],
    text: &str,
    text_workspace: &mut [u8; 0x400],
    parameters: &[(i32, i32)],
    context: &Context,
    teardown: bool,
    pcm: &mut jp::PcmLog<'_>,
) -> Result<(), Error> {
    voice::encode_text(text, text_workspace)?;
    voice::render(
        engine,
        tables,
        presets,
        text_workspace,
        parameters,
        context,
        teardown,
        pcm,
    )
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::{boxed::Box, format, fs, path::PathBuf, println, string::String, vec, vec::Vec};
    fn snapshot() -> PathBuf {
        std::env::var("TTS_JP_SNAPSHOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(std::env::var("HOME").unwrap()).join("tomodachi-roms/snapshot_JP")
            })
    }

    fn load_class_table() -> Vec<Vec<u8>> {
        let code = fs::read(snapshot().join("code.bin")).expect("snapshot code.bin");
        let base = 0x100000usize;
        let t = 0x722740 - base;
        let mut out = Vec::new();
        for i in 0..38 {
            let p = u32::from_le_bytes([
                code[t + i * 4],
                code[t + i * 4 + 1],
                code[t + i * 4 + 2],
                code[t + i * 4 + 3],
            ]) as usize;
            let e = code[p - base..].iter().position(|&b| b == 0).unwrap();
            out.push(code[p - base..p - base + e + 1].to_vec());
        }
        out
    }

    fn to_i16(bytes: &[u8]) -> Vec<i16> {
        bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect()
    }

    /// Borrowed ROM windows backing `jp::Tables` for render-path tests.
    struct JpTables {
        code: Vec<u8>,
        t8468: Vec<i16>,
        t7436: Vec<i16>,
        t82e2: Vec<i16>,
        t82e0: Vec<i16>,
        t908: Vec<i16>,
        t89e: Vec<i16>,
        t912: Vec<i16>,
        ph38: Vec<Vec<u8>>,
    }

    impl JpTables {
        fn win(&self, addr: usize, len: usize) -> &[u8] {
            let o = addr - 0x100000;
            &self.code[o..o + len]
        }

        fn load() -> Self {
            let code = fs::read(snapshot().join("code.bin")).expect("snapshot code.bin");
            let w = |addr: usize, len: usize| {
                let o = addr - 0x100000;
                code[o..o + len].to_vec()
            };
            // expander-table pointer resolves to D_74249C
            let twp = u32::from_le_bytes([
                code[0x72273c - 0x100000],
                code[0x72273d - 0x100000],
                code[0x72273e - 0x100000],
                code[0x72273f - 0x100000],
            ]) as usize;
            assert_eq!(twp, 0x74249c, "tw pointer");
            JpTables {
                t8468: to_i16(&w(0x8468d0, 512)),
                t7436: to_i16(&w(0x743608, 10)),
                t82e2: to_i16(&w(0x82e2b2 - 256, 356)),
                t82e0: to_i16(&w(0x82e0fc, 182)),
                t908: to_i16(&w(0x743908, 12)),
                t89e: to_i16(&w(0x7438fe, 12)),
                t912: to_i16(&w(0x743912, 10)),
                ph38: load_class_table(),
                code,
            }
        }

        fn tables(&self) -> jp::Tables<'_> {
            let k7873a4 = i16::from_le_bytes([self.win(0x7873a4, 2)[0], self.win(0x7873a4, 2)[1]]);
            let k = u32::from_le_bytes(self.win(0x743944, 4).try_into().unwrap());
            let p38: [&[u8]; 38] = self
                .ph38
                .iter()
                .map(|v| &v[..])
                .collect::<Vec<_>>()
                .try_into()
                .unwrap_or_else(|_| panic!("ph38 len"));
            jp::Tables {
                t790: self.win(0x790ba8, 16),
                t8468: &self.t8468,
                t7436: &self.t7436,
                t82e2: &self.t82e2,
                t82e0: &self.t82e0,
                t8460: self.win(0x8460c0, 320),
                tw: self.win(0x74249c, 258),
                t7439: self.win(0x74391c, 256),
                t846w: self.win(0x846200, 8192),
                t908: &self.t908,
                t89e: &self.t89e,
                t912: &self.t912,
                t787: self.win(0x780000, 0x10000),
                w787: 0x780000i32 - 0x7873a6,
                t7439pre: self.win(0x74391c - 128, 128),
                k743944: k,
                t743948: self.win(0x743948, self.code.len() - (0x743948 - 0x100000)),
                t74269c: self.win(0x74269c, 144),
                t74272c: self.win(0x74272c, 144),
                t7427bc: self.win(0x7427bc, 144),
                t74284c: self.win(0x74284c, 2400),
                t7431ec: self.win(0x7431ec, 216),
                t743380: self.win(0x743380, 200),
                t7434be: self.win(0x7434be, 260),
                t74362e: self.win(0x74362e, 640),
                t7435f4: self.win(0x7435f4, 32),
                t7868c4: self.win(0x7868c4, (k7873a4 as usize + 1) * 12),
                k7873a4,
                t743614: self.win(0x743614, 16),
                t743620: self.win(0x743620, 16),
                t74362c: self.win(0x74362c, 8),
                t722727: self.win(0x722727, 16),
                ph38: p38,
                tag53: self.win(0x740bc4, 6360),
                t7227d8: self.win(0x7227d8, 8),
                code: &self.code,
            }
        }

        fn preset(&self) -> &[u8] {
            self.win(0x715514, 4096)
        }
    }

    #[test]
    fn utf8_kana_boundaries() {
        let mut text = [0; 0x400];
        encode_text("こんにちは、コンニチハ？", &mut text).unwrap();
        let want = [
            0x83, 0x52, 0x83, 0x93, 0x83, 0x6a, 0x83, 0x60, 0x83, 0x6e, 0x81, 0x41, 0x83, 0x52,
            0x83, 0x93, 0x83, 0x6a, 0x83, 0x60, 0x83, 0x6e, 0x81, 0x48,
        ];
        assert_eq!(&text[..want.len()], &want);
        assert!(text[want.len()..].iter().all(|&v| v == 0));
        encode_text(&"a".repeat(1023), &mut text).unwrap();
        assert_eq!(text[1023], 0);
        assert_eq!(
            encode_text(&"a".repeat(1024), &mut text),
            Err(Error::InvalidText)
        );
        encode_text(&"あ".repeat(511), &mut text).unwrap();
        assert_eq!(
            encode_text(&"あ".repeat(512), &mut text),
            Err(Error::InvalidText)
        );
        assert_eq!(encode_text("漢字", &mut text), Err(Error::InvalidText));
        assert_eq!(encode_text("a\0b", &mut text), Err(Error::InvalidText));
    }

    fn gunzip(path: &std::path::Path) -> Vec<u8> {
        let out = std::process::Command::new("gzip")
            .arg("-dc")
            .arg(path)
            .output()
            .unwrap();
        assert!(out.status.success(), "missing fixture {path:?}");
        out.stdout
    }
    /// No state word is excluded: sync echoes and the scheduler request
    /// struct are reproduced bit-exactly from caller-owned inputs.
    #[allow(dead_code)]
    fn excluded(_o: usize) -> bool {
        false
    }
    fn compare_state(got: &[u8], want: &[u8]) {
        assert_eq!(got.len(), want.len());
        let word = |b: &[u8], o| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let base = word(want, 0xDEE4).wrapping_sub(word(got, 0xDEE4));
        for o in (0..got.len()).step_by(4) {
            let (r, a) = (word(got, o), word(want, o));
            if r == a {
                continue;
            }
            // Relative engine links are the only pointer normalization allowed.
            if a.wrapping_sub(base) == r && r < jp::ENGINE_SIZE as u32 {
                continue;
            }
            panic!("engine memory differs at {o:#x}: rust={r:#x}, ARM={a:#x}");
        }
    }
    #[test]
    #[ignore = "requires local ARM captures and voice tables"]
    fn japanese_request_pcm() {
        let phrases: [(&str, &str, &[(i32, i32)], bool); 8] = [
            ("konnichiwa", "こんにちは", &[], false),
            ("arigatou", "ありがとう", &[], false),
            ("long", "おはようございます、げんきですか", &[], false),
            (
                "params",
                "こんにちは",
                &[(3, 150), (4, 5500), (14, 300)],
                false,
            ),
            ("quality", "こんにちは", &[(6, 80)], false),
            ("punct", "おはよう、げんき？", &[], false),
            (
                "fxchain",
                "コンニチハ",
                &[(8, 100), (10, 500), (11, 50), (13, 50), (7, 1000)],
                false,
            ),
            ("voicestop", "ありがとう", &[], true),
        ];
        let dir = PathBuf::from(
            std::env::var("TTS_ENGINE_STAGES")
                .unwrap_or_else(|_| "engine-stages".into()),
        )
        .join("finish");
        let jt = JpTables::load();
        let tables = jt.tables();
        for (name, request, params, teardown) in phrases {
            let mut text = [0; 0x400];
            let mut mem = Box::new([0; jp::ENGINE_SIZE]);
            let mut store = vec![0; 1 << 20];
            let mut pcm = jp::PcmLog::new(&mut store);
            let want_eng = gunzip(&dir.join(format!("{name}.eng_arm.gz")));
            // Guest base the references were captured under (scheduler
            // request struct is base-placed); echoes are 0 under harness.
            let engine_base = u32::from_le_bytes(
                want_eng[0x13570..0x13574].try_into().unwrap(),
            )
            .wrapping_sub(0x1357C);
            render_text(
                &mut jp::Engine::new(&mut mem),
                &tables,
                jt.preset(),
                request,
                &mut text,
                params,
                &Context {
                    sound: 0x607b10,
                    voice: 0x863c480,
                    thread: 0xffff8000,
                    sync: jp::sync::GuestSync { engine_base, mutex_a: 0, mutex_b: 0 },
                },
                teardown,
                &mut pcm,
            )
            .unwrap();
            let arm = gunzip(&dir.join(format!("{name}.pcm_arm.gz")));
            let want = to_i16(&arm);
            let got = &pcm.buf[..pcm.len];
            assert_eq!(got.len(), want.len(), "{name}: sample count");
            if let Some(i) = got.iter().zip(&want).position(|(a, b)| a != b) {
                panic!(
                    "{name}: first differing sample {i}, rust={} ARM={}",
                    got[i], want[i]
                );
            }
            compare_state(&mem[..], &want_eng);
            println!("JP_ARM_IDENTICAL {name} samples={}", got.len());
            if std::env::var_os("TTS_JP_MASK_CHECK").is_some() {
                let offsets: Vec<usize> = (0..jp::ENGINE_SIZE).filter(|&o| excluded(o)).collect();
                for &offset in &offsets {
                    let mut altered = Box::new([0; jp::ENGINE_SIZE]);
                    let mut e = jp::Engine::new(&mut altered);
                    let sync = jp::sync::GuestSync { engine_base: 0x0D000000, mutex_a: 0, mutex_b: 0 };
                    jp::construct(&mut e, sync);
                    jp::reset(&mut e);
                    for &(id, v) in params {
                        jp::set_param(&mut e, jt.preset(), id, v);
                    }
                    let mut scratch = vec![0; 1 << 20];
                    let mut changed = jp::PcmLog::new(&mut scratch);
                    e.mem[offset] ^= 0xff;
                    assert_eq!(
                        jp::render_entry(
                            &mut e,
                            &tables,
                            &text,
                            0,
                            0x11,
                            0x607b10,
                            0x863c480,
                            0xffff8000,
                            &mut changed,
                            sync
                        ),
                        0
                    );
                    for _ in 0..64 {
                        if e.rd8(0xE6CB) == 0 {
                            break;
                        }
                        e.mem[offset] ^= 0xff;
                        jp::task_driver(&mut e, &tables, &mut changed);
                    }
                    assert_eq!(e.rd8(0xE6CB), 0, "masked byte {offset:#x} hung");
                    if teardown {
                        e.mem[offset] ^= 0xff;
                        jp::render_sync(&mut e, 0xffff8000);
                    }
                    assert_eq!(
                        &changed.buf[..changed.len],
                        got,
                        "{name}: masked byte {offset:#x} affects PCM"
                    );
                }
                println!(
                    "JP_MASK_IDENTICAL {name} bytes={} xor=255 at=submit,each-worker,teardown",
                    offsets.len()
                );
            }
        }
    }
}
