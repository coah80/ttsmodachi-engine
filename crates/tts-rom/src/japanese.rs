use tts_core::jp;
pub struct JpImage {
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

impl JpImage {
    fn win(&self, addr: usize, len: usize) -> &[u8] {
        let o = addr - 0x100000;
        &self.code[o..o + len]
    }
    pub fn load(dump: &crate::image::Dump) -> Result<Self, String> {
        let mut code = dump.code()?;
        dump.verify_jp(&code)?;
        crate::image::mask_hook_body(&mut code);
        let w = |addr: usize, len: usize| {
            let o = addr - 0x100000;
            code[o..o + len].to_vec()
        };
        let to_i16 = |b: &[u8]| {
            b.chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]))
                .collect::<Vec<_>>()
        };
        let t = 0x722740 - 0x100000;
        let mut ph38 = Vec::new();
        for i in 0..38 {
            let p = u32::from_le_bytes([
                code[t + i * 4],
                code[t + i * 4 + 1],
                code[t + i * 4 + 2],
                code[t + i * 4 + 3],
            ]) as usize;
            let e = code[p - 0x100000..].iter().position(|&b| b == 0).unwrap();
            ph38.push(code[p - 0x100000..p - 0x100000 + e + 1].to_vec());
        }
        let twp = u32::from_le_bytes([
            code[0x72273c - 0x100000],
            code[0x72273d - 0x100000],
            code[0x72273e - 0x100000],
            code[0x72273f - 0x100000],
        ]) as usize;
        assert_eq!(twp, 0x74249c, "tw pointer");
        Ok(JpImage {
            t8468: to_i16(&w(0x8468d0, 512)),
            t7436: to_i16(&w(0x743608, 10)),
            t82e2: to_i16(&w(0x82e2b2 - 256, 356)),
            t82e0: to_i16(&w(0x82e0fc, 182)),
            t908: to_i16(&w(0x743908, 12)),
            t89e: to_i16(&w(0x7438fe, 12)),
            t912: to_i16(&w(0x743912, 10)),
            ph38,
            code,
        })
    }
    pub fn tables(&self) -> jp::Tables<'_> {
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
            delimiters: self.win(0x19eae4, 36),
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
    pub fn preset(&self) -> &[u8] {
        self.win(0x715514, 8192)
    }
}
