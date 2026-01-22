//! Ralph Loop state management
//!
//! Provides state structures and file-based persistence for Ralph Loop iterations.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tokio::fs;

/// Ralph Loop specific errors
#[derive(Error, Debug, Clone)]
pub enum RalphError {
    /// I/O error during state persistence
    #[error("Ralph I/O error: {0}")]
    Io(String),

    /// Serialization error
    #[error("Ralph serialization error: {0}")]
    Serialization(String),

    /// State corruption or invalid data
    #[error("Ralph state error: {0}")]
    State(String),

    /// Operation not allowed in current state
    #[error("Invalid Ralph operation: {0}")]
    InvalidOperation(String),

    /// Loop already running
    #[error("Ralph Loop is already running")]
    AlreadyRunning,

    /// No active loop
    #[error("No Ralph Loop is currently running")]
    NotRunning,

    /// Max iterations reached
    #[error("Max iterations ({0}) reached")]
    MaxIterationsReached(u32),

    /// Iteration timeout
    #[error("Iteration timed out after {0:?}")]
    IterationTimeout(Duration),

    /// Circuit breaker triggered
    #[error("Circuit breaker triggered after {0} consecutive failures")]
    CircuitBreakerTriggered(u32),
}

impl RalphError {
    pub fn state(msg: impl Into<String>) -> Self {
        RalphError::State(msg.into())
    }

    pub fn invalid_operation(msg: impl Into<String>) -> Self {
        RalphError::InvalidOperation(msg.into())
    }

    pub fn io(msg: impl Into<String>) -> Self {
        RalphError::Io(msg.into())
    }

    pub fn serialization(msg: impl Into<String>) -> Self {
        RalphError::Serialization(msg.into())
    }
}

impl From<std::io::Error> for RalphError {
    fn from(err: std::io::Error) -> Self {
        RalphError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for RalphError {
    fn from(err: serde_json::Error) -> Self {
        RalphError::Serialization(err.to_string())
    }
}

/// Result type for Ralph operations
pub type RalphResult<T> = std::result::Result<T, RalphError>;

/// Configuration for a Ralph Loop session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RalphConfig {
    /// Maximum number of iterations (default: 20)
    pub max_iterations: u32,

    /// Timeout per iteration in seconds (default: 300 = 5 minutes)
    pub timeout_per_iteration_secs: u64,

    /// Number of consecutive failures before circuit breaker triggers (default: 3)
    pub circuit_breaker_threshold: u32,

    /// Patterns in output that signal completion
    #[serde(default)]
    pub completion_signals: Vec<String>,

    /// Whether to auto-commit after successful iterations
    #[serde(default)]
    pub auto_commit: bool,
}

impl Default for RalphConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            timeout_per_iteration_secs: 300, // 5 minutes
            circuit_breaker_threshold: 3,
            completion_signals: vec![
                "LOOP_COMPLETE".to_string(),
                "TASK_COMPLETE".to_string(),
                "DONE".to_string(),
            ],
            auto_commit: false,
        }
    }
}

impl RalphConfig {
    /// Create a new config with specified max iterations
    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = max;
        self
    }

    /// Create a new config with specified timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_per_iteration_secs = secs;
        self
    }

    /// Get timeout as Duration
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_secs(self.timeout_per_iteration_secs)
    }
}

/// Status of the Ralph Loop
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RalphStatus {
    /// Not running
    Idle,
    /// Currently executing an iteration
    Running,
    /// Paused between iterations
    Paused,
    /// Completed successfully
    Complete,
    /// Stopped due to error
    Error,
}

impl RalphStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, RalphStatus::Running | RalphStatus::Paused)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, RalphStatus::Complete | RalphStatus::Error | RalphStatus::Idle)
    }
}

/// Status of a single iteration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IterationStatus {
    /// Iteration completed successfully
    Success,
    /// Iteration failed
    Failed,
    /// Iteration timed out
    Timeout,
    /// Iteration was cancelled
    Cancelled,
}

