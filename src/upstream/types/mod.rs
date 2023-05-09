pub(crate) mod data_fetcher;
pub(crate) mod data_source;
pub(crate) mod platform;
pub(crate) mod target;

use serde::{Deserialize, Serialize};

pub use data_fetcher::DataFetcher;
pub use data_source::vec_string_to_vec_datasource;
pub use data_source::DataSource;
pub use platform::Platform;
pub use target::{Target, TargetProcessedList};

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
