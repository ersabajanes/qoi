use std::fs::File;
use std::io::{Read, Write};

#[inline]
fn index_position(pix: [u8; 4]) -> u8 {
    let [r, g, b, a] = pix;

    // (r * 3 + g * 5 + b * 7 + a * 11) % 64
    u8::wrapping_add(
        u8::wrapping_add(u8::wrapping_mul(r, 3), u8::wrapping_mul(g, 5)),
        u8::wrapping_add(u8::wrapping_mul(b, 7), u8::wrapping_mul(a, 11)),
    ) % 64
}

#[inline]
fn sub(lhs: [u8; 4], rhs: [u8; 4]) -> [u8; 4] {
    [
        u8::wrapping_sub(lhs[0], rhs[0]),
        u8::wrapping_sub(lhs[1], rhs[1]),
        u8::wrapping_sub(lhs[2], rhs[2]),
        u8::wrapping_sub(lhs[3], rhs[3]),
    ]
}

#[inline]
fn add(lhs: [u8; 4], rhs: [u8; 4]) -> [u8; 4] {
    [
        u8::wrapping_add(lhs[0], rhs[0]),
        u8::wrapping_add(lhs[1], rhs[1]),
        u8::wrapping_add(lhs[2], rhs[2]),
        u8::wrapping_add(lhs[3], rhs[3]),
    ]
}

#[inline]
fn lte(lhs: [u8; 4], rhs: [u8; 4]) -> bool {
    lhs[0] <= rhs[0] && lhs[1] <= rhs[1] && lhs[2] <= rhs[2] && lhs[3] <= rhs[3]
}

pub fn save(filename: &str, w: u32, h: u32, d: u8, c: u8, data: &[u8]) -> std::io::Result<()> {
    let mut file = File::create(filename)?;

    file.write_all(b"qoif")?; // Header
    file.write_all(&u32::to_be_bytes(w))?;
    file.write_all(&u32::to_be_bytes(h))?;
    file.write_all(&[d, c])?;

    let mut run: i32 = 0;
    let mut prev: [u8; 4] = [0, 0, 0, 255];
    let mut seen: [[u8; 4]; 64] = [[0; 4]; 64];
    for i in (0..data.len()).step_by(d as usize) {
        let curr = [
            data[i + 0],
            data[i + 1],
            data[i + 2],
            if d == 4 { data[i + 3] } else { 255 },
        ];

        if prev == curr {
            run += 1;
            continue;
        }

        while run > 0 {
            let part = std::cmp::min(run, 62) - 1;
            run -= part + 1;
            let chunk: [u8; 1] = [0xc0 | part as u8];
            file.write_all(&chunk)?;
            // println!("{:02b}{:06b}  # QOI_OP_RUN", 0b11, part);
        }

        let look = index_position(curr);
        let diff = sub(curr, prev);
        let luma = sub(diff, [diff[1], 0, diff[1], 0]);
        let diff = add(diff, [2, 2, 2, 0]);
        let luma = add(luma, [8, 32, 8, 0]);

        if seen[look as usize] == curr {
            let chunk: [u8; 1] = [0x00 | look];
            file.write_all(&chunk)?;
            // println!("{:02b}{:06b}  # QOI_OP_INDEX", 0b00, look);
        } else if lte(diff, [3, 3, 3, 0]) {
            let chunk: [u8; 1] = [0x40 | diff[0] << 4 | diff[1] << 2 | diff[2]];
            file.write_all(&chunk)?;
            // println!("{:02b}{:02b}{:02b}{:02b}  # QOI_OP_DIFF", 0b01, diff[0], diff[1], diff[2]);
        } else if lte(luma, [15, 63, 15, 0]) {
            let chunk: [u8; 2] = [0x80 | luma[1], luma[0] << 4 | luma[2]];
            file.write_all(&chunk)?;
            // println!("{:02b}{:06b} {:04b}{:04b}  # QOI_OP_LUMA", 0b10, luma[1], luma[0], luma[2]);
        } else if diff[3] == 0 {
            let chunk: [u8; 4] = [0xfe, curr[0], curr[1], curr[2]];
            file.write_all(&chunk)?;
            // println!("{:08b} {:08b} {:08b} {:08b}  # QOI_OP_RGB", 0xfe, curr[0], curr[1], curr[2]);
        } else {
            let chunk: [u8; 5] = [0xff, curr[0], curr[1], curr[2], curr[3]];
            file.write_all(&chunk)?;
            // println!("{:08b} {:08b} {:08b} {:08b} {:08b}  # QOI_OP_RGBA", 0xff, curr[0], curr[1], curr[2], curr[3]);
        }

        seen[look as usize] = curr;
        prev = curr;
    }

    while run > 0 {
        let part = std::cmp::min(run, 62) - 1;
        run -= part + 1;
        let chunk: [u8; 1] = [0xc0 | part as u8];
        file.write_all(&chunk)?;
        // println!("{:02b}{:06b}  # QOI_OP_RUN", 0b11, part);
    }

    file.write_all(b"\x00\x00\x00\x00\x00\x00\x00\x01")?; // End marker
    file.flush()?;
    Ok(())
}

