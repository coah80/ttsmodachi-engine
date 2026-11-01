

include!("rom_region_tables.rs");

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Rom {
    #[default]
    Us,
    Eu,
    Kr,
}

impl Rom {

    pub fn or(self, other: Rom) -> Rom {
        if self != Rom::Us {
            self
        } else {
            other
        }
    }

    pub fn from_entry(entry: u32) -> Rom {
        if is_us(entry) {
            return Rom::Us;
        }

        let eu = lookup_eu(entry);
        let kr = lookup_kr(entry);
        match (eu, kr) {
            (Some(a), Some(b)) => {
                if a < b {
                    Rom::Eu
                } else if b < a {
                    Rom::Kr
                } else {
                    Rom::Eu
                }
            }
            (Some(_), None) => Rom::Eu,
            (None, Some(_)) => Rom::Kr,
            (None, None) => Rom::Us,
        }
    }
}

pub fn addr(rom: Rom, us: u32) -> u32 {
    if rom == Rom::Us {
        return us;
    }
    if rom == Rom::Eu && (0x917000..0x918000).contains(&us) {
        return us + 0x1240;
    }
    if rom == Rom::Eu {
        match us {
            0x7ae148 => return 0x7af388,
            0x22a86c => return 0x22aa28,
            0x22a874 => return 0x22aa30,
            0x22a87c => return 0x22aa38,
            0x914092 => return 0x9152d2,
            0x91426c => return 0x9154ac,
            0x91436c => return 0x9155ac,
            0x9140e4 => return 0x915324,
            0x9147ac => return 0x9159ec,
            0x9147c4 => return 0x915a04,
            0x9147f4 => return 0x915a34,
            0x9147dc => return 0x915a1c,
            0x914044 => return 0x915284,
            0x91404a => return 0x91528a,
            0x914834 => return 0x915a74,
            0x914878 => return 0x915ab8,
            0x2a0e30 => return 0x2a0fec,
            0x2a3340 => return 0x2a34fc,
            0x2a334c => return 0x2a3508,
            _ => {}
        }
    }
    match REGION_TABLE.binary_search_by_key(&us, |row| row.0) {
        Ok(i) => {
            let (_, eu, kr) = REGION_TABLE[i];
            if rom == Rom::Eu {
                eu
            } else {
                kr
            }
        }
        Err(_) => us,
    }
}

pub fn to_us(rom: Rom, entry: u32) -> u32 {
    if is_us(entry) {
        return entry;
    }

    match rom {
        Rom::Us => {
            let eu = lookup_eu(entry);
            let kr = lookup_kr(entry);
            match (eu, kr) {
                (Some(a), Some(b)) => a.min(b),
                (Some(a), None) => a,
                (None, Some(b)) => b,
                (None, None) => entry,
            }
        }
        Rom::Eu => lookup_eu(entry).unwrap_or(entry),
        Rom::Kr => lookup_kr(entry).unwrap_or(entry),
    }
}

fn is_us(address: u32) -> bool {
    REGION_TABLE.binary_search_by_key(&address, |row| row.0).is_ok()
}

fn lookup_eu(entry: u32) -> Option<u32> {
    match REGION_BY_EU.binary_search_by_key(&entry, |row| row.0) {
        Ok(mut i) => {
            while i > 0 && REGION_BY_EU[i - 1].0 == entry {
                i -= 1;
            }
            Some(REGION_BY_EU[i].1)
        }
        Err(_) => None,
    }
}

fn lookup_kr(entry: u32) -> Option<u32> {
    match REGION_BY_KR.binary_search_by_key(&entry, |row| row.0) {
        Ok(mut i) => {
            while i > 0 && REGION_BY_KR[i - 1].0 == entry {
                i -= 1;
            }
            Some(REGION_BY_KR[i].1)
        }
        Err(_) => None,
    }
}

pub fn to_us_guest(rom: Rom, entry: u32) -> u32 {

    match rom {
        Rom::Eu => {
            if let Some(us) = lookup_eu(entry) {
                return us;
            }
        }
        Rom::Kr => {
            if let Some(us) = lookup_kr(entry) {
                return us;
            }
        }
        _ => {}
    }
    if is_us(entry) {
        return entry;
    }

    match rom {
        Rom::Us => {
            let eu = lookup_eu(entry);
            let kr = lookup_kr(entry);
            match (eu, kr) {
                (Some(a), Some(b)) => a.min(b),
                (Some(a), None) => a,
                (None, Some(b)) => b,
                (None, None) => entry,
            }
        }
        Rom::Eu => lookup_eu(entry).unwrap_or(entry),
        Rom::Kr => lookup_kr(entry).unwrap_or(entry),
    }
}

pub fn capture_slot(rom: Rom) -> u32 {
    match rom {
        Rom::Us => 0xc72688,
        Rom::Eu => 0xc7373c,
        Rom::Kr => 0xc72688,
    }
}
