import fs from 'fs/promises';
import path from 'path';
import os from 'os';

export interface FileEntry {
  name: string;
  path: string;
  isDirectory: boolean;
  size?: number;
  lastModified?: Date;
}

/**
 * Lists files and directories in a given path.
 */
export async function listDirectory(dirPath: string): Promise<FileEntry[]> {
  try {
    const stats = await fs.stat(dirPath);
    if (!stats.isDirectory()) {
      throw new Error(`Path is not a directory: ${dirPath}`);
    }

    const entries = await fs.readdir(dirPath, { withFileTypes: true });
    
    const fileEntries = await Promise.all(entries.map(async (entry) => {
      const fullPath = path.join(dirPath, entry.name);
      // Skip hidden files/folders for now (basic filter)
      if (entry.name.startsWith('.')) return null;

      return {
        name: entry.name,
        path: fullPath,
        isDirectory: entry.isDirectory(),
      };
    }));

    return fileEntries.filter((entry): entry is FileEntry => entry !== null);
  } catch (error) {
    console.error('Error listing directory:', error);
    throw error;
  }
}

/**
 * Ensures the configuration directory exists.
 */
export async function ensureConfigDir() {
  const homeDir = os.homedir();
  const configDir = path.join(homeDir, '.osmo');
  
  try {
    await fs.access(configDir);
  } catch {
    await fs.mkdir(configDir, { recursive: true });
  }
  
  return configDir;
}

export async function getHomeDir(): Promise<string> {
  return os.homedir();
}

export async function readFile(filePath: string): Promise<string> {
  try {
    return await fs.readFile(filePath, 'utf-8');
  } catch (error) {
    console.error(`Error reading file ${filePath}:`, error);
    throw error;
  }
}

export async function writeFile(filePath: string, content: string): Promise<void> {
  try {
    await fs.mkdir(path.dirname(filePath), { recursive: true });
    await fs.writeFile(filePath, content, 'utf-8');
  } catch (error) {
    console.error(`Error writing file ${filePath}:`, error);
    throw error;
  }
}