/// Reason the loop completed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompletionReason {
    /// Task was completed (detected via complete.txt or output signal)
    TaskComplete,
    /// Max iterations reached without completion
    MaxIterations,
    /// User stopped the loop
    UserStopped,
    /// Error occurred
    Error { message: String },
    /// Circuit breaker triggered
    CircuitBreaker,
}

/// Record of a completed iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IterationRecord {
    /// Iteration number (1-indexed)
    pub iteration: u32,
    /// Status of this iteration
    pub status: IterationStatus,
    /// Summary of work done
    pub summary: String,
    /// Duration in seconds
    pub duration_secs: f64,
    /// Timestamp when iteration completed
    pub completed_at: u64,
}

impl IterationRecord {
    pub fn new(iteration: u32, status: IterationStatus, summary: String, duration_secs: f64) -> Self {
        let completed_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            iteration,
            status,
            summary,
            duration_secs,
            completed_at,
        }
    }
}

/// State of an active Ralph Loop
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RalphState {
    /// Current task description
    pub task: String,

    /// Configuration for this loop
    pub config: RalphConfig,

    /// Current iteration number (0 = not started, 1+ = iteration number)
    pub current_iteration: u32,

    /// Current status
    pub status: RalphStatus,

    /// History of completed iterations
    pub iteration_history: Vec<IterationRecord>,

    /// Feedback for the next iteration (if any)
    pub pending_feedback: Option<String>,

    /// Last work summary
    pub last_summary: Option<String>,

    /// Consecutive failure count (for circuit breaker)
    pub consecutive_failures: u32,

    /// Timestamp when the loop started
    pub started_at: u64,

    /// Completion reason (if complete)
    pub completion_reason: Option<CompletionReason>,

    /// Working directory for this loop
    #[serde(skip)]
    pub working_dir: PathBuf,
}

