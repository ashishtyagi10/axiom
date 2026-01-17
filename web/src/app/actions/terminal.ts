'use server';

import { axiomApi } from '@/lib/api';

export interface CommandResult {
  stdout: string;
  stderr: string;
  exitCode: number;
}

export async function runCommandAction(
  command: string,
  workspaceId: string
): Promise<CommandResult> {
  try {
    const result = await axiomApi.runCommand(workspaceId, command);
    return {
      stdout: result.stdout,
      stderr: result.stderr,
      exitCode: result.exit_code,
    };
  } catch (error: any) {
    return {
      stdout: '',
      stderr: error.message || 'Command execution failed',
      exitCode: 1,
    };
  }
}
