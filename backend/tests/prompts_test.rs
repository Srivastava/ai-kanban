use ai_kanban_backend::claude::build_prompt;

#[test]
fn test_prompt_planning_stage() {
    let prompt = build_prompt("Fix bug", Some("Details here"), "planning", None);
    assert!(prompt.contains("Fix bug"));
    assert!(prompt.contains("PLANNING mode"));
    assert!(prompt.contains("Details here"));
    assert!(!prompt.contains("Conversation History"));
}

#[test]
fn test_prompt_in_progress_stage() {
    let prompt = build_prompt("Build feature", None, "in_progress", None);
    assert!(prompt.contains("IN_PROGRESS mode"));
    assert!(!prompt.contains("Task Description"));
}

#[test]
fn test_prompt_review_stage() {
    let prompt = build_prompt("Review task", None, "review", None);
    assert!(prompt.contains("REVIEW mode"));
}

#[test]
fn test_prompt_unknown_stage() {
    let prompt = build_prompt("Task", None, "done", None);
    assert!(prompt.contains("Complete the task"));
}

#[test]
fn test_prompt_with_conversation_context() {
    let ctx = "[Claude]: I did X\n[You]: Great, now do Y";
    let prompt = build_prompt("Task", None, "planning", Some(ctx));
    assert!(prompt.contains("Conversation History"));
    assert!(prompt.contains("I did X"));
}

#[test]
fn test_prompt_no_description() {
    let prompt = build_prompt("Task", None, "planning", None);
    assert!(!prompt.contains("Task Description"));
}
