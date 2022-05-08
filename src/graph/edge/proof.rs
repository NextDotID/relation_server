use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Proof {
    pub uuid: String,
    pub method: String,
    pub upstream: String,
    pub record_id: String,
    pub created_at: u128,
    pub last_verified_at: u128,
}
