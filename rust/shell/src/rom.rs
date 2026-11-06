//! ROM recognition. A dump is named by its code hash, not its file name.

use std::io::Read;

use crate::copy::{self, Lang};

pub const ROM_BYTES: usize = 8_388_608;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageClass {
    Supported {
        version: u32,
        stem: String,
        title: String,
    },
    NotImage,
    Unsupported,
}

pub fn classify(data: &[u8]) -> ImageClass {
    if data.len() != ROM_BYTES {
        return ImageClass::NotImage;
    }
    match tg18::roms::identify(data) {
        Some(k) => ImageClass::Supported {
            version: k.version,
            stem: k.stem(),
            title: k.display_name(),
        },
        None => ImageClass::Unsupported,
    }
}

pub fn native_error(class: &ImageClass) -> Option<&'static str> {
    match class {
        ImageClass::NotImage => Some(copy::text(Lang::En, "err_not_8mib")),
        ImageClass::Unsupported => Some(copy::text(Lang::En, "err_unsupported")),
        ImageClass::Supported { .. } => None,
    }
}

pub fn user_error() -> &'static str {
    copy::text(Lang::En, "choose_rom")
}

/// Read at most 8 MiB + one chunk. Longer streams are unrecognized immediately.
pub fn read_candidate(mut r: impl Read) -> Result<Vec<u8>, ImageClass> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 64 * 1024];
    loop {
        let n = r.read(&mut tmp).map_err(|_| ImageClass::NotImage)?;
        if n == 0 {
            break;
        }
        if buf.len() + n > ROM_BYTES {
            return Err(ImageClass::NotImage);
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    if buf.len() != ROM_BYTES {
        Err(ImageClass::NotImage)
    } else {
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Read};

    struct Zeros {
        left: usize,
        served: usize,
        panic_after: usize,
    }

    impl Read for Zeros {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.left == 0 {
                return Ok(0);
            }
            let n = buf.len().min(self.left).min(1024 * 1024);
            if self.served + n > self.panic_after {
                panic!("read past the early reject window");
            }
            buf[..n].fill(0);
            self.left -= n;
            self.served += n;
            Ok(n)
        }
    }

    #[test]
    fn bad_images_and_early_reject() {
        assert!(matches!(classify(b"short"), ImageClass::NotImage));
        assert_eq!(
            native_error(&ImageClass::NotImage),
            Some("Expected an 8 MiB Meets/On SPI flash dump.")
        );
        let big = vec![0u8; ROM_BYTES];
        assert!(matches!(classify(&big), ImageClass::Unsupported));
        assert_eq!(
            native_error(&ImageClass::Unsupported),
            Some("This firmware version is not supported by tg18-emu yet.")
        );
        assert_eq!(user_error(), "choose a supported Meets / On ROM.");
        let mut over = Zeros {
            left: ROM_BYTES + 5_000_000,
            served: 0,
            panic_after: ROM_BYTES + 64 * 1024,
        };
        assert!(matches!(
            read_candidate(&mut over),
            Err(ImageClass::NotImage)
        ));
        assert!(over.served <= ROM_BYTES + 64 * 1024);
        let mut short = Zeros {
            left: 10,
            served: 0,
            panic_after: 100,
        };
        assert!(matches!(
            read_candidate(&mut short),
            Err(ImageClass::NotImage)
        ));
    }

    #[test]
    fn supported_rom_if_present() {
        let Some(path) = std::env::var_os("POCKET_MEETS_ROM") else {
            eprintln!("skip: POCKET_MEETS_ROM is not set");
            return;
        };
        let data = std::fs::read(path).expect("rom");
        match classify(&data) {
            ImageClass::Supported { stem, title, .. } => {
                assert!(stem.starts_with("fw_tg18_"));
                eprintln!("rom {title} {stem}");
            }
            other => panic!("expected a supported dump, got {other:?}"),
        }
        let sigs = crate::shelf::locate_firmware(&data).expect("shelf signatures");
        eprintln!(
            "shelf parameter={:#x} receive_type={:#x} marriage_event={:#x}",
            sigs.parameter, sigs.receive_type, sigs.marriage_event
        );
        assert!(sigs.parameter < 0x10_0000);
        assert!((0x2000_0000..0x2020_0000).contains(&sigs.marriage_event));
        assert!(sigs.receive_type >= 3);
    }
}
