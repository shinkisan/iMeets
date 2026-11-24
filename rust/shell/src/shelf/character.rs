//! Character pose sprites from MyMeets material packs.

use std::collections::BTreeMap;

use super::marriage::PARTS;

const TRANSPARENT: u16 = 0x07e0;
const FAMILY_WIDTH: usize = 48;
const FAMILY_HEIGHT: usize = 40;
const PALETTES: [[u16; 8]; 13] = [
    [
        0xd7ff, 0x6fff, 0x071c, 0x67ff, 0x06df, 0x05ff, 0x4f5e, 0x067a,
    ],
    [
        0x9c7f, 0x83bf, 0x49de, 0x62df, 0x101c, 0x0816, 0x0010, 0x000d,
    ],
    [
        0xfe73, 0xed2d, 0xc409, 0xfb09, 0xe2a7, 0xb963, 0xa0a0, 0x8080,
    ],
    [
        0xc638, 0xa514, 0x8430, 0x73ae, 0x632c, 0x4a69, 0x39c9, 0x2104,
    ],
    [
        0xffff, 0xffff, 0xf79e, 0xffff, 0xef7e, 0xdedb, 0xd69a, 0xbebb,
    ],
    [
        0xffff, 0xeedf, 0xd5de, 0xde1f, 0xcd5f, 0xabfd, 0xa41f, 0x72bc,
    ],
    [
        0x7f5f, 0x05df, 0x043f, 0x061f, 0x049f, 0x039c, 0x02d3, 0x024f,
    ],
    [
        0xefff, 0xc7df, 0x875e, 0xe7ff, 0xbfdf, 0x5e9b, 0xcfbf, 0x779f,
    ],
    [
        0x47b5, 0x2691, 0x054c, 0x4d4f, 0x446c, 0x1b85, 0x046a, 0x0388,
    ],
    [
        0xffde, 0xff59, 0xe633, 0xffba, 0xfe90, 0xd56b, 0xd4ed, 0xab87,
    ],
    [
        0xff5f, 0xd5fc, 0xbc98, 0xd53c, 0xa357, 0x7a32, 0xaa3a, 0x8997,
    ],
    [
        0xffff, 0xe71c, 0xad55, 0xffff, 0xd69a, 0xbdf7, 0xef7d, 0xd6ba,
    ],
    [
        0xdf7d, 0xc6db, 0x9555, 0x7c71, 0x63ae, 0x3249, 0x21a6, 0x08e3,
    ],
];

pub(super) struct Sprite<'a> {
    width: usize,
    height: usize,
    palette: &'a [u8],
    data: &'a [u8],
}

impl<'a> Sprite<'a> {
    pub(super) fn read(bytes: &'a [u8]) -> Option<Self> {
        let [width, height, colors] = <[u8; 3]>::try_from(bytes.get(..3)?).ok()?.map(usize::from);
        if !(1..=128).contains(&width) || !(1..=128).contains(&height) || colors == 0 {
            return None;
        }
        let pixel_count = width * height;
        if pixel_count % 2 != 0 {
            return None;
        }
        let palette_end = 6 + colors * 2;
        let data_len = if colors <= 16 {
            pixel_count / 2
        } else {
            pixel_count
        };
        let sprite = Self {
            width,
            height,
            palette: bytes.get(6..palette_end)?,
            data: bytes.get(palette_end..palette_end + data_len)?,
        };
        (0..pixel_count)
            .all(|i| sprite.index(i) < colors)
            .then_some(sprite)
    }

    fn index(&self, i: usize) -> usize {
        if self.palette.len() <= 32 {
            usize::from((self.data[i / 2] >> ((i % 2) * 4)) & 15)
        } else {
            usize::from(self.data[i])
        }
    }

    fn pixels(&self) -> Vec<u16> {
        let palette: Vec<_> = self
            .palette
            .chunks_exact(2)
            .map(|word| u16::from_be_bytes([word[0], word[1]]))
            .collect();
        (0..self.width * self.height)
            .map(|i| palette[self.index(i)])
            .collect()
    }
}

