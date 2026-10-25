/*
    This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

    (C) 2026 ItsThatOneJack (Jack Bacon)
*/

use chrono::{DateTime, Utc};

/// Parses an RFC3339-like timestamp and humanises it into a relative time (e.g. "2 days ago"), falls back to the raw string if it can't parse.
pub fn relative_time(created_at: &str) -> String {
    let Ok(parsed) = DateTime::parse_from_rfc3339(created_at) else {
        return created_at.to_string();
    };
    let parsed: DateTime<Utc> = parsed.with_timezone(&Utc);
    let delta = Utc::now().signed_duration_since(parsed);

    if delta.num_seconds() < 60 {
        "just now".to_string()
    } else if delta.num_minutes() < 60 {
        plural(delta.num_minutes(), "minute")
    } else if delta.num_hours() < 24 {
        plural(delta.num_hours(), "hour")
    } else if delta.num_days() < 30 {
        plural(delta.num_days(), "day")
    } else if delta.num_days() < 365 {
        plural(delta.num_days() / 30, "month")
    } else {
        plural(delta.num_days() / 365, "year")
    }
}

fn plural(n: i64, unit: &str) -> String {
    if n == 1 {
        format!("1 {unit} ago")
    } else {
        format!("{n} {unit}s ago")
    }
}
