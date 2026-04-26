use dashmap::DashMap;
use reqwest::Client;
use serde::Serialize;
use std::borrow::Cow;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::info;

use crate::{
    error::AppError,
    models::{
        contributions::{
            ContributionCell, ContributionLegend, ContributionMeta, ContributionMonth,
            ContributionRange, ContributionSummary, ContributionsResponse,
        },
        github_dto::{GithubContributionCalendar, GithubGqlResponse},
    },
};

/// Cache Time-to-Live in seconds (3 hours).
/// This ensures mid-day updates are visible while staying within API rate limits.
const CACHE_TTL_SECONDS: u64 = 10800;
const CACHE_MAX_CAPACITY: usize = 1000;
const SCHEMA_VERSION: u32 = 1;
const PROVIDER_GITHUB: &str = "github";
const USER_AGENT: &str = "nadzu-backend";

const WEEKDAY_LABELS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTH_LABELS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const CONTRIBUTION_COLORS: [Cow<'static, str>; 5] = [
    Cow::Borrowed("#2b2c3494"), // Level 0 (None)
    Cow::Borrowed("#9be9a8"),   // Level 1 (Low)
    Cow::Borrowed("#40c463"),   // Level 2 (Medium)
    Cow::Borrowed("#30a14e"),   // Level 3 (High)
    Cow::Borrowed("#216e39"),   // Level 4 (Very High)
];

const LEGEND_LABELS: [&str; 5] = ["No contributions", "Low", "Medium", "High", "Very high"];

const GITHUB_CONTRIBUTIONS_QUERY: &str = r"
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

#[derive(Debug, Serialize)]
struct GithubGqlRequest {
    query: &'static str,
    variables: GithubGqlVariables,
}

#[derive(Debug, Serialize)]
struct GithubGqlVariables {
    username: String,
}

/// Service for fetching and caching GitHub contribution data.
pub struct ContributionsService {
    http_client: Client,
    pat: String,
    default_username: String,
    graphql_url: String,
    /// Cache stores (Username -> (Response, `ExpiryTimestamp`))
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
    /// Creates a new `ContributionsService` and starts the background cache cleanup task.
    pub fn new(
        http_client: Client,
        pat: String,
        default_username: String,
        graphql_url: String,
    ) -> Self {
        let cache = Arc::new(DashMap::new());
        let cache_weak = Arc::downgrade(&cache);

        // Background task to prune expired entries every 10 minutes
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_mins(10));
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

    /// Seeds the cache with a predefined response (primarily for testing).
    pub fn seed_cache(&self, username: &str, response: ContributionsResponse, ttl_secs: u64) {
        let expires_at = now_unix() + ttl_secs;
        self.cache
            .insert(username.to_string(), (response, expires_at));
    }

    /// Returns the configured default GitHub username.
    pub fn get_default_username(&self) -> &str {
        &self.default_username
    }

    /// Retrieves contributions for the given username, utilizing cache if available and valid.
    /// Implements a "Midnight Snap" strategy to ensure the calendar refreshes at UTC midnight.
    pub async fn get_contributions(
        &self,
        username: &str,
    ) -> Result<ContributionsResponse, AppError> {
        let now = now_unix();
        let cache_key = username.to_string();

        if username.trim().is_empty() {
            return Err(AppError::Validation("Username cannot be empty".into()));
        }

        // Restrict to default username for security/scope
        if username != self.default_username {
            return Err(AppError::Validation(
                "Only the default username is allowed".into(),
            ));
        }

        // 1. Check Cache
        if let Some(entry) = self.cache.get(&cache_key) {
            let (cached_resp, expires_at) = entry.value();
            if *expires_at > now {
                let mut resp = cached_resp.clone();
                resp.meta.cached = true;
                return Ok(resp);
            }
        }

        // 2. Fetch fresh data
        let resp_result = self.fetch_and_process(username, SystemTime::now()).await;

        match resp_result {
            Ok(new_resp) => {
                // Determine TTL: 3 hours OR seconds until UTC midnight, whichever is sooner.
                let seconds_since_midnight = now % 86400;
                let seconds_until_midnight = 86400 - seconds_since_midnight;
                let ttl = CACHE_TTL_SECONDS.min(seconds_until_midnight);

                let expires_at = now + ttl;

                if self.cache.len() < CACHE_MAX_CAPACITY {
                    self.cache.insert(cache_key, (new_resp.clone(), expires_at));
                }
                Ok(new_resp)
            }
            Err(e) => {
                // Fallback to stale cache on upstream failure
                if let Some(entry) = self.cache.get(&cache_key) {
                    let mut resp = entry.value().0.clone();
                    resp.meta.cached = true;
                    return Ok(resp);
                }
                Err(e)
            }
        }
    }

