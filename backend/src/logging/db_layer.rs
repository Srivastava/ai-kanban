use crate::db::LogRepository;
use crate::models::CreateLog;
use chrono::Utc;
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::thread;
use std::time::Duration;
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// A log message sent through the channel
struct LogMessage {
    timestamp: String,
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
            // Create a Tokio runtime for async operations
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for logging");

            rt.block_on(async {
                loop {
                    // Batch collect messages with a small timeout
                    let mut messages = Vec::new();

                    // Wait for first message
                    match receiver.recv_timeout(Duration::from_millis(100)) {
                        Ok(msg) => {
                            messages.push(msg);

                            // Collect any additional messages available immediately
                            while let Ok(msg) = receiver.try_recv() {
                                messages.push(msg);
                                if messages.len() >= 100 {
                                    break; // Batch limit
                                }
                            }
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                            // No messages available, continue loop
                            continue;
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                            // Channel closed, exit
                            break;
                        }
                    }

                    // Write batch to database
                    if !messages.is_empty() {
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
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // Extract metadata
        let metadata = event.metadata();
        let level = metadata.level().to_string().to_uppercase();
        let target = metadata.target().to_string();

        // Extract message
        let mut message = String::new();
        let mut visitor = MessageVisitor::new(&mut message);
        event.record(&mut visitor);

        // Extract task_id and session_id from span context
        let (task_id, session_id) = extract_span_context(&ctx);

        // Create log message
        let log_msg = LogMessage {
            timestamp: Utc::now().to_rfc3339(),
            level,
            message,
            target,
            task_id,
            session_id,
            metadata: None,
        };

        // Send to channel (non-blocking)
        let _ = self.sender.send(log_msg);
    }
}

/// Visitor to extract the message from an event
struct MessageVisitor<'a> {
    message: &'a mut String,
}

impl<'a> MessageVisitor<'a> {
    fn new(message: &'a mut String) -> Self {
        Self { message }
    }
}

impl tracing::field::Visit for MessageVisitor<'_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            *self.message = value.to_string();
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            *self.message = format!("{:?}", value);
        }
    }
}

/// Extract task_id and session_id from the current span context
fn extract_span_context<S>(ctx: &Context<'_, S>) -> (Option<String>, Option<String>)
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let mut task_id = None;
    let mut session_id = None;

    if let Some(span) = ctx.lookup_current() {
        let extensions = span.extensions();
        // Look for stored context in span extensions
        if let Some(ctx) = extensions.get::<SpanContext>() {
            task_id = ctx.task_id.clone();
            session_id = ctx.session_id.clone();
        }
    }

    (task_id, session_id)
}

/// Context stored in spans for logging
#[derive(Clone, Default)]
pub struct SpanContext {
    pub task_id: Option<String>,
    pub session_id: Option<String>,
}
