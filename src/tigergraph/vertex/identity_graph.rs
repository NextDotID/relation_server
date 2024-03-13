use crate::{
    config::C,
    error::Error,
    tigergraph::{vertex::IdentityRecord, BaseResponse, Graph},
    upstream::{DataSource, Platform},
    util::parse_body,
};
use http::uri::InvalidUri;
use hyper::{client::HttpConnector, Body, Client, Method};
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConnection {
    pub edge_type: String,
    pub data_source: DataSource,
    #[serde(rename = "source_v")]
    pub source: String,
    #[serde(rename = "target_v")]
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityGraph {
    pub graph_id: String,
    pub vertices: Vec<IdentityRecord>,
    pub edges: Vec<IdentityConnection>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct IdentityGraphResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<IdentityGraph>>,
}

impl IdentityGraph {
    pub async fn find_by_platform_identity(
        client: &Client<HttpConnector>,
        platform: &Platform,
        identity: &str,
        reverse: Option<bool>,
    ) -> Result<Option<IdentityGraph>, Error> {
        // This reverse flag can be used as a filtering for Identity which type is domain system .
        // flag = 0, If `reverse=None` if omitted, there is no need to filter anything.
        // flag = 1, When `reverse=true`, just return `primary domain` related identities.
        // flag = 2, When `reverse=false`, Only `non-primary domain` will be returned, which is the inverse set of reverse=true.
        let flag = reverse.map_or(0, |r| match r {
            true => 1,
            false => 2,
        });
        let p = format!("{},{}", platform, identity);
        let encoded_p = urlencoding::encode(&p);
        let uri: http::Uri = format!(
            "{}/query/{}/find_identity_graph?p={}&reverse_flag={}",
            C.tdb.host,
            Graph::SocialGraph.to_string(),
            encoded_p,
            flag,
        )
        .parse()
        .map_err(|_err: InvalidUri| {
            Error::ParamError(format!(
                "query find_identity_graph?p={}&reverse_flag={} Uri format Error | {}",
                p, flag, _err
            ))
        })?;

        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", Graph::SocialGraph.token())
            .body(Body::empty())
            .map_err(|_err| {
                Error::ParamError(format!(
                    "query find_identity_graph ParamError Error {}",
                    _err
                ))
            })?;

        let mut resp = client.request(req).await.map_err(|err| {
            Error::ManualHttpClientError(format!(
                "query find_identity_graph | Fail to request: {:?}",
                err.to_string()
            ))
        })?;

        match parse_body::<IdentityGraphResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query find_identity_graph error | Code: {:?}, Message: {:?}",
                        r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }

                let result = r.results.and_then(|vec_res| vec_res.first().cloned());
                match result {
                    None => return Ok(None),
                    Some(result) => {
                        if result.graph_id == "" {
                            return Ok(None);
                        } else if result.edges.len() == 0 {
                            return Ok(None);
                        } else {
                            // filter out dataSource == "keybase" edges
                            let filter_edges: Vec<IdentityConnection> = result
                                .edges
                                .clone()
                                .into_iter()
                                .filter(|e| e.source != DataSource::Keybase.to_string())
                                .collect();
                            if filter_edges.len() == 0 {
                                return Ok(None);
                            }
                        }
                        return Ok(Some(result));
                    }
                }
            }
            Err(err) => {
                let err_message = format!(
                    "TigerGraph query find_identity_graph parse_body error: {:?}",
                    err
                );
                error!(err_message);
                return Err(err);
            }
        }
    }
}
