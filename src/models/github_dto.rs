use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GithubGqlResponse {
    pub data: Option<GithubGraphQLUser>,
    pub errors: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubGraphQLUser {
    pub user: Option<GithubUserNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubUserNode {
    pub contributions_collection: GithubContributionsCollection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubContributionsCollection {
    pub contribution_calendar: GithubContributionCalendar,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubContributionCalendar {
    pub total_contributions: u32,
    pub weeks: Vec<GithubWeek>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubWeek {
    pub contribution_days: Vec<GithubContributionDay>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubContributionDay {
    pub date: String,
    pub weekday: u8, // 0 = Sunday
    pub contribution_count: u32,
    pub contribution_level: String,
}
