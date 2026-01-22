//! Ralph Loop Executor
//!
//! The executor orchestrates iterations with fresh context per iteration,
//! manages state persistence, detects completion, handles feedback between
//! iterations, and enforces safety limits.

use super::state::{
    CompletionReason, IterationStatus, RalphError, RalphResult, RalphState, RalphStatus,
};
use crate::config::CliAgentConfig;
use crate::notifications::Notification;
use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::RwLock;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cancellation token for graceful shutdown
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent invoker trait for executing iterations
///
/// This abstraction allows for testing with mock implementations
/// and different agent backends (CLI agents, LLM agents, etc.)
pub trait AgentInvoker: Send + Sync {
    /// Invoke an agent with the given prompt and return its output
    ///
    /// Returns the agent's output when complete, or an error if execution fails.
    fn invoke(&self, prompt: &str, timeout: Duration) -> RalphResult<String>;
}

/// Signal from PTY read thread to indicate completion or output
enum PtySignal {
    /// PTY process exited
    Exit { exit_code: i32 },
    /// Data received from PTY (content parsed by vt100 directly)
    Output,
}

/// Default agent invoker that uses CLI agents
pub struct CliAgentInvoker {
    /// CLI agent configuration
    config: CliAgentConfig,

    /// Working directory
    cwd: PathBuf,
}

impl CliAgentInvoker {
    /// Create a new CLI agent invoker with the given agent ID
    ///
    /// Uses default configuration for the agent. If you have a custom config,
    /// use `with_config` instead.
    pub fn new(agent_id: impl Into<String>, cwd: PathBuf) -> Self {
        let agent_id = agent_id.into();
        // Create a default config with the agent ID as the command
        let config = CliAgentConfig {
            name: agent_id.clone(),
            command: agent_id,
            default_args: Vec::new(),
            use_cwd: true,
            env: std::collections::HashMap::new(),
            enabled: true,
            icon: "🤖".to_string(),
        };
        Self { config, cwd }
    }

    /// Create a new CLI agent invoker with a custom configuration
    pub fn with_config(config: CliAgentConfig, cwd: PathBuf) -> Self {
        Self { config, cwd }
    }
}

impl AgentInvoker for CliAgentInvoker {
    fn invoke(&self, prompt: &str, timeout: Duration) -> RalphResult<String> {
        let start = Instant::now();

        // Set up PTY size
        let size = PtySize {
            rows: 50,
            cols: 200,
            pixel_width: 0,
            pixel_height: 0,
        };

        // Create PTY system
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(size)
            .map_err(|e| RalphError::io(format!("Failed to create PTY: {}", e)))?;

        // Build the command
        let mut cmd = CommandBuilder::new(&self.config.command);

        // Add default arguments
        for arg in &self.config.default_args {
            cmd.arg(arg);
        }

        // Add the prompt as the final argument
        if !prompt.is_empty() {
            cmd.arg(prompt);
        }

        // Set working directory if configured
        if self.config.use_cwd {
            cmd.cwd(&self.cwd);
        }

        // Set any custom environment variables
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        // Spawn the CLI agent process
        pair.slave
            .spawn_command(cmd)
            .map_err(|e| RalphError::io(format!("Failed to spawn command '{}': {}", self.config.command, e)))?;

        // Clone reader for background thread
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| RalphError::io(format!("Failed to clone reader: {}", e)))?;

        // Create channel for PTY signals
        let (signal_tx, signal_rx): (Sender<PtySignal>, Receiver<PtySignal>) = bounded(1024);

        // Create vt100 parser for terminal emulation
        let parser = Arc::new(RwLock::new(vt100::Parser::new(size.rows, size.cols, 10000)));
        let parser_clone = parser.clone();

        // Spawn reader thread
        std::thread::spawn(move || {
            Self::read_loop(reader, parser_clone, signal_tx);
        });

        // Keep writer alive to prevent pipe closure
        let _writer = pair
            .master
            .take_writer()
            .map_err(|e| RalphError::io(format!("Failed to get writer: {}", e)))?;

        // Wait for completion or timeout
        let mut exit_code: Option<i32> = None;
        let _remaining_timeout = timeout.saturating_sub(start.elapsed());

        // Calculate per-recv timeout (check every second)
        let check_interval = Duration::from_secs(1);