pub fn load(
    filename: &str,
    w: &mut u32,
    h: &mut u32,
    d: &mut u8,
    c: &mut u8,
) -> std::io::Result<Vec<u8>> {
    let mut data = Vec::new();
    File::open(filename)?.read_to_end(&mut data)?;
    let data = data;

    assert!(data.len() >= 22);
    assert_eq!(&data[..4], b"qoif"); // Header
    *w = u32::from_be_bytes(data[4..8].try_into().unwrap());
    *h = u32::from_be_bytes(data[8..12].try_into().unwrap());
    *d = data[12];
    *c = data[13];

    let mut image = Vec::<u8>::with_capacity(*w as usize * *h as usize * *d as usize);

    let mut idx: usize = 14;
    let mut prev: [u8; 4] = [0, 0, 0, 255];
    let mut seen: [[u8; 4]; 64] = [[0; 4]; 64];
    while idx < data.len() - 8 {
        let chunk = data[idx];
        idx += 1;

        if chunk == 0xff {
            prev[0] = data[idx + 0];
            prev[1] = data[idx + 1];
            prev[2] = data[idx + 2];
            prev[3] = data[idx + 3];
            idx += 4;
            // println!("QOI_OP_RGBA");
        } else if chunk == 0xfe {
            prev[0] = data[idx + 0];
            prev[1] = data[idx + 1];
            prev[2] = data[idx + 2];
            idx += 3;
            // println!("QOI_OP_RGB");
        } else if chunk & 0xc0 == 0xc0 {
            let run = chunk & 0x3f;
            for _ in 0..run {
                image.push(prev[0]);
                image.push(prev[1]);
                image.push(prev[2]);
                if *d == 4 {
                    image.push(prev[3]);
                }
            }
            // println!("QOI_OP_RUN");
        } else if chunk & 0xc0 == 0x80 {
            let next = data[idx + 0];
            idx += 1;
            let mut luma = [next >> 4 & 0x0f, chunk & 0x3f, next >> 0 & 0x0f, 0];
            luma = sub(luma, [8, 32, 8, 0]);
            luma = add(luma, [luma[1], 0, luma[1], 0]);
            prev = add(prev, luma);
            // println!("QOI_OP_LUMA");
        } else if chunk & 0xc0 == 0x40 {
            let mut diff = [chunk >> 4 & 0x03, chunk >> 2 & 0x03, chunk >> 0 & 0x03, 0];
            diff = sub(diff, [2, 2, 2, 0]);
            prev = add(prev, diff);
            // println!("QOI_OP_DIFF");
        } else if chunk & 0xc0 == 0x00 {
            let look = chunk & 0x3f;
            prev = seen[look as usize];
            // println!("QOI_OP_INDEX");
        }

        let look = index_position(prev);
        seen[look as usize] = prev;

        image.push(prev[0]);
        image.push(prev[1]);
        image.push(prev[2]);
        if *d == 4 {
            image.push(prev[3]);
        }
    }

    assert_eq!(&data[data.len() - 8..], b"\x00\x00\x00\x00\x00\x00\x00\x01"); // End marker
    Ok(image)
}

mod tests {
    #[test]
    fn save_load() {
        const W: usize = 256;
        const H: usize = 256;
        const D: usize = 4;
        const C: usize = 0;

        let mut save: [u8; W * H * D] = [255; W * H * D];
        for i in 0..W {
            for j in 0..H {
                save[(i * H + j) * D + 0] = i as u8;
                save[(i * H + j) * D + 1] = j as u8;
                save[(i * H + j) * D + 2] = 0 as u8;
            }
        }

        const E: f32 = 1.0;
        const G: f32 = 1.0 / 2.2;
        for i in (0..save.len()).step_by(D as usize) {
            save[i + 0] = (E * (save[i + 0] as f32 / 255.0).powf(G) * 255.0) as u8;
            save[i + 1] = (E * (save[i + 1] as f32 / 255.0).powf(G) * 255.0) as u8;
            save[i + 2] = (E * (save[i + 2] as f32 / 255.0).powf(G) * 255.0) as u8;
        }

        crate::save("test.qoi", W as u32, H as u32, D as u8, C as u8, &save).unwrap();

        let mut w = 0;
        let mut h = 0;
        let mut d = 0;
        let mut c = 0;
        let load = crate::load("test.qoi", &mut w, &mut h, &mut d, &mut c).unwrap();

        assert_eq!(load[..], save[..]);
    }
}
