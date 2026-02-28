use crate::db::LogRepository;
use crate::models::CreateLog;
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::thread;
use std::time::Duration;
use tracing::{Event, Subscriber};
use tracing::span;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// A log message sent through the channel
struct LogMessage {
    level: String,
    message: String,
    target: String,
    task_id: Option<String>,
    session_id: Option<String>,
    metadata: Option<serde_json::Value>,
}

/// A tracing layer that writes to the database via a channel
pub struct DbLayer {
    sender: Sender<LogMessage>,
}

impl DbLayer {
    pub fn new(repo: LogRepository) -> Self {
        let (sender, receiver): (Sender<LogMessage>, Receiver<LogMessage>) = unbounded();

        // Spawn a background thread to write logs to the database
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for logging");

            rt.block_on(async {
                loop {
                    let mut messages = Vec::new();

                    match receiver.recv_timeout(Duration::from_millis(100)) {
                        Ok(msg) => {
                            messages.push(msg);
                            while let Ok(msg) = receiver.try_recv() {
                                messages.push(msg);
                                if messages.len() >= 100 {
                                    break;
                                }
                            }
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                    }

                    for msg in messages {
                        if let Err(e) = repo.create(CreateLog {
                            level: msg.level,
                            message: msg.message,
                            target: Some(msg.target),
                            source: Some("backend".to_string()),
                            task_id: msg.task_id,
                            session_id: msg.session_id,
                            metadata: msg.metadata,
                        }).await {
                            eprintln!("Failed to write log to database: {}", e);
                        }
                    }
                }
            });
        });

        Self { sender }
    }
}

impl<S> Layer<S> for DbLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    /// When a new span is created, extract task_id/session_id from its fields
    /// and store in SpanContext, inheriting any unset values from the parent span.
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("span not found in on_new_span");

        // Extract IDs from this span's own fields
        let mut span_ctx = SpanContext::default();
        let mut visitor = SpanFieldVisitor(&mut span_ctx);
        attrs.record(&mut visitor);

        // Inherit unset values from the nearest ancestor that has them
        if span_ctx.task_id.is_none() || span_ctx.session_id.is_none() {
            // Walk up the span tree
            let mut current = span.parent();
            while let Some(parent) = current {
                let ext = parent.extensions();
                if let Some(parent_ctx) = ext.get::<SpanContext>() {
                    if span_ctx.task_id.is_none() {
                        span_ctx.task_id = parent_ctx.task_id.clone();
                    }
                    if span_ctx.session_id.is_none() {
                        span_ctx.session_id = parent_ctx.session_id.clone();
                    }
                    if span_ctx.task_id.is_some() && span_ctx.session_id.is_some() {
                        break;
                    }
                }
                current = parent.parent();
            }
        }

        span.extensions_mut().insert(span_ctx);
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = metadata.level().to_string().to_uppercase();
        let target = metadata.target().to_string();

        // Extract all fields from the event
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        // Get IDs from span context (event fields take priority over span context)
        let (span_task_id, span_session_id) = ctx
            .lookup_current()
            .and_then(|span| {
                let ext = span.extensions();
                ext.get::<SpanContext>().map(|c| (c.task_id.clone(), c.session_id.clone()))
            })
            .unwrap_or((None, None));

        let task_id = visitor.task_id.or(span_task_id);
        let session_id = visitor.session_id.or(span_session_id);

        // Build metadata from all extra fields (if any)
        let metadata_val = if visitor.extra.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(visitor.extra))
        };

        let _ = self.sender.send(LogMessage {
            level,
            message: visitor.message,
            target,
            task_id,
            session_id,
            metadata: metadata_val,
        });
    }
}

/// Visitor to extract task_id/session_id from span field attributes
struct SpanFieldVisitor<'a>(&'a mut SpanContext);

impl tracing::field::Visit for SpanFieldVisitor<'_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "task_id" => self.0.task_id = Some(value.to_string()),
            "session_id" => self.0.session_id = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let s = format!("{:?}", value);
        // Strip surrounding quotes that Debug adds to strings
        let s = s.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
            .map(|s| s.to_string())
            .unwrap_or(s);
        match field.name() {
            "task_id" => self.0.task_id = Some(s),
            "session_id" => self.0.session_id = Some(s),
            _ => {}
        }
    }
}

/// Visitor to extract all fields from an event
#[derive(Default)]
struct EventVisitor {
    message: String,
    task_id: Option<String>,
    session_id: Option<String>,
    extra: serde_json::Map<String, serde_json::Value>,
}

impl tracing::field::Visit for EventVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "message" => self.message = value.to_string(),
            "task_id" => self.task_id = Some(value.to_string()),
            "session_id" => self.session_id = Some(value.to_string()),
            name => {
                self.extra.insert(name.to_string(), serde_json::Value::String(value.to_string()));
            }
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let s = format!("{:?}", value);
        // Strip surrounding quotes that Debug adds for Display (%)-formatted string fields
        let clean = s.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
            .map(|s| s.to_string())
            .unwrap_or_else(|| s.clone());
        match field.name() {
            "message" => self.message = clean,
            "task_id" => self.task_id = Some(clean),
            "session_id" => self.session_id = Some(clean),
            name => {
                self.extra.insert(name.to_string(), serde_json::Value::String(s));
            }
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.extra.insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.extra.insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.extra.insert(field.name().to_string(), serde_json::json!(value));
    }
}

/// Context stored in span extensions for propagation
#[derive(Clone, Default)]
pub struct SpanContext {
    pub task_id: Option<String>,
    pub session_id: Option<String>,
}
