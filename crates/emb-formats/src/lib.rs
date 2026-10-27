//! Machine embroidery formats: VP3, DST, PES, PEC, EXP, JEF, XXX, TBF, TAP
//! and U01 (read and write); SEW, HUS, SHV, DSB, DSZ, 10O, 100, DAT, EXY,
//! FXY, GT, STX, PHB, PHC, BRO, EMD, INB, JPX, KSM, MAX, MIT, NEW, PCD, PCQ,
//! PCS, PCM, SPX, STC, ZHS and ZXY (read only). Thread color files (COL,
//! EDR, INF) are in [`colors`].
//!
//! Each format is one module with two functions:
//! `read(bytes) -> StitchDesign` and `write(&StitchDesign) -> bytes`.
//! Convert from mm to the format unit only in this crate.
//!
//! The formats follow pyembroidery (MIT, commit c16d1b4) where it has them:
//! each module names its source file in its header. The others follow public
//! descriptions (EMF, WMF, PDF) or are our own (artwork, metadata).
//! The native project format is in `emb_model::project`, not here.
#![forbid(unsafe_code)]

pub mod artwork;
pub mod barudan;
pub mod colors;
mod common;
pub mod cutter;
pub mod dsb;
pub mod dst;
pub mod dsz;
pub mod emf;
pub mod exp;
pub mod hus;
pub mod jef;
mod meta;
pub mod minor;
pub mod pdf;
pub mod pes;
pub mod sew;
pub mod shv;
pub mod stones;
pub mod svg;
mod tables;
pub mod tap;
pub mod tbf;
pub mod toyota;
pub mod vp3;
pub mod wmf;
pub mod xxx;

use emb_model::StitchDesign;

/// Machine formats that can be read, by file extension.
pub const READ_EXTENSIONS: [&str; 40] = [
    "dst", "vp3", "pes", "pec", "exp", "jef", "xxx", "sew", "hus", "shv", "tbf", "tap", "dsb", "dsz", "10o", "100",
    "u01", "dat", "exy", "fxy", "gt", "stx", "phb", "phc", "bro", "emd", "inb", "jpx", "ksm", "max", "mit", "new",
    "pcd", "pcq", "pcs", "pcm", "spx", "stc", "zhs", "zxy",
];

/// Machine formats that can be written, by file extension.
pub const WRITE_EXTENSIONS: [&str; 10] = ["dst", "vp3", "pes", "pec", "exp", "jef", "xxx", "tbf", "tap", "u01"];

/// Reads a machine file. `ext` is the lower-case file extension.
pub fn read_machine(ext: &str, bytes: &[u8]) -> Result<StitchDesign, String> {
    match ext {
        "dst" => dst::read(bytes).map(|f| f.design).map_err(|e| e.to_string()),
        "vp3" => vp3::read(bytes).map_err(|e| e.to_string()),
        "pes" | "pec" => pes::read(bytes).map_err(|e| e.to_string()),
        "exp" => Ok(exp::read(bytes)),
        "jef" => jef::read(bytes).map_err(|e| e.to_string()),
        "xxx" => Ok(xxx::read(bytes)),
        "sew" => Ok(sew::read(bytes)),
        "hus" => hus::read(bytes).map_err(|e| e.to_string()),
        "shv" => Ok(shv::read(bytes)),
        "tbf" => tbf::read(bytes).map_err(|e| e.to_string()),
        "tap" => Ok(tap::read(bytes)),
        "dsb" => dsb::read(bytes).map_err(|e| e.to_string()),
        "dsz" => dsz::read(bytes).map_err(|e| e.to_string()),
        "10o" => Ok(toyota::read_10o(bytes)),
        "100" => Ok(toyota::read_100(bytes)),
        "u01" => Ok(barudan::read(bytes)),
        "dat" => Ok(barudan::read_dat(bytes)),
        "exy" => dst::read_exy(bytes).map_err(|e| e.to_string()),
        "fxy" => dsz::read_fxy(bytes).map_err(|e| e.to_string()),
        "gt" => dsz::read_gt(bytes).map_err(|e| e.to_string()),
        "stx" => Ok(exp::read_stx(bytes)),
        "phb" => pes::read_phb(bytes).map_err(|e| e.to_string()),
        "phc" => pes::read_phc(bytes).map_err(|e| e.to_string()),
        "bro" => Ok(minor::read_bro(bytes)),
        "emd" => Ok(minor::read_emd(bytes)),
        "inb" => Ok(minor::read_inb(bytes)),
        "jpx" => Ok(minor::read_jpx(bytes)),
        "ksm" => Ok(minor::read_ksm(bytes)),
        "max" => Ok(minor::read_max(bytes)),
        "mit" => Ok(minor::read_mit(bytes)),
        "new" => Ok(minor::read_new(bytes)),
        "pcd" | "pcq" | "pcs" => Ok(minor::read_pcs(bytes)),
        "pcm" => Ok(minor::read_pcm(bytes)),
        "spx" => Ok(minor::read_spx(bytes)),
        "stc" => Ok(minor::read_stc(bytes)),
        "zhs" => Ok(minor::read_zhs(bytes)),
        "zxy" => Ok(minor::read_zxy(bytes)),
        _ => Err(format!("unknown machine format .{ext}")),
    }
}

