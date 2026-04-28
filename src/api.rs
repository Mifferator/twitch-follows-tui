use std::sync::mpsc;
use serde::Serialize;

use crate::app::Status;
use crate::models::{Channel, ChannelAvatarData, ChannelFollowsData, FollowedAtData, GqlResponse};

const TWITCH_GQL: &str = "https://gql.twitch.tv/gql";
const TWITCH_INTEGRITY: &str = "https://gql.twitch.tv/integrity";
const CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
const ORIGIN: &str = "https://www.twitch.tv";

#[derive(Serialize)]
struct RawGqlRequest {
    query: String,
}

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
}

#[derive(Serialize)]
struct ChannelAvatarVars<'a> {
    #[serde(rename = "channelLogin")]
    channel_login: &'a str,
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
        .header("User-Agent", USER_AGENT)
        .header("Origin", ORIGIN)
        .header("Referer", ORIGIN)
        .send()
        .await?
        .json()
        .await?;

    resp["token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no token in integrity response: {resp}"))
}

pub async fn fetch_follows(client: &reqwest::Client, login: &str, tx: mpsc::Sender<Status>) {
    if let Err(e) = fetch_follows_inner(client, login, &tx).await {
        tx.send(Status::Error(e)).ok();
    }
}

async fn fetch_follows_inner(
    client: &reqwest::Client,
    login: &str,
    tx: &mpsc::Sender<Status>,
) -> anyhow::Result<()> {
    let device_id = random_device_id();
    let integrity_token = fetch_integrity_token(client, &device_id).await?;

    let body = vec![GqlRequest {
        operation_name: "ChannelFollows",
        variables: ChannelFollowsVars {
            login,
            limit: 100,
            order: "DESC",
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
        .header("User-Agent", USER_AGENT)
        .header("Origin", ORIGIN)
        .header("Referer", ORIGIN)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    let mut channels: Vec<Channel> = resp.remove(0).data.user
        .and_then(|u| u.follows)
        .map(|f| f.edges.into_iter().map(|e| e.node).collect())
        .unwrap_or_default();

    tx.send(Status::LoadingDetails).ok();
    fetch_follower_counts(client, &integrity_token, &device_id, &mut channels).await?;

    tx.send(Status::LoadingDates).ok();
    fetch_followed_at(client, &integrity_token, &device_id, login, &mut channels).await?;

    tx.send(Status::LoadingMutuals).ok();
    fetch_mutuals(client, &integrity_token, &device_id, login, &mut channels).await?;

    tx.send(Status::Loaded(channels)).ok();
    Ok(())
}

async fn fetch_follower_counts(
    client: &reqwest::Client,
    integrity_token: &str,
    device_id: &str,
    channels: &mut Vec<Channel>,
) -> anyhow::Result<()> {
    for chunk in channels.chunks_mut(35) {
        let body: Vec<_> = chunk.iter().map(|c| GqlRequest {
            operation_name: "ChannelAvatar",
            variables: ChannelAvatarVars { channel_login: &c.login },
            extensions: Extensions {
                persisted_query: PersistedQuery {
                    version: 1,
                    sha256_hash: "db0e7b54c5e75fcf7874cafca2dacde646344cbbd1a80a2488a7953176c87a68",
                },
            },
        }).collect();

        let resp: Vec<GqlResponse<ChannelAvatarData>> = client
            .post(TWITCH_GQL)
            .header("Client-Id", CLIENT_ID)
            .header("Client-Integrity", integrity_token)
            .header("X-Device-Id", device_id)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        for (channel, r) in chunk.iter_mut().zip(resp) {
            channel.follower_count = r.data.user.map(|u| u.followers.total_count);
        }
    }

    Ok(())
}

async fn fetch_followed_at(
    client: &reqwest::Client,
    integrity_token: &str,
    device_id: &str,
    login: &str,
    channels: &mut Vec<Channel>,
) -> anyhow::Result<()> {
    for chunk in channels.chunks_mut(35) {
        let body: Vec<_> = chunk.iter().map(|c| RawGqlRequest {
            query: format!(
                r#"{{ user(login: "{}") {{ follow(targetLogin: "{}") {{ followedAt }} }} }}"#,
                login, c.login
            ),
        }).collect();

        let resp: Vec<GqlResponse<FollowedAtData>> = client
            .post(TWITCH_GQL)
            .header("Client-Id", CLIENT_ID)
            .header("Client-Integrity", integrity_token)
            .header("X-Device-Id", device_id)
            .header("User-Agent", USER_AGENT)
            .header("Origin", ORIGIN)
            .header("Referer", ORIGIN)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        for (channel, r) in chunk.iter_mut().zip(resp) {
            channel.followed_at = r.data.user
                .and_then(|u| u.follow)
                .map(|f| f.followed_at);
        }
    }

    Ok(())
}

async fn fetch_mutuals(
    client: &reqwest::Client,
    integrity_token: &str,
    device_id: &str,
    login: &str,
    channels: &mut Vec<Channel>,
) -> anyhow::Result<()> {
    for chunk in channels.chunks_mut(35) {
        let body: Vec<_> = chunk.iter().map(|c| RawGqlRequest {
            query: format!(
                r#"{{ user(login: "{}") {{ follow(targetLogin: "{}") {{ followedAt }} }} }}"#,
                c.login, login
            ),
        }).collect();

        let resp: Vec<GqlResponse<FollowedAtData>> = client
            .post(TWITCH_GQL)
            .header("Client-Id", CLIENT_ID)
            .header("Client-Integrity", integrity_token)
            .header("X-Device-Id", device_id)
            .header("User-Agent", USER_AGENT)
            .header("Origin", ORIGIN)
            .header("Referer", ORIGIN)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        for (channel, r) in chunk.iter_mut().zip(resp) {
            channel.is_mutual = r.data.user.and_then(|u| u.follow).is_some();
        }
    }

    Ok(())
}