impl RalphState {
    /// Create a new Ralph Loop state
    pub fn new(task: String, config: RalphConfig, working_dir: PathBuf) -> Self {
        let started_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            task,
            config,
            current_iteration: 0,
            status: RalphStatus::Idle,
            iteration_history: Vec::new(),
            pending_feedback: None,
            last_summary: None,
            consecutive_failures: 0,
            started_at,
            completion_reason: None,
            working_dir,
        }
    }

    /// Get the ralph state directory path
    pub fn state_dir(&self) -> PathBuf {
        self.working_dir.join(".axiom").join("ralph")
    }

    /// Start the loop (transition from Idle to Running)
    pub fn start(&mut self) -> RalphResult<()> {
        if self.status != RalphStatus::Idle {
            return Err(RalphError::AlreadyRunning);
        }
        self.status = RalphStatus::Running;
        self.current_iteration = 1;
        Ok(())
    }

    /// Begin next iteration
    pub fn begin_iteration(&mut self) -> RalphResult<u32> {
        if !self.status.is_active() {
            return Err(RalphError::NotRunning);
        }

        if self.current_iteration > self.config.max_iterations {
            return Err(RalphError::MaxIterationsReached(self.config.max_iterations));
        }

        self.status = RalphStatus::Running;
        Ok(self.current_iteration)
    }

    /// Complete current iteration
    pub fn complete_iteration(
        &mut self,
        status: IterationStatus,
        summary: String,
        duration_secs: f64,
    ) {
        let record = IterationRecord::new(self.current_iteration, status, summary.clone(), duration_secs);
        self.iteration_history.push(record);
        self.last_summary = Some(summary);

        // Update consecutive failures counter
        match status {
            IterationStatus::Success => {
                self.consecutive_failures = 0;
            }
            IterationStatus::Failed | IterationStatus::Timeout => {
                self.consecutive_failures += 1;
            }
            IterationStatus::Cancelled => {
                // Don't count cancellation as failure
            }
        }

        // Clear pending feedback after it's been consumed
        self.pending_feedback = None;

        // Increment iteration counter for next iteration
        self.current_iteration += 1;

        // Check if we should stop
        if self.current_iteration > self.config.max_iterations {
            self.mark_complete(CompletionReason::MaxIterations);
        } else if self.consecutive_failures >= self.config.circuit_breaker_threshold {
            self.mark_complete(CompletionReason::CircuitBreaker);
        }
    }

    /// Mark the loop as complete
    pub fn mark_complete(&mut self, reason: CompletionReason) {
        self.status = RalphStatus::Complete;
        self.completion_reason = Some(reason);
    }

    /// Mark the loop as errored
    pub fn mark_error(&mut self, message: String) {
        self.status = RalphStatus::Error;
        self.completion_reason = Some(CompletionReason::Error { message });
    }

    /// Stop the loop (user-initiated)
    pub fn stop(&mut self) {
        self.mark_complete(CompletionReason::UserStopped);
    }

    /// Set feedback for next iteration
    pub fn set_feedback(&mut self, feedback: String) {
        self.pending_feedback = Some(feedback);
    }

    /// Check if the loop should continue
    pub fn should_continue(&self) -> bool {
        self.status.is_active()
            && self.current_iteration <= self.config.max_iterations
            && self.consecutive_failures < self.config.circuit_breaker_threshold
    }

    /// Check if task is complete (via completion file)
    pub async fn check_completion_file(&self) -> bool {
        let complete_file = self.state_dir().join("complete.txt");
        fs::metadata(&complete_file).await.is_ok()
    }

    /// Check if output contains completion signals
    pub fn check_completion_signals(&self, output: &str) -> bool {
        self.config
            .completion_signals
            .iter()
            .any(|signal| output.contains(signal))
    }

    /// Save state to persistence directory (async)
    pub async fn save(&self) -> RalphResult<()> {
        let state_dir = self.state_dir();
        fs::create_dir_all(&state_dir).await?;

        // Write task.md
        fs::write(state_dir.join("task.md"), &self.task).await?;

        // Write iteration.txt
        fs::write(
            state_dir.join("iteration.txt"),
            self.current_iteration.to_string(),
        )
        .await?;

        // Write feedback.txt (if any)
        if let Some(ref feedback) = self.pending_feedback {
            fs::write(state_dir.join("feedback.txt"), feedback).await?;
        } else {
            // Remove feedback file if no feedback
            let feedback_path = state_dir.join("feedback.txt");
            if fs::metadata(&feedback_path).await.is_ok() {
                fs::remove_file(&feedback_path).await?;
            }
        }

        // Write work-summary.txt (if any)
        if let Some(ref summary) = self.last_summary {
            fs::write(state_dir.join("work-summary.txt"), summary).await?;
        }

        // Write state.json (full state for recovery)
        let state_json = serde_json::to_string_pretty(self)?;
        fs::write(state_dir.join("state.json"), state_json).await?;

        Ok(())
    }

    /// Save state to persistence directory (sync version)
    pub fn save_sync(&self) -> RalphResult<()> {
        use std::fs as sync_fs;

        let state_dir = self.state_dir();
        sync_fs::create_dir_all(&state_dir)?;

        // Write task.md
        sync_fs::write(state_dir.join("task.md"), &self.task)?;

        // Write iteration.txt
        sync_fs::write(
            state_dir.join("iteration.txt"),
            self.current_iteration.to_string(),
        )?;

        // Write feedback.txt (if any)
        if let Some(ref feedback) = self.pending_feedback {
            sync_fs::write(state_dir.join("feedback.txt"), feedback)?;
        } else {
            // Remove feedback file if no feedback
            let feedback_path = state_dir.join("feedback.txt");
            if feedback_path.exists() {
                sync_fs::remove_file(&feedback_path)?;
            }
        }

        // Write work-summary.txt (if any)
        if let Some(ref summary) = self.last_summary {
            sync_fs::write(state_dir.join("work-summary.txt"), summary)?;
        }

        // Write state.json (full state for recovery)
        let state_json = serde_json::to_string_pretty(self)?;
        sync_fs::write(state_dir.join("state.json"), state_json)?;

        Ok(())
    }

    /// Load state from persistence directory
    pub async fn load(working_dir: PathBuf) -> RalphResult<Option<Self>> {
        let state_dir = working_dir.join(".axiom").join("ralph");
        let state_file = state_dir.join("state.json");

        if !fs::metadata(&state_file).await.is_ok() {
            return Ok(None);
        }

        let content = fs::read_to_string(&state_file).await?;
        let mut state: RalphState = serde_json::from_str(&content)?;
        state.working_dir = working_dir;

        // Also check for external modifications
        // Read feedback.txt in case it was updated externally
        let feedback_path = state_dir.join("feedback.txt");
        if let Ok(feedback) = fs::read_to_string(&feedback_path).await {
            if !feedback.trim().is_empty() {
                state.pending_feedback = Some(feedback);
            }
        }

        Ok(Some(state))
    }

    /// Clear state (remove persistence directory)
    pub async fn clear(working_dir: &Path) -> RalphResult<()> {
        let state_dir = working_dir.join(".axiom").join("ralph");
        if fs::metadata(&state_dir).await.is_ok() {
            fs::remove_dir_all(&state_dir).await?;
        }
        Ok(())
    }

    /// Get elapsed time since loop started
    pub fn elapsed_secs(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.started_at)
    }

    /// Get total iterations completed
    pub fn completed_iterations(&self) -> u32 {
        self.iteration_history.len() as u32
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.iteration_history.is_empty() {
            return 0.0;
        }
        let successes = self
            .iteration_history
            .iter()
            .filter(|r| r.status == IterationStatus::Success)
            .count();
        (successes as f64 / self.iteration_history.len() as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config() -> RalphConfig {
        RalphConfig {
            max_iterations: 5,
            timeout_per_iteration_secs: 30,
            circuit_breaker_threshold: 2,
            completion_signals: vec!["DONE".to_string()],
            auto_commit: false,
        }
    }

    #[test]
    fn test_ralph_config_default() {
        let config = RalphConfig::default();
        assert_eq!(config.max_iterations, 20);
        assert_eq!(config.timeout_per_iteration_secs, 300);
        assert_eq!(config.circuit_breaker_threshold, 3);
    }

    #[test]
    fn test_ralph_config_builders() {
        let config = RalphConfig::default()
            .with_max_iterations(10)
            .with_timeout(60);
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.timeout_per_iteration_secs, 60);
    }

    #[test]
    fn test_ralph_state_new() {
        let state = RalphState::new(
            "Test task".to_string(),
            test_config(),
            PathBuf::from("/tmp"),
        );
        assert_eq!(state.task, "Test task");
        assert_eq!(state.current_iteration, 0);
        assert_eq!(state.status, RalphStatus::Idle);
        assert!(state.iteration_history.is_empty());
    }

    #[test]
    fn test_ralph_state_start() {
        let mut state = RalphState::new(
            "Test task".to_string(),
            test_config(),
            PathBuf::from("/tmp"),
        );

        assert!(state.start().is_ok());
        assert_eq!(state.status, RalphStatus::Running);
        assert_eq!(state.current_iteration, 1);

        // Can't start again
        assert!(matches!(state.start(), Err(RalphError::AlreadyRunning)));
    }

    #[test]
    fn test_ralph_state_complete_iteration() {
        let mut state = RalphState::new(
            "Test task".to_string(),
            test_config(),
            PathBuf::from("/tmp"),
        );
        state.start().unwrap();

        state.complete_iteration(IterationStatus::Success, "Did stuff".to_string(), 10.5);

        assert_eq!(state.iteration_history.len(), 1);
        assert_eq!(state.current_iteration, 2);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.last_summary, Some("Did stuff".to_string()));
    }

    #[test]
    fn test_ralph_state_circuit_breaker() {
        let mut state = RalphState::new(
            "Test task".to_string(),
            test_config(),
            PathBuf::from("/tmp"),
        );
        state.start().unwrap();

        // First failure
        state.complete_iteration(IterationStatus::Failed, "Error 1".to_string(), 1.0);
        assert!(state.should_continue());

        // Second failure - circuit breaker should trigger
        state.complete_iteration(IterationStatus::Failed, "Error 2".to_string(), 1.0);
        assert!(!state.should_continue());
        assert_eq!(state.status, RalphStatus::Complete);
        assert_eq!(
            state.completion_reason,
            Some(CompletionReason::CircuitBreaker)
        );
    }

    #[test]
    fn test_ralph_state_max_iterations() {
        let mut state = RalphState::new(
            "Test task".to_string(),
            RalphConfig::default().with_max_iterations(2),
            PathBuf::from("/tmp"),
        );
        state.start().unwrap();

        state.complete_iteration(IterationStatus::Success, "Iter 1".to_string(), 1.0);
        assert!(state.should_continue());

        state.complete_iteration(IterationStatus::Success, "Iter 2".to_string(), 1.0);
        assert!(!state.should_continue());
        assert_eq!(
            state.completion_reason,
            Some(CompletionReason::MaxIterations)
        );
    }

    #[test]
    fn test_ralph_state_completion_signals() {
        let state = RalphState::new(
            "Test task".to_string(),
            test_config(),
            PathBuf::from("/tmp"),
        );

        assert!(state.check_completion_signals("Task completed. DONE"));
        assert!(!state.check_completion_signals("Still working..."));
    }

    #[test]
    fn test_ralph_state_feedback() {
        let mut state = RalphState::new(
            "Test task".to_string(),
            test_config(),
            PathBuf::from("/tmp"),
        );

        state.set_feedback("Focus on error handling".to_string());
        assert_eq!(
            state.pending_feedback,
            Some("Focus on error handling".to_string())
        );

        state.start().unwrap();
        state.complete_iteration(IterationStatus::Success, "Done".to_string(), 1.0);

        // Feedback should be cleared after iteration
        assert!(state.pending_feedback.is_none());
    }

    #[test]
    fn test_ralph_state_success_rate() {
        let mut state = RalphState::new(
            "Test task".to_string(),
            test_config(),
            PathBuf::from("/tmp"),
        );
        state.start().unwrap();

        // No iterations yet
        assert_eq!(state.success_rate(), 0.0);

        // One success
        state.complete_iteration(IterationStatus::Success, "OK".to_string(), 1.0);
        assert_eq!(state.success_rate(), 100.0);

        // One failure (reset consecutive to allow)
        state.consecutive_failures = 0;
        state.status = RalphStatus::Running;
        state.completion_reason = None;
        state.complete_iteration(IterationStatus::Failed, "ERR".to_string(), 1.0);
        assert_eq!(state.success_rate(), 50.0);
    }

    #[tokio::test]
    async fn test_ralph_state_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path().to_path_buf();

        // Create and save state
        let mut state = RalphState::new(
            "Implement feature X".to_string(),
            test_config(),
            working_dir.clone(),
        );
        state.start().unwrap();
        state.set_feedback("Focus on tests".to_string());
        state.complete_iteration(IterationStatus::Success, "Added tests".to_string(), 15.0);
        state.save().await.unwrap();

        // Load state
        let loaded = RalphState::load(working_dir.clone()).await.unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.task, "Implement feature X");
        assert_eq!(loaded.current_iteration, 2);
        assert_eq!(loaded.iteration_history.len(), 1);
        assert_eq!(loaded.last_summary, Some("Added tests".to_string()));

        // Clear state
        RalphState::clear(&working_dir).await.unwrap();
        let cleared = RalphState::load(working_dir).await.unwrap();
        assert!(cleared.is_none());
    }

    #[test]
    fn test_iteration_record() {
        let record = IterationRecord::new(
            1,
            IterationStatus::Success,
            "Completed successfully".to_string(),
            12.5,
        );

        assert_eq!(record.iteration, 1);
        assert_eq!(record.status, IterationStatus::Success);
        assert_eq!(record.summary, "Completed successfully");
        assert_eq!(record.duration_secs, 12.5);
        assert!(record.completed_at > 0);
    }

    #[test]
    fn test_ralph_status_predicates() {
        assert!(RalphStatus::Running.is_active());
        assert!(RalphStatus::Paused.is_active());
        assert!(!RalphStatus::Idle.is_active());
        assert!(!RalphStatus::Complete.is_active());

        assert!(RalphStatus::Complete.is_terminal());
        assert!(RalphStatus::Error.is_terminal());
        assert!(RalphStatus::Idle.is_terminal());
        assert!(!RalphStatus::Running.is_terminal());
    }
}
