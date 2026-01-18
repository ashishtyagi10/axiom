'use server';

import { axiomApi } from '@/lib/api/client';
import type { SlashCommandResult } from '@/lib/api/types';

/**
 * Execute a slash command in the specified workspace.
 *
 * @param workspaceId - The workspace to execute the command in
 * @param command - The raw command string (e.g., "/help", "/init")
 * @returns The result of executing the command
 */
export async function slashCommandAction(
  workspaceId: string,
  command: string
): Promise<SlashCommandResult> {
  try {
    const result = await axiomApi.executeSlashCommand(workspaceId, command);
    return result;
  } catch (error) {
    console.error('Slash command error:', error);
    return {
      type: 'Error',
      message: error instanceof Error ? error.message : 'Failed to execute command',
    };
  }
}
