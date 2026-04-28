use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct GqlResponse<T> {
    pub data: T,
}

#[derive(Deserialize, Debug)]
pub struct ChannelFollowsData {
    pub user: Option<User>,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub follows: Option<FollowConnection>,
}

#[derive(Deserialize, Debug)]
pub struct FollowConnection {
    pub edges: Vec<FollowEdge>,
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
}

#[derive(Deserialize, Debug)]
pub struct FollowEdge {
    pub cursor: String,
    pub node: Channel,
}

#[derive(Deserialize, Debug)]
pub struct Channel {
    pub login: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub follower_count: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct ChannelAvatarData {
    pub user: Option<ChannelAvatarUser>,
}

#[derive(Deserialize, Debug)]
pub struct ChannelAvatarUser {
    pub followers: FollowerCount,
}

#[derive(Deserialize, Debug)]
pub struct FollowerCount {
    #[serde(rename = "totalCount")]
    pub total_count: u32,
}

#[derive(Deserialize, Debug)]
pub struct PageInfo {
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
}
