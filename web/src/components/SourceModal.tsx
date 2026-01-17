'use client';

import React from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, FileUp, Link, Github, FileText, Globe } from 'lucide-react';
import { cn } from '@/lib/utils';

interface SourceModalProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SourceModal({ isOpen, onOpenChange }: SourceModalProps) {
  return (
    <Dialog.Root open={isOpen} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/40 backdrop-blur-sm z-50 animate-in fade-in duration-200" />
        <Dialog.Content className="fixed left-[50%] top-[50%] translate-x-[-50%] translate-y-[-50%] w-full max-w-2xl bg-surface rounded-2xl shadow-2xl z-50 overflow-hidden animate-in zoom-in-95 duration-200">
          <div className="flex items-center justify-between p-4 border-b border-outline-variant">
            <Dialog.Title className="text-xl font-normal">Add sources</Dialog.Title>
            <Dialog.Close className="p-2 hover:bg-surface-container rounded-full transition-colors">
              <X size={20} />
            </Dialog.Close>
          </div>

          <div className="p-6">
            <p className="text-sm text-outline mb-6">
              Sources help OSMO answer your questions. You can upload documents, paste text, or link to websites.
            </p>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <SourceOption 
                icon={<FileUp className="text-primary" size={24} />}
                title="Upload Files"
                description="PDF, Markdown, Text"
              />
              <SourceOption 
                icon={<Link className="text-secondary" size={24} />}
                title="Website"
                description="Paste a URL"
              />
              <SourceOption 
                icon={<FileText className="text-amber-600" size={24} />}
                title="Copied Text"
                description="Paste your notes"
              />
              <SourceOption 
                icon={<Github className="text-foreground" size={24} />}
                title="GitHub Repo"
                description="Import from GitHub"
              />
            </div>

            <div className="mt-8 p-4 bg-surface-container-low rounded-xl border border-dashed border-outline-variant flex flex-col items-center justify-center py-12">
              <FileUp size={32} className="text-outline mb-3" />
              <p className="text-sm font-medium">Drag & drop files here</p>
              <p className="text-xs text-outline mt-1">or click to browse</p>
            </div>
          </div>

          <div className="flex justify-end gap-3 p-4 bg-surface-container-low border-t border-outline-variant">
            <Dialog.Close className="px-6 py-2 rounded-full text-sm font-medium hover:bg-surface-container transition-colors">
              Cancel
            </Dialog.Close>
            <button className="px-6 py-2 bg-primary text-white rounded-full text-sm font-medium hover:shadow-md transition-all">
              Add
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function SourceOption({ icon, title, description }: { icon: React.ReactNode, title: string, description: string }) {
  return (
    <button className="flex items-start gap-4 p-4 rounded-xl border border-outline-variant hover:bg-surface-container-low hover:border-primary/30 transition-all text-left group">
      <div className="p-2 bg-surface-container rounded-lg group-hover:bg-white transition-colors">
        {icon}
      </div>
      <div>
        <h4 className="text-sm font-medium text-foreground">{title}</h4>
        <p className="text-xs text-outline mt-0.5">{description}</p>
      </div>
    </button>
  );
}