pub(super) fn family_image(character: &[u8], colors: [u8; 3]) -> Option<Vec<u8>> {
    let frames = [9, 15, 7, 6];
    let sprites = PARTS
        .iter()
        .zip(frames)
        .map(|(part, pose)| {
            let header = part.header + pose * 4;
            let start =
                u32::from_le_bytes(character.get(header..header + 4)?.try_into().ok()?) as usize;
            (start >= part.pixel).then_some(())?;
            Sprite::read(character.get(start..part.end)?)
        })
        .collect::<Option<Vec<_>>>()?;
    let mut body = sprites[0].pixels();
    let mut anchors = [(0, 0); 4];
    let marker_colors: [u16; 4] = body.get(..4)?.try_into().ok()?;
    body[..4].fill(marker_colors[0]);
    // The first body pixels give the background beneath each colored anchor.
    for (i, pixel) in body.iter_mut().enumerate() {
        let (layer, background) = match *pixel {
            0x07ff => (1, 2), // Yellow anchors the face.
            0x001f => (2, 1), // Red anchors the head.
            0xf800 => (3, 3), // Blue anchors the accessory.
            _ => continue,
        };
        anchors[layer] = (
            (i % sprites[0].width) as isize,
            (i / sprites[0].width) as isize,
        );
        *pixel = marker_colors[background];
    }

    let order = if *character.get(506)? == 0 {
        [3, 0, 2, 1]
    } else {
        [0, 2, 1, 3]
    };
    let mut canvas = [TRANSPARENT; FAMILY_WIDTH * FAMILY_HEIGHT];
    let layer_colors = [colors[0], 12, colors[1], colors[2]];
    for layer in order {
        let sprite = &sprites[layer];
        let mut x = 8;
        let mut y = 4;
        if layer != 0 {
            x += anchors[layer].0 - (sprite.width / 2) as isize;
            y += anchors[layer].1 - (sprite.height / 2) as isize;
        }
        let palette = PALETTES.get(usize::from(layer_colors[layer]))?;
        let original = if layer == 0 {
            body.clone()
        } else {
            sprite.pixels()
        };
        for (i, pixel) in original.into_iter().enumerate() {
            if pixel == TRANSPARENT {
                continue;
            }
            let column = x + (i % sprite.width) as isize;
            let row = y + (i / sprite.width) as isize;
            if (0..FAMILY_WIDTH as isize).contains(&column)
                && (0..FAMILY_HEIGHT as isize).contains(&row)
            {
                let recolored = PALETTES[12]
                    .iter()
                    .position(|&gray| distance(gray, pixel) <= 100)
                    .map(|index| palette[index])
                    .unwrap_or(pixel);
                canvas[row as usize * FAMILY_WIDTH + column as usize] = recolored;
            }
        }
    }
    Some(encode_family_image(&canvas))
}

fn distance(a: u16, b: u16) -> i32 {
    let red = |v: u16| i32::from(v & 31) * 255 / 31;
    let green = |v: u16| i32::from((v >> 5) & 63) * 255 / 63;
    let blue = |v: u16| i32::from(v >> 11) * 255 / 31;
    (red(a) - red(b)).pow(2) + (green(a) - green(b)).pow(2) + (blue(a) - blue(b)).pow(2)
}

fn encode_family_image(pixels: &[u16]) -> Vec<u8> {
    let mut counts = BTreeMap::new();
    for &pixel in pixels.iter().filter(|&&color| color != TRANSPARENT) {
        *counts.entry(pixel).or_insert(0) += 1;
    }
    let mut colors: Vec<_> = counts.into_iter().collect();
    colors.sort_by_key(|&(color, count)| (std::cmp::Reverse(count), color));
    let palette: Vec<_> = std::iter::once(TRANSPARENT)
        .chain(colors.into_iter().take(15).map(|(color, _)| color))
        .collect();
    let mut bytes = vec![
        FAMILY_WIDTH as u8,
        FAMILY_HEIGHT as u8,
        palette.len() as u8,
        0,
        1,
        255,
    ];
    for color in &palette {
        bytes.extend(color.to_be_bytes());
    }
    let index = |color| {
        if color == TRANSPARENT {
            return 0;
        }
        palette
            .iter()
            .enumerate()
            .skip(1)
            .min_by_key(|&(_, &sample)| distance(sample, color))
            .map_or(0, |(i, _)| i as u8)
    };
    bytes.extend(
        pixels
            .chunks_exact(2)
            .map(|pair| index(pair[0]) | (index(pair[1]) << 4)),
    );
    bytes.resize(bytes.len().next_multiple_of(4), 0);
    bytes
}
