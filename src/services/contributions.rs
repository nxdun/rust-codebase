use dashmap::DashMap;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::info;

use crate::{
    error::AppError,
    models::contributions_model::{
        ContributionCell, ContributionLegend, ContributionMeta, ContributionMonth,
        ContributionRange, ContributionSummary, ContributionsResponse,
    },
};

const CACHE_TTL_SECONDS: u32 = 86400;
const CACHE_MAX_CAPACITY: u32 = 1000;
const SCHEMA_VERSION: u32 = 1;
const PROVIDER_GITHUB: &str = "github";
const USER_AGENT: &str = "nadzu-backend";

const WEEKDAY_LABELS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTH_LABELS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const CONTRIBUTION_COLORS: [&str; 5] = [
    "#2b2c3494", // Level 0
    "#9be9a8",   // Level 1
    "#40c463",   // Level 2
    "#30a14e",   // Level 3
    "#216e39",   // Level 4
];

#[derive(Debug, Deserialize)]
struct GithubGqlResponse {
    data: Option<GithubGraphQLUser>,
    errors: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GithubGraphQLUser {
    user: Option<GithubUserNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GithubUserNode {
    contributions_collection: GithubContributionsCollection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GithubContributionsCollection {
    contribution_calendar: GithubContributionCalendar,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GithubContributionCalendar {
    total_contributions: u32,
    weeks: Vec<GithubWeek>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GithubWeek {
    contribution_days: Vec<GithubContributionDay>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GithubContributionDay {
    date: String,
    weekday: u8, // 0 = Sunday
    contribution_count: u32,
    contribution_level: String,
}

pub struct ContributionsService {
    http_client: Client,
    pat: String,
    default_username: String,
    graphql_url: String,
    cache: Arc<DashMap<String, (ContributionsResponse, u64)>>,
}

impl std::fmt::Debug for ContributionsService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContributionsService")
            .field("default_username", &self.default_username)
            .field("cache_len", &self.cache.len())
            .field("graphql_url", &self.graphql_url)
            .finish_non_exhaustive()
    }
}

impl ContributionsService {
    pub fn new(
        http_client: Client,
        pat: String,
        default_username: String,
        graphql_url: String,
    ) -> Self {
        let cache = Arc::new(DashMap::new());
        let cache_weak = Arc::downgrade(&cache);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(600)); // Clean every 10 mins
            loop {
                interval.tick().await;
                if let Some(cache) = cache_weak.upgrade() {
                    let now = now_unix();
                    cache.retain(|_, (_, expires_at)| *expires_at > now);
                } else {
                    info!("ContributionsService dropped, stopping cleanup task");
                    break;
                }
            }
        });

        Self {
            http_client,
            pat,
            default_username,
            graphql_url,
            cache,
        }
    }

    pub fn seed_cache(&self, username: &str, response: ContributionsResponse, ttl_secs: u64) {
        let expires_at = now_unix() + ttl_secs;
        self.cache
            .insert(username.to_string(), (response, expires_at));
    }

    pub fn get_default_username(&self) -> &str {
        &self.default_username
    }

    pub async fn get_contributions(
        &self,
        username: &str,
    ) -> Result<ContributionsResponse, AppError> {
        let now = now_unix();
        let cache_key = username.to_string();

        if username.trim().is_empty() {
            return Err(AppError::Validation("Username cannot be empty".into()));
        }

        //not allow any other username other than default for now.
        if username != self.default_username {
            return Err(AppError::Validation(
                "Only the default username is allowed".into(),
            ));
        }

        if let Some(entry) = self.cache.get(&cache_key) {
            let (cached_resp, expires_at) = entry.value();
            if *expires_at > now {
                let mut resp = cached_resp.clone();
                resp.meta.cached = true;
                return Ok(resp);
            }
        }

        let resp_result = self.fetch_and_process(username, SystemTime::now()).await;

        match resp_result {
            Ok(new_resp) => {
                let expires_at = now + u64::from(CACHE_TTL_SECONDS);
                // Bounded: only insert if under capacity to prevent memory leaks
                if self.cache.len() < CACHE_MAX_CAPACITY as usize {
                    self.cache.insert(cache_key, (new_resp.clone(), expires_at));
                }
                Ok(new_resp)
            }
            Err(e) => {
                // Stale-cache fallback
                if let Some(entry) = self.cache.get(&cache_key) {
                    let mut resp = entry.value().0.clone();
                    resp.meta.cached = true;
                    return Ok(resp);
                }
                Err(e)
            }
        }
    }

    async fn fetch_and_process(
        &self,
        username: &str,
        fetched_at: SystemTime,
    ) -> Result<ContributionsResponse, AppError> {
        let query = r"
            query($username: String!) {
              user(login: $username) {
                contributionsCollection {
                  contributionCalendar {
                    totalContributions
                    weeks {
                      contributionDays {
                        date
                        weekday
                        contributionCount
                        contributionLevel
                      }
                    }
                  }
                }
              }
            }
        ";

        let payload = json!({
            "query": query,
            "variables": { "username": username }
        });

        let resp = self
            .http_client
            .post(&self.graphql_url)
            .bearer_auth(&self.pat)
            .header("User-Agent", USER_AGENT)
            .timeout(Duration::from_secs(30))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Network or timeout error: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::Internal(anyhow::anyhow!(
                "Upstream API returned status: {}",
                resp.status()
            )));
        }

