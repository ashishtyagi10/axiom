'use server';

import { axiomApi } from '@/lib/api';
import type { RalphState, RalphConfig } from '@/lib/api/types';

export interface StartRalphLoopResult {
  success: boolean;
  state?: RalphState;
  error?: string;
}

export interface RalphStatusResult {
  state?: RalphState;
  error?: string;
}

export interface RalphActionResult {
  success: boolean;
  error?: string;
}

/**
 * Start a Ralph Loop for autonomous iterative development
 */
export async function startRalphLoopAction(
  workspaceId: string,
  task: string,
  config?: Partial<RalphConfig>
): Promise<StartRalphLoopResult> {
  try {
    const response = await axiomApi.startRalphLoop(workspaceId, task, config);
    return {
      success: response.success,
      state: response.state,
      error: response.error,
    };
  } catch (error) {
    console.error('Failed to start Ralph Loop:', error);
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Failed to start Ralph Loop',
    };
  }
}

/**
 * Stop the current Ralph Loop
 */
export async function stopRalphLoopAction(
  workspaceId: string
): Promise<RalphActionResult> {
  try {
    const response = await axiomApi.stopRalphLoop(workspaceId);
    return {
      success: response.success,
      error: response.error,
    };
  } catch (error) {
    console.error('Failed to stop Ralph Loop:', error);
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Failed to stop Ralph Loop',
    };
  }
}

/**
 * Get the current status of the Ralph Loop
 */
export async function getRalphStatusAction(
  workspaceId: string
): Promise<RalphStatusResult> {
  try {
    const response = await axiomApi.getRalphStatus(workspaceId);
    return {
      state: response.state,
      error: response.error,
    };
  } catch (error) {
    console.error('Failed to get Ralph status:', error);
    return {
      error: error instanceof Error ? error.message : 'Failed to get Ralph status',
    };
  }
}

/**
 * Update feedback for the next Ralph Loop iteration
 */
export async function updateRalphFeedbackAction(
  workspaceId: string,
  feedback: string
): Promise<RalphActionResult> {
  try {
    const response = await axiomApi.updateRalphFeedback(workspaceId, feedback);
    return {
      success: response.success,
      error: response.error,
    };
  } catch (error) {
    console.error('Failed to update Ralph feedback:', error);
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Failed to update feedback',
    };
  }
}
