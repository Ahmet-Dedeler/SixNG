//! sixng-core: GTA VI (RPF9, Xbox Series) asset extraction core.
pub mod audio;
pub mod bounds;
pub mod classify;
pub mod navmesh;
pub mod lights;
pub mod paths;
pub mod batches;
pub mod drawable;
pub mod export;
pub mod game;
pub mod gltf;
pub mod meta7a;
pub mod names;
pub mod material;
pub mod gfx10;
pub mod resource;
pub mod texture;
pub mod ttpf;
pub mod gaps;
pub mod lodlinks;
pub mod rpf9;
pub mod scan;
pub mod surface;
pub mod world;

pub const INDEX_VERSION: &str = "4";