    /// Fetches data from GitHub GraphQL API and processes it into the internal model.
    async fn fetch_and_process(
        &self,
        username: &str,
        fetched_at: SystemTime,
    ) -> Result<ContributionsResponse, AppError> {
        let payload = GithubGqlRequest {
            query: GITHUB_CONTRIBUTIONS_QUERY,
            variables: GithubGqlVariables {
                username: username.to_string(),
            },
        };

        let resp = self
            .http_client
            .post(&self.graphql_url)
            .bearer_auth(&self.pat)
            .header("User-Agent", USER_AGENT)
            .timeout(Duration::from_secs(30))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::UpstreamError(format!("Network or timeout error: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::UpstreamError(format!(
                "Upstream API returned status: {}",
                resp.status()
            )));
        }

        let gh_resp: GithubGqlResponse = resp
            .json()
            .await
            .map_err(|e| AppError::UpstreamError(format!("JSON parsing error: {e}")))?;

        if let Some(errs) = gh_resp.errors
            && !errs.is_empty()
        {
            return Err(AppError::UpstreamError(
                "GitHub GraphQL returned errors".to_string(),
            ));
        }

        let calendar = gh_resp
            .data
            .and_then(|d| d.user)
            .map(|u| u.contributions_collection.contribution_calendar)
            .ok_or_else(|| {
                AppError::UpstreamError("User not found or missing fields".to_string())
            })?;

        Ok(Self::transform_calendar(username, calendar, fetched_at))
    }

    /// Transforms GitHub's raw calendar data into the simplified `ContributionsResponse`.
    fn transform_calendar(
        username: &str,
        calendar: GithubContributionCalendar,
        fetched_at: SystemTime,
    ) -> ContributionsResponse {
        let mut cells = Vec::with_capacity(calendar.weeks.len() * 7);
        let mut months = Vec::new();
        let mut max_daily_count = 0;
        let mut last_month: Option<u32> = None;

        let mut level_mins = [u32::MAX; 5];
        let mut level_maxs = [0u32; 5];
        level_mins[0] = 0;

        // Compare against "Today" in UTC to identify future cells or the current month
        let (current_year, current_month_num, current_day) = get_utc_date(SystemTime::now());
        let current_date_str = format!("{current_year:04}-{current_month_num:02}-{current_day:02}");
        let fetched_at_str = format_iso_time(fetched_at);

        let total_contributions = calendar.total_contributions;
        let total_weeks = u32::try_from(calendar.weeks.len()).unwrap_or_default();

        for (week_idx, week) in calendar.weeks.into_iter().enumerate() {
            let mut month_added_this_week = false;

            for day in week.contribution_days {
                if day.contribution_count > max_daily_count {
                    max_daily_count = day.contribution_count;
                }

                let (y_day, m_day, _) = parse_ymd(&day.date).unwrap_or((1970, 1, 1));

                // Detect month transitions to add labels to the grid
                if !month_added_this_week && (last_month != Some(m_day)) {
                    months.push(ContributionMonth {
                        label: Cow::Borrowed(get_month_label(m_day)),
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

                // Track min/max for legend in the same pass
                if (1..5).contains(&level) {
                    level_mins[level] = level_mins[level].min(day.contribution_count);
                    level_maxs[level] = level_maxs[level].max(day.contribution_count);
                }

                #[allow(clippy::cast_possible_truncation)]
                cells.push(ContributionCell {
                    date: day.date,
                    week_index: week_idx,
                    weekday: day.weekday,
                    weekday_label: Cow::Borrowed(
                        WEEKDAY_LABELS.get(day.weekday as usize).unwrap_or(&""),
                    ),
                    count: day.contribution_count,
                    level: level as u32,
                    color: CONTRIBUTION_COLORS[level].clone(),
                    is_future,
                    is_in_current_month,
                });
            }
        }

        // Finalize Legend
        #[allow(clippy::cast_possible_truncation)]
        let legend = (0..5)
            .map(|i| ContributionLegend {
                level: i as u32,
                label: Cow::Borrowed(LEGEND_LABELS[i]),
                min: if level_mins[i] == u32::MAX {
                    0
                } else {
                    level_mins[i]
                },
                max: level_maxs[i],
                color: CONTRIBUTION_COLORS[i].clone(),
            })
            .collect();

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
                total_contributions,
                total_weeks,
                max_daily_count,
            },

            legend,
            months,
            cells,
            meta: ContributionMeta {
                provider: PROVIDER_GITHUB.into(),
                cached: false,
                cache_ttl_seconds: u32::try_from(CACHE_TTL_SECONDS).unwrap_or(86400),
                fetched_at: fetched_at_str,
                schema_version: SCHEMA_VERSION,
            },
        }
    }
}

/// Parses "YYYY-MM-DD" into (Year, Month, Day)
fn parse_ymd(date: &str) -> Option<(u32, u32, u32)> {
    if date.len() < 10 {
        return None;
    }
    Some((
        date.get(0..4)?.parse().ok()?,
        date.get(5..7)?.parse().ok()?,
        date.get(8..10)?.parse().ok()?,
    ))
}

/// Returns the short month name for a given month index (1-12)
fn get_month_label(month: u32) -> &'static str {
    MONTH_LABELS
        .get((month.saturating_sub(1)) as usize)
        .unwrap_or(&"")
}

/// Calculates UTC Year, Month, Day from `SystemTime`
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
    let y = (u64::from(yoe)) + (era * 400);
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + u64::from(m <= 2);
    (year as u32, m, d)
}

/// Formats `SystemTime` into ISO 8601 string (UTC)
fn format_iso_time(sys_time: SystemTime) -> String {
    let secs = sys_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, m, d) = get_utc_date(sys_time);
    let rem = secs % 86400;
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Returns current Unix timestamp in seconds
fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
