use ai_kanban_backend::claude::build_prompt;

#[test]
fn test_prompt_planning_stage() {
    let prompt = build_prompt("Fix bug", Some("Details here"), "planning", None, false);
    assert!(prompt.contains("Fix bug"));
    assert!(prompt.contains("PLANNING mode"));
    assert!(prompt.contains("Details here"));
    assert!(!prompt.contains("Conversation History"));
}

#[test]
fn test_prompt_in_progress_stage() {
    let prompt = build_prompt("Build feature", None, "in_progress", None, false);
    assert!(prompt.contains("IN_PROGRESS mode"));
    assert!(!prompt.contains("Task Description"));
}

#[test]
fn test_prompt_in_progress_with_plan() {
    let prompt = build_prompt("Build feature", None, "in_progress", None, true);
    assert!(prompt.contains("IN_PROGRESS mode"));
    assert!(prompt.contains("ai-kanban-plan.md"));
}

#[test]
fn test_prompt_review_stage() {
    let prompt = build_prompt("Review task", None, "review", None, false);
    assert!(prompt.contains("REVIEW mode"));
}

#[test]
fn test_prompt_unknown_stage() {
    let prompt = build_prompt("Task", None, "done", None, false);
    assert!(prompt.contains("Complete the task"));
}

#[test]
fn test_prompt_with_conversation_context() {
    let ctx = "[Claude]: I did X\n[You]: Great, now do Y";
    let prompt = build_prompt("Task", None, "planning", Some(ctx), false);
    assert!(prompt.contains("Context"));
    assert!(prompt.contains("I did X"));
}

#[test]
fn test_prompt_no_description() {
    let prompt = build_prompt("Task", None, "planning", None, false);
    assert!(!prompt.contains("Task Description"));
}

#[test]
fn test_prompt_description_appears_before_context() {
    let prompt = build_prompt(
        "Task",
        Some("The desc"),
        "planning",
        Some("Some ctx"),
        false,
    );
    let desc_pos = prompt.find("The desc").unwrap();
    let ctx_pos = prompt.find("Some ctx").unwrap();
    assert!(
        desc_pos < ctx_pos,
        "description must appear before context for prompt cache efficiency"
    );
}

#[test]
fn test_prompt_planning_saves_plan_instruction() {
    let prompt = build_prompt("My Task", None, "planning", None, false);
    assert!(
        prompt.contains("ai-kanban-plan.md"),
        "planning prompt must instruct Claude to save plan to ai-kanban-plan.md"
    );
    assert!(prompt.contains("PLANNING mode"), "must be in PLANNING mode");
    assert!(
        prompt.contains("Do NOT make any code changes"),
        "planning mode must prohibit code changes"
    );
}

#[test]
fn test_prompt_review_with_plan_references_plan() {
    let prompt = build_prompt("My Task", None, "review", None, true);
    assert!(prompt.contains("REVIEW mode"), "must be in REVIEW mode");
    assert!(
        prompt.contains("ai-kanban-plan.md"),
        "review+has_plan prompt must reference the plan file to verify completeness"
    );
}

#[test]
fn test_prompt_review_without_plan_does_not_mention_plan_file() {
    let prompt = build_prompt("My Task", None, "review", None, false);
    assert!(prompt.contains("REVIEW mode"), "must be in REVIEW mode");
    assert!(
        !prompt.contains("ai-kanban-plan.md"),
        "review without plan should not reference plan file"
    );
}

#[test]
fn test_prompt_in_progress_without_plan_does_not_mention_plan_file() {
    let prompt = build_prompt("My Task", None, "in_progress", None, false);
    assert!(
        prompt.contains("IN_PROGRESS mode"),
        "must be in IN_PROGRESS mode"
    );
    assert!(
        !prompt.contains("ai-kanban-plan.md"),
        "in_progress without plan should not reference plan file"
    );
}