        loop {
            // Check if we've exceeded the total timeout
            if start.elapsed() >= timeout {
                // Timeout - extract what we have so far
                let output = Self::extract_output(&parser);
                return Ok(format!(
                    "[Timeout after {:?}]\n\n{}",
                    timeout, output
                ));
            }

            // Try to receive with short timeout
            match signal_rx.recv_timeout(check_interval) {
                Ok(PtySignal::Exit { exit_code: code }) => {
                    exit_code = Some(code);
                    break;
                }
                Ok(PtySignal::Output) => {
                    // Data already processed by parser in read_loop, continue waiting
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // Continue waiting
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    // Reader thread exited unexpectedly
                    break;
                }
            }
        }

        // Give a brief moment for any final output to be processed
        std::thread::sleep(Duration::from_millis(100));

        // Extract the accumulated output
        let output = Self::extract_output(&parser);

        // Include exit code in output if non-zero
        if let Some(code) = exit_code {
            if code != 0 {
                return Ok(format!("{}\n\n[Exit code: {}]", output, code));
            }
        }

        Ok(output)
    }
}

impl CliAgentInvoker {
    /// Reader loop - runs in background thread, feeds data to parser and signals completion
    fn read_loop(
        mut reader: Box<dyn Read + Send>,
        parser: Arc<RwLock<vt100::Parser>>,
        tx: Sender<PtySignal>,
    ) {
        let mut buf = [0u8; 4096];

        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    // EOF - process exited
                    let _ = tx.send(PtySignal::Exit { exit_code: 0 });
                    break;
                }
                Ok(n) => {
                    let data = &buf[..n];

                    // Feed data to vt100 parser
                    parser.write().process(data);

                    // Signal that we received output
                    if tx.send(PtySignal::Output).is_err() {
                        // Channel closed
                        break;
                    }
                }
                Err(e) => {
                    // Read error
                    let code = e.raw_os_error().unwrap_or(-1);
                    let _ = tx.send(PtySignal::Exit { exit_code: code });
                    break;
                }
            }
        }
    }

    /// Extract text output from the vt100 parser
    fn extract_output(parser: &Arc<RwLock<vt100::Parser>>) -> String {
        let parser = parser.read();
        let screen = parser.screen();
        let (rows, cols) = screen.size();
        let mut lines: Vec<String> = Vec::new();

        // Get visible screen content
        for row in 0..rows {
            let mut line = String::new();
            for col in 0..cols {
                if let Some(cell) = screen.cell(row, col) {
                    let text = cell.contents();
                    if text.is_empty() {
                        line.push(' ');
                    } else {
                        line.push_str(&text);
                    }
                } else {
                    line.push(' ');
                }
            }
            // Trim trailing whitespace from each line
            let trimmed = line.trim_end();
            if !trimmed.is_empty() {
                lines.push(trimmed.to_string());
            }
        }

        // Join and trim the result
        lines.join("\n").trim().to_string()
    }
}

