'use client';

import React, { useState, useEffect } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Folder, ChevronUp, Loader2, HardDrive } from 'lucide-react';
import { cn } from '@/lib/utils';
import { listDirectories, getHomeDirectory, DirectoryEntry } from '@/app/actions/files';

interface FilePickerModalProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onSelect: (path: string) => void;
}

export function FilePickerModal({ isOpen, onOpenChange, onSelect }: FilePickerModalProps) {
  const [currentPath, setCurrentPath] = useState<string>('');
  const [entries, setEntries] = useState<DirectoryEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Initial load: Go to home directory
  useEffect(() => {
    if (isOpen && !currentPath) {
      loadHomeDir();
    }
  }, [isOpen]);

  const loadHomeDir = async () => {
    try {
      setIsLoading(true);
      const home = await getHomeDirectory();
      setCurrentPath(home);
      await loadDirectory(home);
    } catch (err) {
      setError('Failed to load home directory');
    } finally {
      setIsLoading(false);
    }
  };

  const loadDirectory = async (path: string) => {
    try {
      setIsLoading(true);
      setError(null);
      const dirs = await listDirectories(path);
      setEntries(dirs);
      setCurrentPath(path);
    } catch (err) {
      setError('Failed to load directory. Access denied?');
    } finally {
      setIsLoading(false);
    }
  };

  const handleNavigate = (path: string) => {
    loadDirectory(path);
  };

  const handleUp = () => {
    if (!currentPath) return;
    // Simple parent resolution for Unix-like systems (MacOS)
    // For Windows we might need more robust handling, but this suffices for the current environment
    const parent = currentPath.split('/').slice(0, -1).join('/') || '/';
    loadDirectory(parent);
  };

  const handleConfirm = () => {
    onSelect(currentPath);
    onOpenChange(false);
  };

  return (
    <Dialog.Root open={isOpen} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/40 backdrop-blur-sm z-50 animate-in fade-in duration-200" />
        <Dialog.Content className="fixed left-[50%] top-[50%] translate-x-[-50%] translate-y-[-50%] w-full max-w-2xl bg-surface rounded-2xl shadow-2xl z-50 overflow-hidden animate-in zoom-in-95 duration-200 flex flex-col max-h-[80vh]">
          <div className="flex items-center justify-between p-4 border-b border-outline-variant shrink-0">
            <Dialog.Title className="text-xl font-normal">Select Workspace Directory</Dialog.Title>
            <Dialog.Close className="p-2 hover:bg-surface-container rounded-full transition-colors">
              <X size={20} />
            </Dialog.Close>
          </div>

          {/* Current Path Bar */}
          <div className="flex items-center gap-2 p-3 bg-surface-container-low border-b border-outline-variant shrink-0">
            <button 
              onClick={handleUp}
              className="p-2 hover:bg-surface-container rounded-lg text-outline disabled:opacity-30"
              disabled={currentPath === '/'}
              title="Go Up"
            >
              <ChevronUp size={20} />
            </button>
            <div className="flex-1 px-3 py-1.5 bg-surface rounded-md border border-outline-variant text-sm font-mono truncate text-outline">
              {currentPath || 'Loading...'}
            </div>
          </div>

          {/* Directory List */}
          <div className="flex-1 overflow-y-auto p-4 min-h-[300px]">
            {isLoading ? (
              <div className="flex flex-col items-center justify-center h-full text-outline gap-2">
                <Loader2 size={24} className="animate-spin" />
                <span>Loading directories...</span>
              </div>
            ) : error ? (
              <div className="flex flex-col items-center justify-center h-full text-red-500 gap-2">
                <span>{error}</span>
                <button onClick={() => loadHomeDir()} className="text-sm underline">Go Home</button>
              </div>
            ) : (
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                {entries.map((dir) => (
                  <button
                    key={dir.path}
                    onClick={() => handleNavigate(dir.path)}
                    className="flex items-center gap-3 p-3 rounded-xl hover:bg-surface-container-high transition-colors text-left group"
                  >
                    <div className="shrink-0 text-primary">
                      <Folder size={20} />
                    </div>
                    <span className="text-sm text-foreground truncate">{dir.name}</span>
                  </button>
                ))}
                {entries.length === 0 && (
                  <div className="col-span-full flex flex-col items-center justify-center py-10 text-outline">
                    <p>No subdirectories found.</p>
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Footer */}
          <div className="flex justify-between items-center gap-3 p-4 bg-surface-container-low border-t border-outline-variant shrink-0">
            <div className="text-xs text-outline px-2">
              <span className="font-semibold">Selected:</span> {currentPath.split('/').pop()}
            </div>
            <div className="flex gap-2">
                <Dialog.Close className="px-6 py-2 rounded-full text-sm font-medium hover:bg-surface-container transition-colors">
                Cancel
                </Dialog.Close>
                <button 
                onClick={handleConfirm}
                className="px-6 py-2 bg-primary text-white rounded-full text-sm font-medium hover:shadow-md transition-all disabled:opacity-50"
                disabled={!currentPath}
                >
                Select Folder
                </button>
            </div>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