        let gh_resp: GithubGqlResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("JSON parsing error: {e}")))?;

        if let Some(errs) = gh_resp.errors
            && !errs.is_empty()
        {
            return Err(AppError::Internal(anyhow::anyhow!(
                "GitHub GraphQL returned errors"
            )));
        }

        let calendar = gh_resp
            .data
            .and_then(|d| d.user)
            .map(|u| u.contributions_collection.contribution_calendar)
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("User not found or missing fields"))
            })?;

        Ok(Self::transform_calendar(username, &calendar, fetched_at))
    }

    #[allow(clippy::too_many_lines)]
    fn transform_calendar(
        username: &str,
        calendar: &GithubContributionCalendar,
        fetched_at: SystemTime,
    ) -> ContributionsResponse {
        let mut cells = Vec::new();
        let mut months = Vec::new();
        let mut max_daily_count = 0;

        let mut last_month: Option<u32> = None;

        let (current_year, current_month_num, current_day) = get_utc_date(SystemTime::now());
        let current_date_str = format!("{current_year:04}-{current_month_num:02}-{current_day:02}");

        let fetched_at_str = format_iso_time(fetched_at);

        for (week_idx, week) in calendar.weeks.iter().enumerate() {
            let mut month_added_this_week = false;

            for day in &week.contribution_days {
                if day.contribution_count > max_daily_count {
                    max_daily_count = day.contribution_count;
                }

                // Parse YYYY-MM-DD
                let (y_day, m_day, _) = parse_ymd(&day.date).unwrap_or((1970, 1, 1));

                if let Some(lm) = last_month {
                    if lm != m_day && !month_added_this_week {
                        months.push(ContributionMonth {
                            label: get_month_label(m_day),
                            week_index: week_idx,
                        });
                        month_added_this_week = true;
                    }
                } else if !month_added_this_week {
                    months.push(ContributionMonth {
                        label: get_month_label(m_day),
                        week_index: week_idx,
                    });
                    month_added_this_week = true;
                }
                last_month = Some(m_day);

                let is_future = day.date > current_date_str;
                let is_in_current_month = m_day == current_month_num && y_day == current_year;

                let level = match day.contribution_level.as_str() {
                    "FIRST_QUARTILE" => 1,
                    "SECOND_QUARTILE" => 2,
                    "THIRD_QUARTILE" => 3,
                    "FOURTH_QUARTILE" => 4,
                    _ => 0,
                };

                let weekday_label = WEEKDAY_LABELS
                    .get(day.weekday as usize)
                    .unwrap_or(&"")
                    .to_string();

                cells.push(ContributionCell {
                    date: day.date.clone(),
                    week_index: week_idx,
                    weekday: day.weekday,
                    weekday_label,
                    count: day.contribution_count,
                    level,
                    color: CONTRIBUTION_COLORS[level as usize].to_string(),
                    is_future,
                    is_in_current_month,
                });
            }
        }

        let total_weeks = u32::try_from(calendar.weeks.len()).unwrap_or_default();

        let mut level_mins = [u32::MAX; 5];
        let mut level_maxs = [0u32; 5];
        level_mins[0] = 0;
        level_maxs[0] = 0;

        for cell in &cells {
            let l = cell.level as usize;
            if l < 5 && l > 0 {
                if cell.count < level_mins[l] {
                    level_mins[l] = cell.count;
                }
                if cell.count > level_maxs[l] {
                    level_maxs[l] = cell.count;
                }
            }
        }

        for min_val in level_mins.iter_mut().skip(1) {
            if *min_val == u32::MAX {
                *min_val = 0;
            }
        }

        let legend = vec![
            ContributionLegend {
                level: 0,
                label: "No contributions".into(),
                min: level_mins[0],
                max: level_maxs[0],
                color: CONTRIBUTION_COLORS[0].to_string(),
            },
            ContributionLegend {
                level: 1,
                label: "Low".into(),
                min: level_mins[1],
                max: level_maxs[1],
                color: CONTRIBUTION_COLORS[1].to_string(),
            },
            ContributionLegend {
                level: 2,
                label: "Medium".into(),
                min: level_mins[2],
                max: level_maxs[2],
                color: CONTRIBUTION_COLORS[2].to_string(),
            },
            ContributionLegend {
                level: 3,
                label: "High".into(),
                min: level_mins[3],
                max: level_maxs[3],
                color: CONTRIBUTION_COLORS[3].to_string(),
            },
            ContributionLegend {
                level: 4,
                label: "Very high".into(),
                min: level_mins[4],
                max: level_maxs[4],
                color: CONTRIBUTION_COLORS[4].to_string(),
            },
        ];

        let from_date = cells.first().map(|c| c.date.clone()).unwrap_or_default();
        let to_date = cells.last().map(|c| c.date.clone()).unwrap_or_default();

        ContributionsResponse {
            username: username.to_string(),
            range: ContributionRange {
                from: from_date,
                to: to_date,
                timezone: "UTC".into(),
            },
            summary: ContributionSummary {
                total_contributions: calendar.total_contributions,
                total_weeks,
                max_daily_count,
            },
            legend,
            months,
            cells,
            meta: ContributionMeta {
                provider: PROVIDER_GITHUB.into(),
                cached: false,
                cache_ttl_seconds: CACHE_TTL_SECONDS,
                fetched_at: fetched_at_str,
                schema_version: SCHEMA_VERSION,
            },
        }
    }
}

fn parse_ymd(date: &str) -> Option<(u32, u32, u32)> {
    if date.len() < 10 {
        return None;
    }
    let y = date.get(0..4)?.parse().ok()?;
    let m = date.get(5..7)?.parse().ok()?;
    let d = date.get(8..10)?.parse().ok()?;
    Some((y, m, d))
}

fn get_month_label(month: u32) -> String {
    if (1..=12).contains(&month) {
        MONTH_LABELS[(month - 1) as usize].to_string()
    } else {
        String::new()
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::bool_to_int_with_if,
    clippy::unnecessary_cast
)]
fn get_utc_date(sys_time: SystemTime) -> (u32, u32, u32) {
    let secs = sys_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let z = secs / 86400 + 719_468;
    let era = z / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = (yoe as u64) + (era * 400);
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + (if m <= 2 { 1 } else { 0 });
    (year as u32, m as u32, d as u32)
}

fn format_iso_time(sys_time: SystemTime) -> String {
    let secs = sys_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, m, d) = get_utc_date(sys_time);
    let rem = secs % 86400;
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let ss = rem % 60;
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
