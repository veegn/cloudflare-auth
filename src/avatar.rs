//! 确定性 identicon SVG

fn hash_bytes(input: &str) -> Vec<u8> {
    let mut h1: u32 = 0x811c9dc5;
    let mut h2: u32 = 0x01000193;
    let src = format!("{input}|avatar");
    for (i, b) in src.bytes().enumerate() {
        h1 ^= b as u32;
        h1 = h1.wrapping_mul(0x01000193);
        h2 = h2.wrapping_add((b as u32).wrapping_mul((i as u32) + 7));
    }
    let mut out = Vec::with_capacity(16);
    for _ in 0..16 {
        h1 ^= h1 << 13;
        h1 ^= h1 >> 17;
        h1 ^= h1 << 5;
        h2 = h2.wrapping_mul(1664525).wrapping_add(1013904223);
        out.push((h1 ^ h2) as u8);
    }
    out
}

fn hsl(h: f32, s: f32, l: f32) -> String {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let s = s.clamp(0.0, 100.0) / 100.0;
    let l = l.clamp(0.0, 100.0) / 100.0;
    let a = s * l.min(1.0 - l);
    let f = |n: f32| -> u8 {
        let k = (n + h / 30.0) % 12.0;
        let value = l - a * (k - 3.0).min(9.0 - k).min(1.0).max(-1.0);
        (255.0 * value.clamp(0.0, 1.0)).round() as u8
    };
    format!("#{:02x}{:02x}{:02x}", f(0.0), f(8.0), f(4.0))
}

pub fn render_identicon_svg(seed: &str, size: u32) -> String {
    let seed = if seed.is_empty() { "anonymous" } else { seed };
    let bytes = hash_bytes(seed);
    let bg_h = bytes[0] as f32 % 360.0;
    let fg_h = (bg_h + 140.0 + (bytes[1] as f32 % 80.0)) % 360.0;
    let bg = hsl(bg_h, 28.0, 93.0);
    let fg = hsl(fg_h, 55.0, 42.0);
    let fg2 = hsl((fg_h + 20.0) % 360.0, 45.0, 58.0);

    let mut cells = String::new();
    let unit = 20;
    for y in 0..5 {
        for x in 0..3 {
            let bit = bytes[2 + y + x] % 3;
            if bit == 0 {
                continue;
            }
            let color = if bit == 1 { &fg } else { &fg2 };
            let x1 = x * unit;
            let x2 = (4 - x) * unit;
            let y1 = y * unit;
            cells.push_str(&format!(
                r#"<rect x="{x1}" y="{y1}" width="{unit}" height="{unit}" fill="{color}"/>"#
            ));
            if x != 2 {
                cells.push_str(&format!(
                    r#"<rect x="{x2}" y="{y1}" width="{unit}" height="{unit}" fill="{color}"/>"#
                ));
            }
        }
    }
    let deco = if bytes[9] % 2 == 0 {
        format!(r#"<circle cx="50" cy="50" r="14" fill="{bg}" opacity="0.55"/>"#)
    } else {
        format!(r#"<rect x="36" y="36" width="28" height="28" rx="6" fill="{bg}" opacity="0.45"/>"#)
    };

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 100 100" role="img" aria-label="User avatar"><rect width="100" height="100" fill="{bg}"/>{cells}{deco}</svg>"#
    )
}

pub fn generate_avatar_seed() -> String {
    crate::password::random_hex(8)
}
