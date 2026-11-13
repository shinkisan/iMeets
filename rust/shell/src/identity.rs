//! Product strings. Mixing still
//! sends 1.4.3 because that is the client version the server saw.

pub const RELEASE: &str = "1.3";
pub const BUILD: i32 = 10027;
pub const MIXING_VERSION: &str = "1.4.3";
pub const CORE_VERSION: &str = "1.0.0";
pub const MIXING_APP_ID: &str = "com.bandai.tamagotchion";
pub const PACKAGE_ID: &str = "com.daiban.tamagotchion";
pub const APP_KEY: &str = "695719951020924";
pub const SUPPORT_URL: &str = "https://mixing-meets.com/support";
pub const ASSET_ROOT: &str = "asset/v7/Android";
pub const ASSET_HOST: &str = "https://dev.tmgcmeetsapp.com/asset/Android";
pub const BLE_NAME: &str = "TMGC_meets";
pub const FAKE_BLE_ADDRESS: &str = "02:00:00:00:00:18";
/// Album in the photo library. The product name is Pocket Meets.
pub const ALBUM: &str = "Pocket Meets";
pub const PNG_FOLDER_LABEL: &str = "Pictures/Pocket Meets";

/// Joined with no extra slash, the form to try first.
/// On 2026-10-31 the public host echoed both this path and `MIXING_URL_SLASHED`
/// (and any other unknown path) as `text/html`. `AppVersion` `1.4.3` and `9.9.9`
/// produced the same echo, so the slash and the version predicate stay open.
/// TODO(mixing-url-slash): SPEC §14.9.
pub const MIXING_URL: &str = "https://mixing-meets.com/com.daiban.tamagotchionapi_en/meets.php";
pub const MIXING_URL_SLASHED: &str =
    "https://mixing-meets.com/com.daiban.tamagotchion/api_en/meets.php";
/// Related endpoints from the client string table. Query fragments are known;
/// the request that uses them is not. TODO(bnid-query): SPEC §10.3.
pub const BNID_JOIN: &str = "api/bnidjoin.php";
pub const BNID_MOV: &str = "api/bnidmov.php";
pub const BNID_HOST: &str = "bandainamcoid.com";

pub const FAKE_DEVICE_JSON: &str =
    r#"{"data":["TMGC_meets"],"uuid_address":["02:00:00:00:00:18"]}"#;

pub fn meets_urls() -> [&'static str; 2] {
    [MIXING_URL, MIXING_URL_SLASHED]
}

