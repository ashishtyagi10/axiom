'use client';

import React, { useState } from 'react';
import {
  Play,
  Square,
  Pause,
  RotateCcw,
  CheckCircle2,
  XCircle,
  Clock,
  AlertTriangle,
  Loader2,
  ChevronDown,
  ChevronRight,
  Zap
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { RalphState, RalphStatus, IterationRecord, IterationStatus, CompletionReason } from '@/lib/api/types';

interface RalphPanelProps {
  state: RalphState | null;
  onStart?: (task: string) => void;
  onStop?: () => void;
  onUpdateFeedback?: (feedback: string) => void;
  isStarting?: boolean;
}

export function RalphPanel({
  state,
  onStart,
  onStop,
  onUpdateFeedback,
  isStarting = false
}: RalphPanelProps) {
  const [taskInput, setTaskInput] = useState('');
  const [feedbackInput, setFeedbackInput] = useState('');
  const [showHistory, setShowHistory] = useState(true);

  const handleStart = () => {
    if (taskInput.trim() && onStart) {
      onStart(taskInput.trim());
      setTaskInput('');
    }
  };

  const handleFeedbackSubmit = () => {
    if (feedbackInput.trim() && onUpdateFeedback) {
      onUpdateFeedback(feedbackInput.trim());
      setFeedbackInput('');
    }
  };

  // Idle state - show input form
  if (!state || state.status === 'Idle') {
    return (
      <div className="flex flex-col h-full">
        <div className="p-4 border-b border-outline-variant">
          <div className="flex items-center gap-2">
            <Zap size={20} className="text-primary" />
            <h3 className="font-medium text-foreground">Ralph Loop</h3>
          </div>
        </div>

        <div className="flex-1 p-4 flex flex-col justify-center">
          <div className="space-y-4">
            <div className="text-center space-y-2 mb-6">
              <div className="w-16 h-16 mx-auto bg-surface-container rounded-full flex items-center justify-center">
                <Zap size={32} className="text-outline" />
              </div>
              <p className="text-sm text-outline">
                Start an autonomous development loop
              </p>
            </div>

            <div className="space-y-3">
              <textarea
                value={taskInput}
                onChange={(e) => setTaskInput(e.target.value)}
                placeholder="Describe the task for Ralph to work on..."
                className="w-full bg-surface-container border border-outline-variant rounded-xl p-3 text-sm resize-none h-24 focus:border-primary focus:outline-none transition-colors"
              />

              <button
                onClick={handleStart}
                disabled={!taskInput.trim() || isStarting}
                className={cn(
                  "w-full py-3 rounded-xl text-sm font-medium transition-all flex items-center justify-center gap-2",
                  taskInput.trim() && !isStarting
                    ? "bg-primary text-white hover:shadow-lg"
                    : "bg-surface-container text-outline cursor-not-allowed"
                )}
              >
                {isStarting ? (
                  <>
                    <Loader2 size={18} className="animate-spin" />
                    Starting...
                  </>
                ) : (
                  <>
                    <Play size={18} />
                    Start Ralph Loop
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  // Active or complete state
  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="p-4 border-b border-outline-variant">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {state.status === 'Running' ? (
              <Loader2 size={20} className="text-primary animate-spin" />
            ) : (
              <Zap size={20} className="text-primary" />
            )}
            <h3 className="font-medium text-foreground">Ralph Loop</h3>
          </div>
          <StatusBadge status={state.status} />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {/* Progress Section */}
        <div className="bg-surface-container rounded-xl p-4 space-y-3">
          <div className="flex items-center justify-between text-sm">
            <span className="text-outline">Progress</span>
            <span className="font-medium text-foreground">
              {state.current_iteration} / {state.config.max_iterations}
            </span>
          </div>

          <div className="w-full bg-surface-container-highest rounded-full h-2">
            <div
              className="bg-primary h-2 rounded-full transition-all duration-500"
              style={{
                width: `${(state.current_iteration / state.config.max_iterations) * 100}%`
              }}
            />
          </div>

          <div className="flex items-center justify-between text-xs text-outline">
            <span>Elapsed: {formatDuration(getElapsedSeconds(state.started_at))}</span>
            <span>Success Rate: {calculateSuccessRate(state.iteration_history)}%</span>
          </div>
        </div>

        {/* Task Description */}
        <div className="bg-surface-container-low rounded-xl p-4">
          <p className="text-xs text-outline mb-1">Task</p>
          <p className="text-sm text-foreground">{state.task}</p>
        </div>

        {/* Completion Reason (if complete/error) */}
        {state.completion_reason && (
          <CompletionReasonCard reason={state.completion_reason} />
        )}

        {/* Feedback Input (for running state) */}
        {state.status === 'Running' && onUpdateFeedback && (
          <div className="bg-surface-container rounded-xl p-4 space-y-2">
            <p className="text-xs text-outline">Add feedback for next iteration</p>
            <div className="flex gap-2">
              <input
                type="text"
                value={feedbackInput}
                onChange={(e) => setFeedbackInput(e.target.value)}
                placeholder="Optional guidance..."
                className="flex-1 bg-surface border border-outline-variant rounded-lg px-3 py-2 text-sm focus:border-primary focus:outline-none"
              />
              <button
                onClick={handleFeedbackSubmit}
                disabled={!feedbackInput.trim()}
                className="px-3 py-2 bg-surface-container-high rounded-lg text-sm text-primary hover:bg-surface-container-highest disabled:opacity-50 transition-colors"
              >
                Send
              </button>
            </div>
          </div>
        )}

        {/* Iteration History */}
        {state.iteration_history.length > 0 && (
          <div className="space-y-2">
            <button
              onClick={() => setShowHistory(!showHistory)}
              className="flex items-center gap-2 text-sm text-outline hover:text-foreground transition-colors"
            >
              {showHistory ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
              Iteration History ({state.iteration_history.length})
            </button>

            {showHistory && (
              <div className="space-y-2 max-h-60 overflow-y-auto">
                {[...state.iteration_history].reverse().map((record, idx) => (
                  <IterationCard key={idx} record={record} />
                ))}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Control Buttons */}
      {(state.status === 'Running' || state.status === 'Paused') && onStop && (
        <div className="p-4 border-t border-outline-variant">
          <button
            onClick={onStop}
            className="w-full py-2 bg-error/10 text-error rounded-xl text-sm font-medium hover:bg-error/20 transition-colors flex items-center justify-center gap-2"
          >
            <Square size={16} />
            Stop Loop
          </button>
        </div>
      )}

      {/* Restart Button (for complete state) */}
      {(state.status === 'Complete' || state.status === 'Error') && onStart && (
        <div className="p-4 border-t border-outline-variant">
          <button
            onClick={() => {
              if (onStart) {
                // Reset to show input form
                onStart('');
              }
            }}
            className="w-full py-2 bg-surface-container text-foreground rounded-xl text-sm font-medium hover:bg-surface-container-high transition-colors flex items-center justify-center gap-2"
          >
            <RotateCcw size={16} />
            Start New Loop
          </button>
        </div>
      )}
    </div>
  );
}

// Helper Components

function StatusBadge({ status }: { status: RalphStatus }) {
  const config = {
    Idle: { color: 'bg-outline/10 text-outline', label: 'Idle' },
    Running: { color: 'bg-primary/10 text-primary', label: 'Running' },
    Paused: { color: 'bg-warning/10 text-warning', label: 'Paused' },
    Complete: { color: 'bg-success/10 text-success', label: 'Complete' },
    Error: { color: 'bg-error/10 text-error', label: 'Error' },
  };

  const { color, label } = config[status];

  return (
    <span className={cn("px-2 py-1 rounded-full text-xs font-medium", color)}>
      {label}
    </span>
  );
}

function IterationCard({ record }: { record: IterationRecord }) {
  const statusConfig: Record<IterationStatus, { icon: React.ReactNode; color: string }> = {
    Success: { icon: <CheckCircle2 size={14} />, color: 'text-success' },
    Failed: { icon: <XCircle size={14} />, color: 'text-error' },
    Timeout: { icon: <Clock size={14} />, color: 'text-warning' },
    Cancelled: { icon: <Square size={14} />, color: 'text-outline' },
  };

  const { icon, color } = statusConfig[record.status];

  return (
    <div className="bg-surface-container-low rounded-lg p-3 space-y-1">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className={color}>{icon}</span>
          <span className="text-xs font-medium text-foreground">
            Iteration #{record.iteration}
          </span>
        </div>
        <span className="text-xs text-outline">
          {formatDuration(record.duration_secs)}
        </span>
      </div>
      <p className="text-xs text-outline line-clamp-2">{record.summary}</p>
    </div>
  );
}

function CompletionReasonCard({ reason }: { reason: CompletionReason }) {
  const config: Record<string, { icon: React.ReactNode; color: string; message: string }> = {
    TaskComplete: {
      icon: <CheckCircle2 size={20} />,
      color: 'text-success bg-success/10',
      message: 'Task completed successfully',
    },
    MaxIterations: {
      icon: <AlertTriangle size={20} />,
      color: 'text-warning bg-warning/10',
      message: 'Maximum iterations reached',
    },
    UserStopped: {
      icon: <Square size={20} />,
      color: 'text-outline bg-surface-container',
      message: 'Stopped by user',
    },
    CircuitBreaker: {
      icon: <Zap size={20} />,
      color: 'text-error bg-error/10',
      message: 'Circuit breaker triggered (too many failures)',
    },
    Error: {
      icon: <XCircle size={20} />,
      color: 'text-error bg-error/10',
      message: reason.type === 'Error' ? reason.message : 'An error occurred',
    },
  };

  const { icon, color, message } = config[reason.type] || config.Error;

  return (
    <div className={cn("rounded-xl p-4 flex items-start gap-3", color.split(' ')[1])}>
      <span className={color.split(' ')[0]}>{icon}</span>
      <div>
        <p className={cn("text-sm font-medium", color.split(' ')[0])}>
          {reason.type === 'Error' ? 'Error' : reason.type.replace(/([A-Z])/g, ' $1').trim()}
        </p>
        <p className="text-xs text-outline mt-1">{message}</p>
      </div>
    </div>
  );
}

// Helper Functions

function formatDuration(secs: number): string {
  const seconds = Math.floor(secs);
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) {
    const mins = Math.floor(seconds / 60);
    const remainingSecs = seconds % 60;
    return `${mins}m ${remainingSecs}s`;
  }
  const hours = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  return `${hours}h ${mins}m`;
}

function getElapsedSeconds(startedAt: number): number {
  return Math.floor((Date.now() - startedAt) / 1000);
}

function calculateSuccessRate(history: IterationRecord[]): number {
  if (history.length === 0) return 0;
  const successful = history.filter(r => r.status === 'Success').length;
  return Math.round((successful / history.length) * 100);
}
