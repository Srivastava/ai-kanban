use crate::models::CreateOtelMetric;
use serde_json::Value;

const TRACKED_METRICS: &[&str] = &[
    "claude_code.token.usage",
    "claude_code.cost.usage",
    "claude_code.session.count",
    "claude_code.lines_of_code.count",
    "claude_code.commit.count",
    "claude_code.pull_request.count",
    "claude_code.active_time.total",
];

/// Parse an OTLP/HTTP JSON metrics payload into CreateOtelMetric records.
/// Only extracts metrics in TRACKED_METRICS.
pub fn parse_otlp_metrics(body: &Value) -> Vec<CreateOtelMetric> {
    let mut results = Vec::new();

    let resource_metrics = match body.get("resourceMetrics").and_then(|v| v.as_array()) {
        Some(rm) => rm,
        None => return results,
    };

    for rm in resource_metrics {
        let claude_session_id = extract_string_attr(
            rm.get("resource").and_then(|r| r.get("attributes")),
            "session.id",
        )
        .unwrap_or_default();

        let resource_attrs = build_attrs(rm.get("resource").and_then(|r| r.get("attributes")));

        let scope_metrics = match rm.get("scopeMetrics").and_then(|v| v.as_array()) {
            Some(sm) => sm,
            None => continue,
        };

        for sm in scope_metrics {
            let metrics = match sm.get("metrics").and_then(|v| v.as_array()) {
                Some(m) => m,
                None => continue,
            };

            for metric in metrics {
                let name = match metric.get("name").and_then(|v| v.as_str()) {
                    Some(n) if TRACKED_METRICS.contains(&n) => n.to_string(),
                    _ => continue,
                };
                let unit = metric.get("unit").and_then(|v| v.as_str()).map(|s| s.to_string());

                let data_points = metric
                    .get("sum").and_then(|s| s.get("dataPoints"))
                    .or_else(|| metric.get("gauge").and_then(|g| g.get("dataPoints")))
                    .and_then(|dp| dp.as_array());

                if let Some(dps) = data_points {
                    for dp in dps {
                        let value = dp.get("asInt").and_then(|v| v.as_f64())
                            .or_else(|| dp.get("asDouble").and_then(|v| v.as_f64()))
                            .unwrap_or(0.0);

                        let otel_timestamp = dp.get("timeUnixNano")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<i64>().ok())
                            .or_else(|| dp.get("timeUnixNano").and_then(|v| v.as_i64()))
                            .unwrap_or(0);

                        let mut attrs = resource_attrs.clone();
                        let dp_attrs = build_attrs(dp.get("attributes"));
                        if let (Some(merged), Some(extra)) = (attrs.as_object_mut(), dp_attrs.as_object()) {
                            for (k, v) in extra {
                                merged.insert(k.clone(), v.clone());
                            }
                        }

                        results.push(CreateOtelMetric {
                            metric_name: name.clone(),
                            value,
                            unit: unit.clone(),
                            session_id: None,
                            task_id: None,
                            claude_session_id: claude_session_id.clone(),
                            attributes: attrs,
                            otel_timestamp,
                        });
                    }
                }
            }
        }
    }

    results
}

fn extract_string_attr(attrs: Option<&Value>, key: &str) -> Option<String> {
    attrs?.as_array()?.iter().find_map(|a| {
        if a.get("key")?.as_str()? == key {
            a.get("value")?.get("stringValue")?.as_str().map(|s| s.to_string())
        } else {
            None
        }
    })
}

fn build_attrs(attrs: Option<&Value>) -> Value {
    let mut map = serde_json::Map::new();
    if let Some(arr) = attrs.and_then(|a| a.as_array()) {
        for a in arr {
            if let (Some(key), Some(val)) = (
                a.get("key").and_then(|k| k.as_str()),
                a.get("value"),
            ) {
                let scalar = val.get("stringValue").or_else(|| val.get("intValue"))
                    .or_else(|| val.get("doubleValue")).or_else(|| val.get("boolValue"))
                    .cloned()
                    .unwrap_or(Value::Null);
                map.insert(key.to_string(), scalar);
            }
        }
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_body(metric_name: &str, value: i64, session_id: &str) -> Value {
        serde_json::json!({
            "resourceMetrics": [{
                "resource": {
                    "attributes": [
                        {"key": "session.id", "value": {"stringValue": session_id}}
                    ]
                },
                "scopeMetrics": [{
                    "metrics": [{
                        "name": metric_name,
                        "unit": "1",
                        "sum": {
                            "dataPoints": [{
                                "attributes": [],
                                "asInt": value,
                                "timeUnixNano": "1709000000000000000"
                            }]
                        }
                    }]
                }]
            }]
        })
    }

    #[test]
    fn test_parse_tracked_metric() {
        let body = sample_body("claude_code.commit.count", 3, "sess-abc");
        let results = parse_otlp_metrics(&body);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metric_name, "claude_code.commit.count");
        assert!((results[0].value - 3.0).abs() < 0.01);
        assert_eq!(results[0].claude_session_id, "sess-abc");
    }

    #[test]
    fn test_parse_untracked_metric_ignored() {
        let body = sample_body("some.internal.metric", 99, "sess-abc");
        let results = parse_otlp_metrics(&body);
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_data_point_attributes() {
        let body = serde_json::json!({
            "resourceMetrics": [{
                "resource": {"attributes": [
                    {"key": "session.id", "value": {"stringValue": "s1"}}
                ]},
                "scopeMetrics": [{"metrics": [{
                    "name": "claude_code.lines_of_code.count",
                    "sum": {"dataPoints": [{
                        "attributes": [
                            {"key": "type", "value": {"stringValue": "added"}}
                        ],
                        "asInt": 50,
                        "timeUnixNano": "1709000000000000000"
                    }]}
                }]}]
            }]
        });
        let results = parse_otlp_metrics(&body);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].attributes["type"], "added");
        assert!((results[0].value - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_empty_body_returns_empty() {
        let results = parse_otlp_metrics(&serde_json::json!({}));
        assert!(results.is_empty());
    }
}
