'use client';

import React, { useState } from 'react';
import { ChevronRight, ChevronDown, File, Folder } from 'lucide-react';
import { cn } from '@/lib/utils';
import { FileEntry } from '@/lib/api/types';
import { listFilesAction } from '@/app/actions/workspace';

interface FileTreeProps {
  files: FileEntry[];
  workspaceId: string;
  onFileClick?: (file: FileEntry) => void;
}

export function FileTree({ files, workspaceId, onFileClick }: FileTreeProps) {
  return (
    <div className="flex flex-col gap-0.5">
      {files.map((file) => (
        <FileTreeItem key={file.path} entry={file} level={0} workspaceId={workspaceId} onFileClick={onFileClick} />
      ))}
    </div>
  );
}

function FileTreeItem({
  entry,
  level,
  workspaceId,
  onFileClick
}: {
  entry: FileEntry;
  level: number;
  workspaceId: string;
  onFileClick?: (file: FileEntry) => void
}) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [children, setChildren] = useState<FileEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [hasLoaded, setHasLoaded] = useState(false);

  const handleClick = async () => {
    if (entry.isDirectory) {
      if (!isExpanded && !hasLoaded) {
        setIsLoading(true);
        try {
          const files = await listFilesAction(workspaceId, entry.path);
          // Sort: Folders first, then files
          const sorted = files.sort((a, b) => {
            if (a.isDirectory === b.isDirectory) return a.name.localeCompare(b.name);
            return a.isDirectory ? -1 : 1;
          });
          setChildren(sorted);
          setHasLoaded(true);
        } catch (error) {
          console.error('Failed to load directory:', error);
        } finally {
          setIsLoading(false);
        }
      }
      setIsExpanded(!isExpanded);
    } else {
      onFileClick?.(entry);
    }
  };

  return (
    <div>
      <div 
        className={cn(
          "flex items-center gap-1.5 py-1 px-2 rounded-lg cursor-pointer transition-colors text-sm select-none",
          "hover:bg-surface-container-high text-outline hover:text-foreground"
        )}
        style={{ paddingLeft: `${level * 12 + 8}px` }}
        onClick={handleClick}
      >
        <span className="shrink-0 text-outline-variant">
          {entry.isDirectory ? (
            isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />
          ) : (
            <div className="w-[14px]" /> 
          )}
        </span>
        
        <span className={cn("shrink-0", entry.isDirectory ? "text-primary" : "text-outline")}>
          {entry.isDirectory ? <Folder size={16} /> : <File size={16} />}
        </span>
        
        <span className="truncate">{entry.name}</span>
      </div>

      {isExpanded && (
        <div className="flex flex-col gap-0.5">
          {isLoading ? (
            <div className="py-1 px-2 text-xs text-outline pl-8">Loading...</div>
          ) : (
            children.map((child) => (
              <FileTreeItem
                key={child.path}
                entry={child}
                level={level + 1}
                workspaceId={workspaceId}
                onFileClick={onFileClick}
              />
            ))
          )}
        </div>
      )}
    </div>
  );
}