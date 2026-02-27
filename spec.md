Local AI Task Automation Platform – Final Architecture Spec
1. System Overview
A local-first AI task automation platform that orchestrates multiple Claude CLI agents to work on tasks within a local project directory.

Key goals:

Resume Claude sessions (claude --continue <session_id>)
Minimize token usage through structured task context
Track detailed analytics
Stage-aware AI prompting (Kanban-based)
Store decisions and summaries per task
Hyper-modern UI dashboard
Fully local, no authentication required
Maximum concurrency: 3 Claude sessions and 3 active tasks
2. Core Principles
Local-First Architecture
Everything runs locally:

No cloud backend
No GitHub integration but YOU MUST USE git to take snapshots
Direct access to filesystem project directories
Token Efficiency
Context is stored in structured task memory to avoid large prompts.

Resume-Safe Execution
Agents can stop and resume without losing context.

Transparent AI Workflows
Humans can read:

Decisions
Summaries
Code changes
Token usage
3. High-Level Architecture
Frontend (Hyper Modern UI)
        |
        v
Backend API (Rust / Node)
        |
        +---------------------+
        |                     |
        v                     v
Task Engine             Analytics Engine
        |                     |
        v                     v
Claude CLI Manager     Token Processing
        |
        v
Local Project Directory
4. Technology Stack
Backend
Recommended: Rust (high performance + concurrency)

Alternatives: Node.js with Bun runtime

Core components:

Task Orchestrator
Claude CLI Manager
Snapshot Engine
Token Analytics Engine
Libraries:

SQLite (database)
WebSockets
File Watchers
Job Queue
Frontend
Recommended stack: Next.js + Tailwind + Shadcn UI

UI Style: Hyper-modern developer dashboard inspired by:

Linear
Vercel
Raycast
Key UI features:

Glassmorphism panels
Real-time updates
Smooth transitions
Dark-first theme
5. Directory Structure
ai-platform/
│
├── backend/
│   ├── task_engine/
│   ├── claude_manager/
│   ├── analytics/
│   ├── snapshot/
│   └── api/
│
├── frontend/
│   ├── dashboard/
│   ├── kanban/
│   ├── analytics/
│   └── task_view/
│
├── data/
│   ├── tasks/
│   ├── sessions/
│   └── snapshots/
│
├── database/
│   └── sqlite.db
│
└── projects/
    └── user_project/
6. Task Lifecycle
Kanban Stages:

Backlog
Planning
Ready
In Progress
Review
Done
Each stage modifies Claude's behavior.

7. Stage-Aware AI System Prompts
These are critical.

Planning Stage Prompt
You are a senior software architect.

Goal:
Analyze the task and produce a detailed plan.

Requirements:
- Ask clarifying questions
- Break work into steps
- Identify risks
- Suggest file changes
- Estimate complexity

Do NOT write code yet.
Ready Stage Prompt
You are preparing to implement the solution.

Ensure:
- Plan is correct
- Dependencies identified
- Files to modify confirmed

Output:
Final implementation plan.
In Progress Prompt
You are implementing the solution.

Rules:
- Work incrementally
- Save decisions
- Keep summaries concise
- Minimize token usage
- Update task memory
Review Stage Prompt
You are reviewing your work.

Check:
- Code correctness
- Tests
- Edge cases
- Performance
8. Claude Session Management
Each task must store:

session_id
last_snapshot
current_file
progress_summary
token_usage
Resume logic:

claude --continue <session_id>
Session stored in DB.

9. Snapshot System
Snapshots created every:

file change
milestone
agent stop
token threshold
Snapshot contains:

task_state.json
diff.patch
decision_log.md
summary.md
10. Task Memory System (Important)
This reduces token usage significantly.

Each task has:

task_context/
   summary.md
   decisions.md
   architecture_notes.md
   implementation_log.md
Claude reads this when resuming.

11. Database Schema
Tasks Table
tasks
-----
id
title
description
stage
project_path
session_id
created_at
updated_at
Token Usage Table
token_usage
------------
task_id
session_id
tokens_input
tokens_output
language
lines_written
timestamp
Snapshots
snapshots
---------
task_id
snapshot_id
session_id
created_at
path
12. Token Analytics System
This is a major feature.

Track:

Per task:

tokens used
cost
duration
Per language:

token efficiency
LoC per token
Per week/month:

total usage
trends
13. Analytics Dashboard
Tabs:

Overview Tasks Language Efficiency Token Usage Performance

Metrics
Current Token Usage Weekly Usage Monthly Usage Tokens per Task Tokens per File Tokens per Language Lines per Token

14. UI Design Spec
Dashboard Layout
------------------------------------------------
Top Bar
------------------------------------------------
Stats Cards
------------------------------------------------
Kanban Board
------------------------------------------------
Live Sessions
------------------------------------------------
Analytics Graph
Task Page
Sections:

Overview AI Decisions Comments Code Changes Snapshots Token Analytics

Hyper Modern UI Features
Smooth animations
Realtime updates
AI activity indicator
Inline comments like Linear
Code diff viewer
Token meter
15. Claude CLI Manager
Responsible for:

Starting sessions

claude run
Resuming sessions

claude --continue <session_id>
Monitoring output

Streaming logs to UI.

16. Concurrency Limits
System limits:

3 active tasks 3 Claude sessions

Queue system required.

17. Local Project Integration
Agents operate directly on:

/projects/<project_name>
Capabilities:

Edit files Run tests Analyze repo Generate code


18. Token Efficiency Strategy
To minimize tokens:

Use structured summaries
Only send diffs
Load task memory instead of full repo
Stage-aware prompts
Avoid repeating context
19. Future Expansion
Planned capabilities:

Multiple AI models Remote workers Plugin system Voice command tasks Autonomous debugging

You can get token usage using ccusage cli. Its a CLI command and you get lot of information, so directly integrate with it. Full documentation is at https://ccusage.com
Usage is below
# Basic usage
npx ccusage          # Show daily report (default)
npx ccusage daily    # Daily token usage and costs
npx ccusage monthly  # Monthly aggregated report
npx ccusage session  # Usage by conversation session
npx ccusage blocks   # 5-hour billing windows
npx ccusage statusline  # Compact status line for hooks (Beta)

# Filters and options
npx ccusage daily --since 20250525 --until 20250530
npx ccusage daily --json  # JSON output
npx ccusage daily --breakdown  # Per-model cost breakdown
npx ccusage daily --timezone UTC  # Use UTC timezone
npx ccusage daily --locale ja-JP  # Use Japanese locale for date/time formatting

# Project analysis
npx ccusage daily --instances  # Group by project/instance
npx ccusage daily --project myproject  # Filter to specific project
npx ccusage daily --instances --project myproject --json  # Combined usage

# Compact mode for screenshots/sharing
npx ccusage --compact  # Force compact table mode
npx ccusage monthly --compact  # Compact monthly report

20. Development Phases
Phase 1 Core backend Task orchestration Claude integration

Phase 2 Snapshot system Analytics

Phase 3 Frontend UI

Phase 4 Optimization

21. Final Implementation Goals
The platform should:

Run locally Be extremely token efficient Allow AI agents to resume work easily Provide deep analytics Have a beautiful UI Support human collaboration
