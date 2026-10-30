use crate::api;

const MAGIC: &str = "D1";

fn enc(b: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut o = String::new();
    for c in b.chunks(3) {
        let n = c.len();
        let b0 = c[0] as u32;
        let b1 = if n > 1 { c[1] as u32 } else { 0 };
        let b2 = if n > 2 { c[2] as u32 } else { 0 };
        let v = (b0 << 16) | (b1 << 8) | b2;
        o.push(T[(v >> 18) as usize & 63] as char);
        o.push(T[(v >> 12) as usize & 63] as char);
        if n > 1 {
            o.push(T[(v >> 6) as usize & 63] as char);
        }
        if n > 2 {
            o.push(T[v as usize & 63] as char);
        }
    }
    o
}

fn dec(s: &str) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits = 0;
    let mut out = Vec::new();
    for ch in s.bytes() {
        let v = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

fn put(o: &mut Vec<u8>, s: &str) -> Option<()> {
    let b = s.as_bytes();
    if b.len() > u16::MAX as usize {
        return None;
    }
    o.extend_from_slice(&(b.len() as u16).to_le_bytes());
    o.extend_from_slice(b);
    Some(())
}

fn take_raw<'a>(b: &'a [u8], i: &mut usize) -> Option<&'a [u8]> {
    if *i + 2 > b.len() {
        return None;
    }
    let n = u16::from_le_bytes([b[*i], b[*i + 1]]) as usize;
    *i += 2;
    let s = b.get(*i..*i + n)?;
    *i += n;
    Some(s)
}

fn take<'a>(b: &'a [u8], i: &mut usize) -> Option<&'a str> {
    if *i + 2 > b.len() {
        return None;
    }
    let n = u16::from_le_bytes([b[*i], b[*i + 1]]) as usize;
    *i += 2;
    let s = b.get(*i..*i + n)?;
    *i += n;
    std::str::from_utf8(s).ok()
}

fn hexpack(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn hexunpack(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn create(label: &str, dt: &api::Mdata) -> Option<String> {
    let mut raw = Vec::new();
    put(&mut raw, dt.id.as_str())?;
    put(&mut raw, dt.time.as_str())?;
    let packed = hexpack(&dt.sign);
    let flag: u8 = if packed.is_some() { 1 } else { 0 };
    let sign = packed.unwrap_or_else(|| dt.sign.as_bytes().to_vec());
    raw.push(flag);
    raw.extend_from_slice(&(sign.len() as u16).to_le_bytes());
    raw.extend_from_slice(&sign);
    put(&mut raw, dt.minfo.format.as_str())?;
    put(&mut raw, dt.minfo.bitrate.as_str())?;
    put(&mut raw, label)?;
    Some(format!("{MAGIC}{}", enc(&raw)))
}

pub fn parse(s: &str) -> Option<String> {
    let t = s.trim();
    let t = t.rsplit_once("#s/").map(|(_, id)| id).unwrap_or(t);
    let t = t.trim_start_matches(MAGIC);
    let raw = dec(t)?;
    let mut i = 0;
    let id = take(&raw, &mut i)?;
    if id.is_empty() {
        return None;
    }
    take(&raw, &mut i)?;
    if *raw.get(i)? != 0 && *raw.get(i)? != 1 {
        return None;
    }
    i += 1;
    let sign = take_raw(&raw, &mut i)?;
    if sign.is_empty() {
        return None;
    }
    take(&raw, &mut i)?;
    take(&raw, &mut i)?;
    take(&raw, &mut i)?;
    if i != raw.len() {
        return None;
    }
    Some(format!("{MAGIC}{t}"))
}

pub fn resolve(token: &str) -> Option<api::Nusic> {
    let raw = dec(token.trim_start_matches(MAGIC))?;
    let mut i = 0;
    let id = take(&raw, &mut i)?.to_string();
    let time = take(&raw, &mut i)?.to_string();
    let flag = *raw.get(i)?;
    i += 1;
    let sign_bytes = take_raw(&raw, &mut i)?;
    let sign = if flag == 1 {
        hexunpack(sign_bytes)
    } else {
        std::str::from_utf8(sign_bytes).ok()?.to_string()
    };
    let format = take(&raw, &mut i)?.to_string();
    let bitrate = take(&raw, &mut i)?.to_string();
    let label = take(&raw, &mut i)?.to_string();
    if i != raw.len() {
        return None;
    }
    let (title, artist, album) = match label
        .split_once(" by ")
        .and_then(|(t, rest)| rest.rsplit_once(" (").map(|(a, al)| (t, a, al)))
    {
        Some((t, a, al)) => (
            t.to_string(),
            a.to_string(),
            al.trim_end_matches(')').to_string(),
        ),
        None => (label.clone(), String::new(), String::new()),
    };
    Some(api::Nusic {
        title,
        artist,
        album,
        data: api::Mdata {
            id,
            time,
            sign,
            minfo: api::Minfobj {
                format,
                bitrate,
                level: String::new(),
                size: String::new(),
            },
            pic: String::new(),
        },
    })
}
