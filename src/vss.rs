pub mod dh;
pub mod dispute;
pub mod feldman;
pub mod pvss;
//pub mod sh;

use crate::*;
pub use dh::*;
pub use dispute::*;
pub use feldman::*;
pub use pvss::*;

/// The possible States of a VSS instance
#[derive(Clone, Debug)]
pub enum VSSState<Affine: AffineCurve> {
    /// The VSS is currently in a Sharing state with weight_ready
    /// of participants signaling Ready for this VSS
    Sharing { weight_ready: u32 },
    /// The VSS has completed Successfully with final secret commitment g^{\phi(0)}
    Success { final_secret: Affine },
    /// The VSS has ended in Failure
    Failure,
}