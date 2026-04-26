use serde::Deserialize;

/// Top-level GraphQL response containing data and errors from GitHub.
#[derive(Debug, Deserialize)]
pub struct GithubGqlResponse {
    pub data: Option<GithubGraphQLUser>,
    pub errors: Option<Vec<serde_json::Value>>,
}

/// Wrapper for the user node in the GraphQL response (camelCase mapped).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubGraphQLUser {
    pub user: Option<GithubUserNode>,
}

/// Contains the user's contributions collection.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubUserNode {
    pub contributions_collection: GithubContributionsCollection,
}

/// The contributions container holding the calendar.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubContributionsCollection {
    pub contribution_calendar: GithubContributionCalendar,
}

/// The contribution calendar containing total contributions and weekly buckets.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubContributionCalendar {
    pub total_contributions: u32,
    pub weeks: Vec<GithubWeek>,
}

/// A weekly bucket of contribution days.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubWeek {
    pub contribution_days: Vec<GithubContributionDay>,
}

/// A single-day record of contributions (date, weekday, count, level).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubContributionDay {
    pub date: String,
    pub weekday: u8, // 0 = Sunday
    pub contribution_count: u32,
    pub contribution_level: String,
}
