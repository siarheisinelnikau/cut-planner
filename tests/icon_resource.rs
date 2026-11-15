//! Inspect embedded PE resources without loading or executing the application.
fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

#[test]
fn executable_contains_eight_nonblank_native_icon_sizes() {
    let bytes = std::fs::read(env!("CARGO_BIN_EXE_CutPlanner")).unwrap();
    assert_eq!(&bytes[..2], b"MZ");
    let pe = u32_at(&bytes, 60) as usize;
    assert_eq!(&bytes[pe..pe + 4], b"PE\0\0");
    assert_eq!(u16_at(&bytes, pe + 4), 0x8664);
    let optional = pe + 24;
    assert_eq!(u16_at(&bytes, optional), 0x20b);
    let sections = optional + u16_at(&bytes, pe + 20) as usize;
    let rva_to_offset = |rva: u32| {
        (0..u16_at(&bytes, pe + 6) as usize)
            .find_map(|i| {
                let at = sections + i * 40;
                let start = u32_at(&bytes, at + 12);
                let size = u32_at(&bytes, at + 8).max(u32_at(&bytes, at + 16));
                if rva >= start && rva < start + size {
                    Some((rva - start + u32_at(&bytes, at + 20)) as usize)
                } else {
                    None
                }
            })
            .expect("RVA must be in a PE section")
    };
    let base = rva_to_offset(u32_at(&bytes, optional + 112 + 16));
    let entries = |offset: usize| {
        (0..u16_at(&bytes, offset + 12) as usize + u16_at(&bytes, offset + 14) as usize)
            .map(|i| {
                let at = offset + 16 + i * 8;
                (u32_at(&bytes, at), u32_at(&bytes, at + 4))
            })
            .collect::<Vec<_>>()
    };
    let root = entries(base);
    assert!(
        root.iter().any(|(kind, _)| *kind == 14),
        "group icon required"
    );
    let (_, directory) = root
        .into_iter()
        .find(|(kind, _)| *kind == 3)
        .expect("RT_ICON resource required");
    let icons = entries(base + (directory & 0x7fff_ffff) as usize);
    assert_eq!(icons.len(), 8);
    let mut sizes = Vec::new();
    for (_, language_directory) in icons {
        let (_, data) = entries(base + (language_directory & 0x7fff_ffff) as usize)[0];
        let descriptor = base + data as usize;
        let start = rva_to_offset(u32_at(&bytes, descriptor));
        let length = u32_at(&bytes, descriptor + 4) as usize;
        let image = &bytes[start..start + length];
        assert_eq!(u32_at(image, 0), 40);
        assert_eq!(u16_at(image, 14), 32);
        let width = u32_at(image, 4) as usize;
        let height = u32_at(image, 8) as usize / 2;
        assert_eq!(width, height);
        sizes.push(width);
        let mut visible = 0;
        let mut min_x = width;
        let mut max_x = 0;
        for (index, pixel) in image[40..40 + width * height * 4]
            .chunks_exact(4)
            .enumerate()
        {
            if pixel[3] > 0 {
                visible += 1;
                min_x = min_x.min(index % width);
                max_x = max_x.max(index % width);
            }
        }
        assert!(
            visible > width * height / 2 && visible < width * height,
            "nonblank artwork with transparent corners"
        );
        assert!(
            max_x - min_x >= width * 4 / 5,
            "artwork must fill its small icon cell"
        );
    }
    sizes.sort();
    assert_eq!(sizes, vec![16, 20, 24, 32, 48, 64, 128, 256]);
}
