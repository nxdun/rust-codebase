use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionsResponse {
    pub username: String,
    pub range: ContributionRange,
    pub summary: ContributionSummary,
    pub legend: Vec<ContributionLegend>,
    pub months: Vec<ContributionMonth>,
    pub cells: Vec<ContributionCell>,
    pub meta: ContributionMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionRange {
    pub from: String,
    pub to: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionSummary {
    #[serde(rename = "totalContributions")]
    pub total_contributions: u32,
    #[serde(rename = "totalWeeks")]
    pub total_weeks: u32,
    #[serde(rename = "maxDailyCount")]
    pub max_daily_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionLegend {
    pub level: u32,
    pub label: Cow<'static, str>,
    pub min: u32,
    pub max: u32,
    pub color: Cow<'static, str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionMonth {
    pub label: Cow<'static, str>,
    #[serde(rename = "weekIndex")]
    pub week_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionCell {
    pub date: String,
    #[serde(rename = "weekIndex")]
    pub week_index: usize,
    pub weekday: u8,
    #[serde(rename = "weekdayLabel")]
    pub weekday_label: Cow<'static, str>,
    pub count: u32,
    pub level: u32,
    pub color: Cow<'static, str>,
    #[serde(rename = "isFuture")]
    pub is_future: bool,
    #[serde(rename = "isInCurrentMonth")]
    pub is_in_current_month: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionMeta {
    pub provider: String,
    pub cached: bool,
    #[serde(rename = "cacheTtlSeconds")]
    pub cache_ttl_seconds: u32,
    #[serde(rename = "fetchedAt")]
    pub fetched_at: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
}
