import { getWorkspaces, addWorkspace, removeWorkspace } from '../src/lib/workspace-config';
import { listDirectory } from '../src/lib/fs';
import path from 'path';

async function test() {
  console.log('--- Testing Workspace Config ---');
  
  // 1. Initial Get
  let workspaces = await getWorkspaces();
  console.log('Initial workspaces:', workspaces.length);

  // 2. Add Workspace
  const newWs = await addWorkspace({
    title: 'Test Workspace',
    path: process.cwd(),
    type: 'local'
  });
  console.log('Added workspace:', newWs.id);

  // 3. Verify Add
  workspaces = await getWorkspaces();
  console.log('Workspaces after add:', workspaces.length);
  const found = workspaces.find(w => w.id === newWs.id);
  if (!found) throw new Error('Workspace not found after adding');
  console.log('Found added workspace:', found.title);

  // 4. Test File Listing
  console.log('\n--- Testing File System ---');
  const files = await listDirectory(process.cwd());
  console.log(`Listed ${files.length} files in ${process.cwd()}`);
  if (files.length === 0) console.warn('Warning: No files found?');
  const packageJson = files.find(f => f.name === 'package.json');
  if (packageJson) console.log('Found package.json!');
  else console.error('Error: package.json should exist');

  // 5. Clean up
  console.log('\n--- Cleaning up ---');
  await removeWorkspace(newWs.id);
  workspaces = await getWorkspaces();
  console.log('Workspaces after cleanup:', workspaces.length);
}

test().catch(console.error);
