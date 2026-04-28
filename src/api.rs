use serde::Serialize;

use crate::models::{Channel, ChannelFollowsData, GqlResponse};

const TWITCH_GQL: &str = "https://gql.twitch.tv/gql";
const TWITCH_INTEGRITY: &str = "https://gql.twitch.tv/integrity";
const CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";

#[derive(Serialize)]
struct GqlRequest<V: Serialize> {
    #[serde(rename = "operationName")]
    operation_name: &'static str,
    variables: V,
    extensions: Extensions,
}

#[derive(Serialize)]
struct Extensions {
    #[serde(rename = "persistedQuery")]
    persisted_query: PersistedQuery,
}

#[derive(Serialize)]
struct PersistedQuery {
    version: u32,
    #[serde(rename = "sha256Hash")]
    sha256_hash: &'static str,
}

#[derive(Serialize)]
struct ChannelFollowsVars<'a> {
    login: &'a str,
    limit: u32,
    order: &'static str,
    cursor: Option<&'a str>,
}

fn random_device_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    format!("{:x}{:x}", seed, seed.wrapping_mul(0x9e3779b9))
}

async fn fetch_integrity_token(
    client: &reqwest::Client,
    device_id: &str,
) -> anyhow::Result<String> {
    let resp: serde_json::Value = client
        .post(TWITCH_INTEGRITY)
        .header("Client-Id", CLIENT_ID)
        .header("X-Device-Id", device_id)
        .send()
        .await?
        .json()
        .await?;

    resp["token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no token in integrity response: {resp}"))
}

pub async fn fetch_follows(
    client: &reqwest::Client,
    login: &str,
) -> anyhow::Result<Vec<Channel>> {
    let device_id = random_device_id();
    let integrity_token = fetch_integrity_token(client, &device_id).await?;

    let mut channels: Vec<Channel> = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let body = vec![GqlRequest {
            operation_name: "ChannelFollows",
            variables: ChannelFollowsVars {
                login,
                limit: 100,
                order: "DESC",
                cursor: cursor.as_deref(),
            },
            extensions: Extensions {
                persisted_query: PersistedQuery {
                    version: 1,
                    sha256_hash: "eecf815273d3d949e5cf0085cc5084cd8a1b5b7b6f7990cf43cb0beadf546907",
                },
            },
        }];

        let mut resp: Vec<GqlResponse<ChannelFollowsData>> = client
            .post(TWITCH_GQL)
            .header("Client-Id", CLIENT_ID)
            .header("Client-Integrity", &integrity_token)
            .header("X-Device-Id", &device_id)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        let follows = match resp.remove(0).data.user.and_then(|u| u.follows) {
            Some(f) => f,
            None => break,
        };

        let has_next = follows.page_info.has_next_page;
        cursor = follows.edges.last().map(|e| e.cursor.clone());
        channels.extend(follows.edges.into_iter().map(|e| e.node));

        if !has_next {
            break;
        }
    }

    Ok(channels)
}
