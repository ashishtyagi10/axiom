'use server';

import fs from 'fs/promises';
import path from 'path';
import os from 'os';

export interface DirectoryEntry {
  name: string;
  path: string;
  isDirectory: boolean;
}

/**
 * Get the user's home directory
 */
export async function getHomeDirectory(): Promise<string> {
  return os.homedir();
}

/**
 * List directories in a given path (for file picker, before workspace exists)
 */
export async function listDirectories(dirPath: string): Promise<DirectoryEntry[]> {
  try {
    const entries = await fs.readdir(dirPath, { withFileTypes: true });

    const directories: DirectoryEntry[] = [];

    for (const entry of entries) {
      // Skip hidden files/folders
      if (entry.name.startsWith('.')) continue;

      if (entry.isDirectory()) {
        directories.push({
          name: entry.name,
          path: path.join(dirPath, entry.name),
          isDirectory: true,
        });
      }
    }

    // Sort alphabetically
    directories.sort((a, b) => a.name.localeCompare(b.name));

    return directories;
  } catch (error) {
    console.error('Error listing directories:', error);
    return [];
  }
}
