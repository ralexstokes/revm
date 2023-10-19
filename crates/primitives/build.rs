#![allow(dead_code, unused_imports)]

use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};
use std::slice;

const BYTES_PER_G1_POINT: usize = 48;
const BYTES_PER_G2_POINT: usize = 96;

fn main() {
    generate_kzg_settings();
}

fn generate_kzg_settings() {
    // Note: we don't use `OUT_DIR` because we don't ship this build script with the crate, so all
    // used files must be in tree.
    // let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let out_dir = Path::new("src/kzg/");
    let out_path = out_dir.join("generated.rs");

    let in_path = Path::new("src/kzg/trusted_setup.txt");
    // println!("cargo:rerun-if-changed={}", in_path.display());
    assert!(in_path.exists());
    let contents = format_kzg_settings(in_path, out_dir);
    fs::write(out_path, contents).unwrap();
}

/// Pros over `include_str!("trusted_setup.txt")`:
/// - partially decoded (hex strings -> point bytes)
/// - smaller runtime static size (198K = `4096*48 + 65*96` vs 404K)
/// - don't have to do weird hacks to call `load_trusted_setup_file` at runtime, see
///   [Reth](https://github.com/paradigmxyz/reth/blob/b839e394a45edbe7b2030fb370420ca771e5b728/crates/primitives/src/constants/eip4844.rs#L44-L52)
fn format_kzg_settings(in_path: &Path, out_dir: &Path) -> String {
    let contents = fs::read_to_string(in_path).unwrap();
    let mut lines = contents.lines();

    // load number of points
    let n_g1 = lines.next().unwrap().parse::<usize>().unwrap();
    let n_g2 = lines.next().unwrap().parse::<usize>().unwrap();

    assert_eq!(n_g2, 65);

    // load g1 points
    let mut g1_points = Vec::with_capacity(n_g1);
    for _ in 0..n_g1 {
        let line = lines.next().unwrap();
        let mut bytes = [0; BYTES_PER_G1_POINT];
        hex::decode_to_slice(line, &mut bytes).unwrap();
        g1_points.push(bytes);
    }

    // load g2 points
    let mut g2_points = Vec::with_capacity(n_g2);
    for _ in 0..n_g2 {
        let line = lines.next().unwrap();
        let mut bytes = [0; BYTES_PER_G2_POINT];
        hex::decode_to_slice(line, &mut bytes).unwrap();
        g2_points.push(bytes);
    }

    assert!(lines.next().is_none());

    fs::write(out_dir.join("g1_points.bin"), into_flattened(g1_points)).unwrap();
    fs::write(out_dir.join("g2_points.bin"), into_flattened(g2_points)).unwrap();

    format!(
        r#"// @generated by build.rs from {in_path:?}, do not modify manually.

pub use c_kzg::{{BYTES_PER_G1_POINT, BYTES_PER_G2_POINT}};

// Ensure that the build script constants are synced with the C bindings ones.
const _: [(); BYTES_PER_G1_POINT] = [(); {BYTES_PER_G1_POINT}];
const _: [(); BYTES_PER_G2_POINT] = [(); {BYTES_PER_G2_POINT}];

pub const NUM_G1_POINTS: usize = {n_g1};
pub const NUM_G2_POINTS: usize = {n_g2};

type G1Points = [[u8; BYTES_PER_G1_POINT]; NUM_G1_POINTS]; 
type G2Points = [[u8; BYTES_PER_G2_POINT]; NUM_G2_POINTS]; 

pub const G1_POINTS: &G1Points = {{
    const BYTES: &[u8] = include_bytes!("./g1_points.bin");
    assert!(BYTES.len() == core::mem::size_of::<G1Points>());
    unsafe {{ &*BYTES.as_ptr().cast::<G1Points>() }}
}};
pub const G2_POINTS: &G2Points = {{
    const BYTES: &[u8] = include_bytes!("./g2_points.bin");
    assert!(BYTES.len() == core::mem::size_of::<G2Points>());
    unsafe {{ &*BYTES.as_ptr().cast::<G2Points>() }}
}};
"#
    )
}

/// [`Vec::into_flattened`].
#[inline]
fn into_flattened<T, const N: usize>(vec: Vec<[T; N]>) -> Vec<T> {
    let (ptr, len, cap) = into_raw_parts(vec);
    let (new_len, new_cap) = if core::mem::size_of::<T>() == 0 {
        (len.checked_mul(N).expect("vec len overflow"), usize::MAX)
    } else {
        // SAFETY:
        // - `cap * N` cannot overflow because the allocation is already in
        // the address space.
        // - Each `[T; N]` has `N` valid elements, so there are `len * N`
        // valid elements in the allocation.
        unsafe {
            (
                len.checked_mul(N).unwrap_unchecked(),
                cap.checked_mul(N).unwrap_unchecked(),
            )
        }
    };
    // SAFETY:
    // - `ptr` was allocated by `self`
    // - `ptr` is well-aligned because `[T; N]` has the same alignment as `T`.
    // - `new_cap` refers to the same sized allocation as `cap` because
    // `new_cap * size_of::<T>()` == `cap * size_of::<[T; N]>()`
    // - `len` <= `cap`, so `len * N` <= `cap * N`.
    unsafe { Vec::from_raw_parts(ptr.cast(), new_len, new_cap) }
}

/// [`Vec::into_raw_parts`]
#[inline(always)]
fn into_raw_parts<T>(vec: Vec<T>) -> (*mut T, usize, usize) {
    let mut me = core::mem::ManuallyDrop::new(vec);
    (me.as_mut_ptr(), me.len(), me.capacity())
}
