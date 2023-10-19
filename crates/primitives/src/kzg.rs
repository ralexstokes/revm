mod env_settings;

mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}

pub use c_kzg::KzgSettings;
pub use env_settings::EnvKzgSettings;
pub use generated::{
    BYTES_PER_G1_POINT, BYTES_PER_G2_POINT, G1_POINTS, G2_POINTS, NUM_G1_POINTS, NUM_G2_POINTS,
};