pub fn should_show_notes(care_intro_version: i32) -> bool {
    care_intro_version < BUILD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_and_nine_hash_stems() {
        assert_eq!(RELEASE, "1.3");
        assert_eq!(BUILD, 10027);
        assert_eq!(MIXING_VERSION, "1.4.3");
        assert_ne!(MIXING_VERSION, RELEASE);
        assert!(should_show_notes(0));
        assert!(should_show_notes(10026));
        assert!(!should_show_notes(10027));
        assert!(!MIXING_URL.contains("/api_en"));
        assert!(MIXING_URL.contains("com.daiban.tamagotchionapi_en/meets.php"));
        assert_eq!(ALBUM, "Pocket Meets");
        assert_eq!(BNID_JOIN, "api/bnidjoin.php");
        assert_eq!(BNID_MOV, "api/bnidmov.php");
        assert_eq!(BNID_HOST, "bandainamcoid.com");
        assert_eq!(ASSET_HOST, "https://dev.tmgcmeetsapp.com/asset/Android");
        assert_eq!(ASSET_ROOT, "asset/v7/Android");

        let expect = [
            (
                0xD780_5452_8833_EB90,
                "en",
                57,
                "2019-05-13",
                "fairy",
                "Fairy",
                "fw_tg18_en_v057_2019-05-13_fairy",
                "Fairy (EN v057)",
            ),
            (
                0x8313_24F3_B2FA_6768,
                "en",
                58,
                "2019-05-13",
                "magic",
                "Magic",
                "fw_tg18_en_v058_2019-05-13_magic",
                "Magic (EN v058)",
            ),
            (
                0xA38D_22B6_76C5_3108,
                "en",
                63,
                "2020-02-13",
                "wondergarden",
                "Wonder Garden",
                "fw_tg18_en_v063_2020-02-13_wondergarden",
                "Wonder Garden (EN v063)",
            ),
            (
                0x7056_A91C_C05E_227F,
                "jp",
                30,
                "2018-10-19",
                "fairy",
                "Fairy",
                "fw_tg18_jp_v030_2018-10-19_fairy",
                "Fairy (JP v030)",
            ),
            (
                0x4089_9249_C2E4_6D53,
                "jp",
                31,
                "2018-10-19",
                "magic",
                "Magic",
                "fw_tg18_jp_v031_2018-10-19_magic",
                "Magic (JP v031)",
            ),
            (
                0xEA18_7BF0_CBFA_04F3,
                "jp",
                47,
                "2019-05-28",
                "fantasy",
                "Fantasy",
                "fw_tg18_jp_v047_2019-05-28_fantasy",
                "Fantasy (JP v047)",
            ),
            (
                0xA6FF_6D03_28E7_0278,
                "jp",
                55,
                "2019-03-10",
                "pastel",
                "Pastel",
                "fw_tg18_jp_v055_2019-03-10_pastel",
                "Pastel (JP v055)",
            ),
            (
                0x3A68_A53E_2DB1_64B2,
                "jp",
                56,
                "2019-04-24",
                "sanrio",
                "Sanrio",
                "fw_tg18_jp_v056_2019-04-24_sanrio",
                "Sanrio (JP v056)",
            ),
            (
                0x28AD_63C3_4FA9_E369,
                "jp",
                62,
                "2019-09-12",
                "sweets",
                "Sweets",
                "fw_tg18_jp_v062_2019-09-12_sweets",
                "Sweets (JP v062)",
            ),
        ];
        assert_eq!(tg18::roms::KNOWN.len(), 9);
        for (i, row) in expect.iter().enumerate() {
            let k = &tg18::roms::KNOWN[i];
            assert_eq!(k.hash, row.0, "{}", k.stem());
            assert_eq!(k.region, row.1);
            assert_eq!(k.version, row.2);
            assert_eq!(k.date, row.3);
            assert_eq!(k.slug, row.4);
            assert_eq!(k.name, row.5);
            assert_eq!(k.stem(), row.6);
            assert_eq!(k.display_name(), row.7);
            assert_eq!(
                tg18::roms::identify_hash(row.0)
                    .map(|r| r.stem())
                    .as_deref(),
                Some(row.6)
            );
        }
    }

    #[test]
    fn license_stays_mit_and_no_rom_is_shipped() {
        let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let lic =
            std::fs::read_to_string(repo.join("third_party/nijiiro-egg/LICENSE")).expect("LICENSE");
        assert!(lic.contains("MIT License"));
        assert!(lic.contains("Copyright (c) 2026 oreo-fox"));
        let mut shipped = Vec::new();
        fn walk(dir: &std::path::Path, shipped: &mut Vec<std::path::PathBuf>) {
            let Ok(rd) = std::fs::read_dir(dir) else {
                return;
            };
            for ent in rd.flatten() {
                let name = ent.file_name().to_string_lossy().to_string();
                if name == "target"
                    || name == ".git"
                    || name == ".opencode"
                    || name == "out"
                    || name == "node_modules"
                {
                    continue;
                }
                let path = ent.path();
                if path.is_dir() {
                    walk(&path, shipped);
                } else {
                    let lower = name.to_ascii_lowercase();
                    if lower.ends_with(".bin") || lower.ends_with(".t18s") {
                        shipped.push(path);
                    }
                }
            }
        }
        walk(&repo, &mut shipped);
        assert!(
            shipped.is_empty(),
            "firmware or saves must not be committed: {shipped:?}"
        );
    }
}
