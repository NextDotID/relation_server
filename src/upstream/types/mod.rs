pub(crate) mod target;
pub(crate) mod platform;
pub(crate) mod data_source;
pub(crate) mod data_fetcher;

use serde::{Serialize, Deserialize};

pub use target::{Target, TargetProcessedList};
pub use platform::Platform;
pub use data_source::DataSource;
pub use data_fetcher::DataFetcher;

/// All asymmetric cryptography algorithm supported by RelationService.
#[derive(Serialize, Deserialize)]
pub enum Algorithm {
    EllipticCurve,
}

/// All elliptic curve supported by RelationService.
#[derive(Serialize, Deserialize)]
pub enum Curve {
    Secp256K1,
}
