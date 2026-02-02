//! Statistics and analytics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// System statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    /// Total number of users.
    pub total_users: usize,
    /// Total number of active sessions.
    pub active_sessions: usize,
    /// Users created today.
    pub users_today: usize,
    /// Signins today.
    pub signins_today: usize,
    /// Failed signins today.
    pub failed_signins_today: usize,
}

impl Default for SystemStats {
    fn default() -> Self {
        Self {
            total_users: 0,
            active_sessions: 0,
            users_today: 0,
            signins_today: 0,
            failed_signins_today: 0,
        }
    }
}

/// Time series data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// Chart data for analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub label: String,
    pub data: Vec<DataPoint>,
}

impl ChartData {
    /// Creates new chart data.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            data: Vec::new(),
        }
    }

    /// Adds a data point.
    pub fn point(mut self, timestamp: DateTime<Utc>, value: f64) -> Self {
        self.data.push(DataPoint { timestamp, value });
        self
    }
}

/// Analytics dashboard data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsDashboard {
    /// Current stats.
    pub stats: SystemStats,
    /// Daily active users chart.
    pub dau_chart: ChartData,
    /// Signups over time chart.
    pub signups_chart: ChartData,
    /// Login failures chart.
    pub failures_chart: ChartData,
}

impl Default for AnalyticsDashboard {
    fn default() -> Self {
        Self {
            stats: SystemStats::default(),
            dau_chart: ChartData::new("Daily Active Users"),
            signups_chart: ChartData::new("Signups"),
            failures_chart: ChartData::new("Login Failures"),
        }
    }
}
