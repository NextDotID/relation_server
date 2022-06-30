mod tests;

use crate::config::C;
use crate::error::Error;
use crate::graph::{edge::Proof, new_db_connection, vertex::Identity};
use crate::graph::{Edge, Vertex};
use crate::upstream::{DataSource, Fetcher, Platform};
use crate::util::{make_client, naive_now, parse_body};
use async_trait::async_trait;
use serde::Deserialize;

use std::str::FromStr;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct KeybaseResponse {
    pub status: Status,
    pub them: Vec<PersonInfo>,
}

#[derive(Deserialize, Debug)]
pub struct PersonInfo {
    pub id: String,
    pub basics: Basics,
    pub proofs_summary: ProofsSummary,
}

#[derive(Deserialize, Debug)]
pub struct Status {
    pub code: i32,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct ProofsSummary {
    pub all: Vec<ProofItem>,
}

#[derive(Deserialize, Debug)]
pub struct Basics {
    pub username: String,
    pub ctime: i64,
    pub mtime: i64,
    pub id_version: i32,
    pub track_version: i32,
    pub last_id_change: i64,
    pub username_cased: String,
    pub status: i32,
    pub salt: String,
    pub eldest_seqno: i32,
}

#[derive(Deserialize, Debug)]
pub struct ProofItem {
    pub proof_type: String,
    pub nametag: String,
    pub state: i32,
    pub service_url: String,
    pub proof_url: String,
    pub sig_id: String,
    pub proof_id: String,
    pub human_url: String,
    pub presentation_group: String,
    pub presentation_tag: String,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}

pub struct Keybase {
    pub platform: String,
    pub identity: String,
}

#[async_trait]
impl Fetcher for Keybase {
    async fn fetch(&self) -> Result<(), Error> {
        let client = make_client();
        let uri: http::Uri = match format!(
            "{}?{}={}&fields=proofs_summary",
            C.upstream.keybase_service.url, self.platform, self.identity
        )
        .parse()
        {
            Ok(n) => n,
            Err(err) => {
                return Err(Error::ParamError(format!(
                    "Uri format Error: {}",
                    err.to_string()
                )))
            }
        };

        let mut resp = client.get(uri).await?;

        if !resp.status().is_success() {
            let body: ErrorResponse = parse_body(&mut resp).await?;
            return Err(Error::General(
                format!("Keybase Result Get Error: {}", body.message),
                resp.status(),
            ));
        }

        let mut body: KeybaseResponse = parse_body(&mut resp).await?;
        if body.status.code != 0 {
            return Err(Error::General(
                format!("Keybase Result Get Error: {}", body.status.name),
                resp.status(),
            ));
        }

        let person_info = match body.them.pop() {
            Some(i) => i,
            None => {
                return Err(Error::NoResult);
            }
        };
        let user_id = person_info.id;
        let user_name = person_info.basics.username;

        let db = new_db_connection().await?;

        for p in person_info.proofs_summary.all.into_iter() {
            let from: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Keybase,
                identity: user_id.clone(),
                created_at: None,
                display_name: user_name.clone(),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
            };
            let from_record = from.create_or_update(&db).await?;

            if Platform::from_str(p.proof_type.as_str()).is_err() {
                continue;
            }
            let to: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::from_str(p.proof_type.as_str()).unwrap(),
                identity: p.nametag.clone(),
                created_at: None,
                display_name: p.nametag.clone(),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
            };
            let to_record = to.create_or_update(&db).await?;

            let pf: Proof = Proof {
                uuid: Uuid::new_v4(),
                source: DataSource::Keybase,
                record_id: Some(p.proof_id.clone()),
                created_at: None,
                last_fetched_at: naive_now(),
            };
            pf.connect(&db, &from_record, &to_record).await?;
        }

        Ok(())
    }

    fn ability(&self) -> Vec<(Vec<Platform>, Vec<Platform>)> {
        return vec![(
            vec![Platform::Twitter, Platform::Github],
            vec![Platform::Keybase, Platform::Twitter, Platform::Github],
        )];
    }
}