/// Mock agent invoker for testing
#[cfg(test)]
pub struct MockAgentInvoker {
    /// Responses to return for each iteration
    pub responses: Vec<RalphResult<String>>,
    /// Current iteration index
    index: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
impl MockAgentInvoker {
    pub fn new(responses: Vec<RalphResult<String>>) -> Self {
        Self {
            responses,
            index: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

#[cfg(test)]
impl AgentInvoker for MockAgentInvoker {
    fn invoke(&self, _prompt: &str, _timeout: Duration) -> RalphResult<String> {
        let idx = self.index.fetch_add(1, Ordering::SeqCst);
        if idx < self.responses.len() {
            self.responses[idx].clone()
        } else {
            Ok("DONE".to_string())
        }
    }
}

/// Ralph Loop executor
///
/// Orchestrates iterations until completion or limits are reached.
pub struct RalphExecutor {
    /// Current state
    state: Arc<RwLock<RalphState>>,

    /// Notification sender
    notification_tx: Sender<Notification>,

    /// Cancellation token
    cancel_token: CancellationToken,

    /// Agent invoker for executing iterations
    invoker: Arc<dyn AgentInvoker>,
}

impl RalphExecutor {
    /// Create a new Ralph executor
    pub fn new(
        state: RalphState,
        notification_tx: Sender<Notification>,
        invoker: Arc<dyn AgentInvoker>,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(state)),
            notification_tx,
            cancel_token: CancellationToken::new(),
            invoker,
        }
    }

    /// Get a clone of the cancellation token
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    /// Get the current state (for status queries)
    pub fn state(&self) -> RalphState {
        self.state.read().clone()
    }

    /// Update feedback for next iteration
    pub fn set_feedback(&self, feedback: String) {
        self.state.write().set_feedback(feedback);
    }

    /// Build the prompt for an iteration
    fn build_iteration_prompt(&self, state: &RalphState) -> String {
        let mut prompt = format!(
            "# Ralph Loop Iteration {}/{}\n\n## Task\n{}\n",
            state.current_iteration, state.config.max_iterations, state.task
        );

        // Add feedback from previous iteration if present
        if let Some(ref feedback) = state.pending_feedback {
            prompt.push_str(&format!("\n## Feedback from Previous Iteration\n{}\n", feedback));
        }

        // Add last summary if available
        if let Some(ref summary) = state.last_summary {
            prompt.push_str(&format!("\n## Previous Work Summary\n{}\n", summary));
        }

        // Add completion instructions
        prompt.push_str(
            "\n## Instructions\n\
            Work on the task. When the task is complete, output 'LOOP_COMPLETE' or create \
            a file at `.axiom/ralph/complete.txt`.\n",
        );

        prompt
    }

    /// Check if the loop should complete based on output
    fn check_completion(&self, state: &RalphState, output: &str) -> Option<CompletionReason> {
        // Check for completion signals in output
        if state.check_completion_signals(output) {
            return Some(CompletionReason::TaskComplete);
        }

        None
    }

    /// Run a single iteration
    pub fn run_iteration(&self) -> RalphResult<(IterationStatus, String)> {
        let iteration_start = Instant::now();

        // Get current state and build prompt
        let (iteration, prompt, timeout) = {
            let mut state = self.state.write();
            let iteration = state.begin_iteration().ok().unwrap_or(state.current_iteration);
            let prompt = self.build_iteration_prompt(&state);
            let timeout = state.config.timeout_duration();
            (iteration, prompt, timeout)
        };

        // Emit iteration started notification
        let task = self.state.read().task.clone();
        let _ = self
            .notification_tx
            .send(Notification::ralph_iteration_started(iteration, &task));

        // Check for cancellation before invoking
        if self.cancel_token.is_cancelled() {
            let duration = iteration_start.elapsed().as_secs_f64();
            let summary = "Iteration cancelled by user".to_string();
            self.state
                .write()
                .complete_iteration(IterationStatus::Cancelled, summary.clone(), duration);
            return Ok((IterationStatus::Cancelled, summary));
        }

        // Invoke the agent
        let result = self.invoker.invoke(&prompt, timeout);

        let duration = iteration_start.elapsed().as_secs_f64();

        match result {
            Ok(output) => {
                // Determine status based on output
                let status = IterationStatus::Success;
                let summary = Self::extract_summary(&output);

                // Update state
                self.state.write().complete_iteration(status, summary.clone(), duration);

                // Save state to disk (sync)
                let state = self.state.read().clone();
                let _ = state.save_sync();

                // Emit iteration complete notification
                let _ = self.notification_tx.send(Notification::ralph_iteration_complete(
                    iteration,
                    &summary,
                    status,
                ));

                // Check for completion - get read lock, check, release, then get write lock if needed
                let completion_reason = {
                    let state = self.state.read();
                    self.check_completion(&state, &output)
                };
                if let Some(reason) = completion_reason {
                    self.state.write().mark_complete(reason);
                }

                Ok((status, summary))
            }
            Err(e) => {
                let summary = format!("Error: {}", e);
                self.state
                    .write()
                    .complete_iteration(IterationStatus::Failed, summary.clone(), duration);

                // Emit error notification
                let _ = self
                    .notification_tx
                    .send(Notification::ralph_loop_error(iteration, &summary));

                Ok((IterationStatus::Failed, summary))
            }
        }
    }

    /// Extract a summary from agent output
    fn extract_summary(output: &str) -> String {
        // Take the last few meaningful lines as a summary
        let lines: Vec<&str> = output
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();

        if lines.is_empty() {
            return "No output".to_string();
        }

        // Take last 3 lines or all if fewer
        let summary_lines = if lines.len() > 3 {
            &lines[lines.len() - 3..]
        } else {
            &lines[..]
        };

        summary_lines.join("\n")
    }

    /// Run the complete loop
    ///
    /// This method runs iterations until:
    /// - Task is complete (complete.txt or output signal)
    /// - Max iterations reached
    /// - User stops the loop
    /// - Circuit breaker triggers
    pub fn run(&self) -> RalphResult<CompletionReason> {
        // Start the loop if idle
        {
            let mut state = self.state.write();
            if state.status == RalphStatus::Idle {
                state.start()?;
            }
        }

        loop {
            // Check cancellation
            if self.cancel_token.is_cancelled() {
                self.state.write().stop();
                let total = self.state.read().completed_iterations();
                let _ = self.notification_tx.send(Notification::ralph_loop_complete(
                    total,
                    CompletionReason::UserStopped,
                ));
                return Ok(CompletionReason::UserStopped);
            }

            // Check if we should continue
            {
                let state = self.state.read();
                if !state.should_continue() {
                    let reason = state
                        .completion_reason
                        .clone()
                        .unwrap_or(CompletionReason::MaxIterations);
                    let total = state.completed_iterations();
                    let _ = self
                        .notification_tx
                        .send(Notification::ralph_loop_complete(total, reason.clone()));
                    return Ok(reason);
                }
            }

            // Check for completion file (sync check using std::fs)
            {
                let state = self.state.read();
                let complete_file = state.state_dir().join("complete.txt");
                if complete_file.exists() {
                    drop(state); // Release read lock before write
                    self.state
                        .write()
                        .mark_complete(CompletionReason::TaskComplete);
                    let total = self.state.read().completed_iterations();
                    let _ = self.notification_tx.send(Notification::ralph_loop_complete(
                        total,
                        CompletionReason::TaskComplete,
                    ));
                    return Ok(CompletionReason::TaskComplete);
                }
            }

            // Run iteration
            let (status, _summary) = self.run_iteration()?;

            // Check if iteration failure should stop the loop
            if status == IterationStatus::Cancelled {
                let total = self.state.read().completed_iterations();
                let _ = self.notification_tx.send(Notification::ralph_loop_complete(
                    total,
                    CompletionReason::UserStopped,
                ));
                return Ok(CompletionReason::UserStopped);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ralph::state::{RalphConfig, RalphError};
    use crossbeam_channel::unbounded;
    use tempfile::TempDir;

    fn create_test_executor(
        invoker: Arc<dyn AgentInvoker>,
    ) -> (RalphExecutor, crossbeam_channel::Receiver<Notification>) {
        let temp_dir = TempDir::new().unwrap();
        let config = RalphConfig::default().with_max_iterations(5);
        let state = RalphState::new(
            "Test task".to_string(),
            config,
            temp_dir.path().to_path_buf(),
        );
        let (tx, rx) = unbounded();
        let executor = RalphExecutor::new(state, tx, invoker);
        (executor, rx)
    }

    #[test]
    fn test_build_iteration_prompt() {
        let invoker = Arc::new(MockAgentInvoker::new(vec![]));
        let (executor, _rx) = create_test_executor(invoker);

        executor.state.write().start().unwrap();
        let prompt = executor.build_iteration_prompt(&executor.state.read());

        assert!(prompt.contains("Iteration 1/5"));
        assert!(prompt.contains("Test task"));
        assert!(prompt.contains("LOOP_COMPLETE"));
    }

    #[test]
    fn test_iteration_with_success() {
        let invoker = Arc::new(MockAgentInvoker::new(vec![
            Ok("Work completed successfully\nDONE".to_string()),
        ]));
        let (executor, rx) = create_test_executor(invoker);

        executor.state.write().start().unwrap();
        let (status, summary) = executor.run_iteration().unwrap();

        assert_eq!(status, IterationStatus::Success);
        assert!(summary.contains("Work completed") || summary.contains("DONE"));

        // Check notification was sent
        let notif = rx.try_recv().unwrap();
        assert!(matches!(notif, Notification::RalphIterationStarted { .. }));
    }

    #[test]
    fn test_iteration_with_error() {
        let invoker = Arc::new(MockAgentInvoker::new(vec![Err(RalphError::state(
            "Agent failed",
        ))]));
        let (executor, _rx) = create_test_executor(invoker);

        executor.state.write().start().unwrap();
        let (status, summary) = executor.run_iteration().unwrap();

        assert_eq!(status, IterationStatus::Failed);
        assert!(summary.contains("Error"));
    }

    #[test]
    fn test_cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_iteration_cancelled() {
        let invoker = Arc::new(MockAgentInvoker::new(vec![Ok("output".to_string())]));
        let (executor, _rx) = create_test_executor(invoker);

        executor.state.write().start().unwrap();
        executor.cancel_token.cancel();

        let (status, _) = executor.run_iteration().unwrap();
        assert_eq!(status, IterationStatus::Cancelled);
    }

    #[test]
    fn test_completion_detection() {
        let invoker = Arc::new(MockAgentInvoker::new(vec![]));
        let (executor, _rx) = create_test_executor(invoker);

        let state = executor.state.read();

        // Should detect LOOP_COMPLETE signal
        let reason = executor.check_completion(&state, "Task done. LOOP_COMPLETE");
        assert_eq!(reason, Some(CompletionReason::TaskComplete));

        // Should not complete on normal output
        let reason = executor.check_completion(&state, "Still working...");
        assert_eq!(reason, None);
    }

    #[test]
    fn test_extract_summary() {
        let output = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        let summary = RalphExecutor::extract_summary(output);
        assert!(summary.contains("Line 3"));
        assert!(summary.contains("Line 5"));
        assert!(!summary.contains("Line 1"));

        let short = "Only one line";
        let summary = RalphExecutor::extract_summary(short);
        assert_eq!(summary, "Only one line");

        let empty = "";
        let summary = RalphExecutor::extract_summary(empty);
        assert_eq!(summary, "No output");
    }

    #[test]
    fn test_feedback_integration() {
        let invoker = Arc::new(MockAgentInvoker::new(vec![]));
        let (executor, _rx) = create_test_executor(invoker);

        executor.set_feedback("Focus on error handling".to_string());

        executor.state.write().start().unwrap();
        let prompt = executor.build_iteration_prompt(&executor.state.read());

        assert!(prompt.contains("Focus on error handling"));
    }

    #[test]
    fn test_run_loop_with_completion() {
        // Create invoker that returns DONE on second iteration
        let invoker = Arc::new(MockAgentInvoker::new(vec![
            Ok("Working...".to_string()),
            Ok("Task finished. DONE".to_string()),
        ]));
        let (executor, _rx) = create_test_executor(invoker);

        let reason = executor.run().unwrap();
        assert_eq!(reason, CompletionReason::TaskComplete);
    }

    #[test]
    fn test_run_loop_with_max_iterations() {
        // Create invoker that never completes
        let invoker = Arc::new(MockAgentInvoker::new(vec![
            Ok("Working 1".to_string()),
            Ok("Working 2".to_string()),
            Ok("Working 3".to_string()),
            Ok("Working 4".to_string()),
            Ok("Working 5".to_string()),
            Ok("Working 6".to_string()), // Beyond max
        ]));
        let (executor, _rx) = create_test_executor(invoker);

        let reason = executor.run().unwrap();
        assert_eq!(reason, CompletionReason::MaxIterations);
    }

    #[test]
    fn test_run_loop_with_cancellation() {
        let invoker = Arc::new(MockAgentInvoker::new(vec![
            Ok("Working...".to_string()),
            Ok("Still working...".to_string()),
        ]));
        let (executor, _rx) = create_test_executor(invoker);

        // Cancel before starting
        executor.cancel_token.cancel();

        let reason = executor.run().unwrap();
        assert_eq!(reason, CompletionReason::UserStopped);
    }

    #[test]
    fn test_circuit_breaker() {
        // Set up with low circuit breaker threshold
        let temp_dir = TempDir::new().unwrap();
        let config = RalphConfig {
            max_iterations: 10,
            circuit_breaker_threshold: 2,
            ..RalphConfig::default()
        };
        let state = RalphState::new(
            "Test task".to_string(),
            config,
            temp_dir.path().to_path_buf(),
        );
        let (tx, _rx) = unbounded();

        // Invoker that always fails
        let invoker = Arc::new(MockAgentInvoker::new(vec![
            Err(RalphError::state("Fail 1")),
            Err(RalphError::state("Fail 2")),
        ]));

        let executor = RalphExecutor::new(state, tx, invoker);

        let reason = executor.run().unwrap();
        assert_eq!(reason, CompletionReason::CircuitBreaker);
    }
}