/// Writes a machine file. `name` goes in the header where the format has a
/// name (DST, PES, TBF); `date` (`YYYYMMDDhhmmss`) where it has a date (JEF).
/// `None` if the format cannot be written.
pub fn write_machine(ext: &str, design: &StitchDesign, name: &str, date: &str) -> Option<Vec<u8>> {
    write_machine_with(ext, design, name, date, None, None)
}

/// The needle of each color block of a machine file that has needle codes
/// (TBF and U01), else an empty list. `ext` is the lower-case extension.
pub fn read_needles(ext: &str, bytes: &[u8]) -> Vec<u8> {
    match ext {
        "tbf" => tbf::read_needles(bytes),
        "u01" => barudan::read_needles(bytes),
        _ => Vec::new(),
    }
}

/// As [`write_machine`], with the encoding settings of a machine profile
/// (the trim code of DST and TAP, P10 Milestone 6) and the needle plan of a
/// multi-needle machine (ADR 0063). TBF and U01 write the needle of each
/// color block and a stop at each thread change; the other formats have no
/// needle codes.
pub fn write_machine_with(
    ext: &str,
    design: &StitchDesign,
    name: &str,
    date: &str,
    machine: Option<&emb_model::MachineProfile>,
    needles: Option<&emb_model::NeedlePlan>,
) -> Option<Vec<u8>> {
    let trim = machine.map(dst::TrimCode::for_machine).unwrap_or_default();
    let with_needles = |write: &dyn Fn(&StitchDesign, &[u8]) -> Vec<u8>| match needles {
        Some(p) => write(&p.with_stops(design), &p.blocks),
        None => write(design, &[]),
    };
    Some(match ext {
        "dst" => dst::write_with(design, name, trim),
        "vp3" => vp3::write(design),
        "pes" => pes::write(design, name),
        "exp" => exp::write(design),
        "jef" => jef::write(design, date),
        "xxx" => xxx::write(design),
        "tbf" => with_needles(&|d, n| tbf::write_with(d, name, n)),
        "tap" => tap::write_with(design, trim),
        "pec" => pes::write_pec_file(design, name),
        "u01" => with_needles(&barudan::write_with),
        _ => return None,
    })
}

#[cfg(test)]
mod lib_tests {
    use super::*;

    #[test]
    fn needle_plans_go_in_tbf_and_u01() {
        let d = common::tests::sample();
        let blocks = emb_model::needles::block_threads(&d);
        // Both threads pinned to needle 4: the second is a thread change.
        let pins: Vec<emb_model::NeedlePin> = d
            .threads
            .iter()
            .map(|t| emb_model::NeedlePin {
                thread: t.id,
                needle: 4,
            })
            .collect();
        let plan = emb_model::needles::assign(&blocks, 6, &Default::default(), &pins);
        assert_eq!(plan.blocks, [4, 4]);
        assert_eq!(plan.changes, [1]);
        for ext in ["tbf", "u01"] {
            let bytes = write_machine_with(ext, &d, "t", "", None, Some(&plan)).unwrap();
            assert_eq!(read_needles(ext, &bytes), [4, 4], "{ext}");
            let back = read_machine(ext, &bytes).unwrap();
            let stops = back
                .records
                .iter()
                .filter(|r| r.kind == emb_model::StitchKind::Stop)
                .count();
            assert_eq!(stops, 1, "{ext}: a stop at the thread change");
        }
        // DST has no needle codes: the plan changes nothing.
        let plain = write_machine("dst", &d, "t", "").unwrap();
        assert_eq!(
            write_machine_with("dst", &d, "t", "", None, Some(&plan)).unwrap(),
            plain
        );
    }

    #[test]
    fn machine_profiles_name_formats_of_this_crate() {
        for m in emb_model::MachineCatalog::builtin().machines {
            assert!(
                WRITE_EXTENSIONS.contains(&m.default_format.as_str()),
                "{}: the default format must be writable",
                m.id
            );
            for f in &m.formats {
                // VIP waits for a public reference (see the spec).
                assert!(READ_EXTENSIONS.contains(&f.as_str()) || f == "vip", "{}: {f}", m.id);
            }
        }
    }

    #[test]
    fn every_writer_reads_back() {
        let d = common::tests::sample();
        for ext in WRITE_EXTENSIONS {
            let bytes = write_machine(ext, &d, "t", "20000101000000").unwrap();
            let back = read_machine(ext, &bytes).unwrap();
            let n = |s: &StitchDesign| {
                s.records
                    .iter()
                    .filter(|r| r.kind == emb_model::StitchKind::Stitch)
                    .count()
            };
            assert_eq!(n(&back), n(&d), "{ext}");
        }
        assert!(write_machine("hus", &d, "t", "").is_none());
        assert!(read_needles("dst", &[]).is_empty());
        assert!(write_machine("10o", &d, "t", "").is_none());
        assert!(read_machine("abc", &[]).is_err());
    }
}
