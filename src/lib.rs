#[cfg(test)]
mod qoi;

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

        crate::qoi::save("test.qoi", W as u32, H as u32, D as u8, C as u8, &save).unwrap();

        let mut w = 0;
        let mut h = 0;
        let mut d = 0;
        let mut c = 0;
        let load = crate::qoi::load("test.qoi", &mut w, &mut h, &mut d, &mut c).unwrap();

        assert_eq!(load[..], save[..]);
    }
}
