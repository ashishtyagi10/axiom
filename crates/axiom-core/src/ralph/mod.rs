//! Ralph Loop - Autonomous iterative development loop
//!
//! The Ralph Loop pattern enables continuous autonomous development cycles with
//! fresh context on each iteration, avoiding context accumulation issues while
//! persisting state through files.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │  StartRalphLoop │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  Load/Create    │
//! │  RalphState     │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐     ┌──────────────────┐
//! │  Run Iteration  │────→│ Check Completion │
//! │  (fresh context)│     │  - complete.txt  │
//! └────────┬────────┘     │  - output signal │
//!          │              └────────┬─────────┘
//!          │                       │
//!          │    ┌──────────────────┘
//!          │    │
//!          ▼    ▼
//! ┌─────────────────┐
//! │  Complete?      │───Yes──→ RalphLoopComplete
//! └────────┬────────┘
//!          │ No
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  Max iterations?│───Yes──→ RalphLoopComplete
//! └────────┬────────┘
//!          │ No
//!          │
//!          └────────→ (loop back to Run Iteration)
//! ```
//!
//! # State Persistence
//!
//! State is persisted to `.axiom/ralph/` directory:
//! - `task.md`: Current task description
//! - `iteration.txt`: Current iteration number
//! - `feedback.txt`: Feedback from last iteration
//! - `work-summary.txt`: Summary of work done
//! - `complete.txt`: Completion signal file

mod executor;
mod state;

pub use executor::{AgentInvoker, CancellationToken, CliAgentInvoker, RalphExecutor};
pub use state::{
    CompletionReason, IterationRecord, IterationStatus, RalphConfig, RalphError, RalphResult,
    RalphState, RalphStatus,
};
