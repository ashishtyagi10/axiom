'use client';

import React, { useState, useEffect } from 'react';
import { Plus, HelpCircle, Shield, FileText, Home, MessageSquare, Menu, PanelLeft, Settings, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import Link from 'next/link';
import { ThemeToggle } from '@/components/ThemeToggle';
import { getWorkspacesAction, addWorkspaceAction } from '@/app/actions/workspace';
import { WorkspaceConfig } from '@/lib/workspace-config';
import { FilePickerModal } from '@/components/FilePickerModal';
import { SettingsModal } from '@/components/SettingsModal';

export default function Dashboard() {
  const [isSidebarOpen, setIsSidebarOpen] = useState(true);
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  const [workspaces, setWorkspaces] = useState<WorkspaceConfig[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isPickerOpen, setIsPickerOpen] = useState(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);

  const loadWorkspaces = async () => {
    try {
      const data = await getWorkspacesAction();
      setWorkspaces(data);
    } catch (error) {
      console.error('Failed to load workspaces:', error);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadWorkspaces();
  }, []);

  const handleCreateWorkspace = () => {
    setIsPickerOpen(true);
  };

  const handleWorkspaceSelected = async (path: string) => {
    if (!path) return;

    const title = path.split('/').pop() || 'Untitled Workspace';
    
    try {
      await addWorkspaceAction(title, path, 'local');
      await loadWorkspaces();
    } catch (error) {
      console.error('Failed to create workspace:', error);
      alert('Failed to create workspace. Check console for details.');
    }
  };

  return (
    <div className="flex h-screen bg-surface">
      <FilePickerModal 
        isOpen={isPickerOpen} 
        onOpenChange={setIsPickerOpen} 
        onSelect={handleWorkspaceSelected} 
      />

      <SettingsModal 
        isOpen={isSettingsOpen}
        onOpenChange={setIsSettingsOpen}
      />

      {/* Desktop Sidebar */}
      <aside className={cn(
        "hidden lg:flex border-r border-outline-variant flex-col transition-all duration-300 ease-in-out",
        isSidebarOpen ? "w-64 p-4" : "w-16 p-2 items-center"
      )}>
        <SidebarContent 
          isSidebarOpen={isSidebarOpen} 
          setIsSidebarOpen={setIsSidebarOpen}
          setIsSettingsOpen={setIsSettingsOpen}
        />
      </aside>

      {/* Mobile Sidebar Overlay */}
      {isMobileMenuOpen && (
        <div 
          className="lg:hidden fixed inset-0 bg-black/40 z-40 backdrop-blur-sm"
          onClick={() => setIsMobileMenuOpen(false)}
        />
      )}
      
      {/* Mobile Sidebar */}
      <aside className={cn(
        "lg:hidden fixed inset-y-0 left-0 z-50 w-64 bg-surface border-r border-outline-variant flex flex-col p-4 transition-transform duration-300",
        isMobileMenuOpen ? "translate-x-0" : "-translate-x-full"
      )}>
        <div className="flex justify-end mb-4">
          <button onClick={() => setIsMobileMenuOpen(false)} className="p-2 hover:bg-surface-container rounded-full">
            <X size={24} />
          </button>
        </div>
        <SidebarContent 
          isSidebarOpen={true} 
          setIsSidebarOpen={() => {}}
          setIsSettingsOpen={setIsSettingsOpen}
          hideToggle
        />
      </aside>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto p-6 md:p-8 lg:p-12">
        <header className="flex justify-between items-center mb-8">
          <h1 className="text-3xl font-normal text-foreground">Workspaces</h1>
          <button 
            onClick={() => setIsMobileMenuOpen(true)}
            className="lg:hidden p-2 rounded-full hover:bg-surface-container"
          >
            <Menu size={24} />
          </button>
        </header>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          {/* New Workspace Card */}
          <button 
            onClick={handleCreateWorkspace}
            className="group flex flex-col items-start p-6 bg-primary-container rounded-2xl hover:shadow-md transition-all text-left"
          >
            <div className="w-10 h-10 bg-white rounded-xl flex items-center justify-center mb-12 text-primary group-hover:scale-110 transition-transform">
              <Plus size={24} />
            </div>
            <span className="text-lg font-medium text-on-primary-container">New Workspace</span>
          </button>

          {/* Workspace Cards */}
          {isLoading ? (
            <div className="col-span-full text-center text-outline">Loading workspaces...</div>
          ) : workspaces.length === 0 ? (
            <div className="col-span-full text-center text-outline py-10">
              No workspaces yet. Create one to get started.
            </div>
          ) : (
            workspaces.map((ws) => (
              <Link 
                key={ws.id}
                href={`/workspace/${ws.id}`}
                className="flex flex-col p-6 bg-surface-container rounded-2xl border border-transparent hover:border-outline-variant hover:shadow-sm cursor-pointer transition-all"
              >
                <div className="flex-1 mb-12">
                  <h3 className="text-lg font-medium text-foreground line-clamp-2" title={ws.path}>{ws.title}</h3>
                </div>
                <div className="flex justify-between items-end">
                  <span className="text-sm text-outline font-medium uppercase text-[10px] tracking-wider">{ws.type}</span>
                  <span className="text-xs text-outline">{new Date(ws.lastAccessed).toLocaleDateString()}</span>
                </div>
              </Link>
            ))
          )}
        </div>
      </main>
    </div>
  );
}

function SidebarContent({ isSidebarOpen, setIsSidebarOpen, setIsSettingsOpen, hideToggle = false }: any) {
  return (
    <>
      <div className={cn("flex items-center gap-2 mb-8", isSidebarOpen ? "px-2" : "justify-center flex-col gap-4")}>
        {!isSidebarOpen && (
          <button 
            onClick={() => setIsSidebarOpen(true)} 
            className="p-2 hover:bg-surface-container rounded-full text-outline"
          >
            <PanelLeft size={20} />
          </button>
        )}
        
        <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center text-white shrink-0">
          <MessageSquare size={20} />
        </div>
        
        {isSidebarOpen && (
          <>
            <span className="text-xl font-medium tracking-tight flex-1">OSMO</span>
            {!hideToggle && (
              <button 
                onClick={() => setIsSidebarOpen(false)} 
                className="p-2 hover:bg-surface-container rounded-full text-outline"
              >
                <PanelLeft size={20} />
              </button>
            )}
          </>
        )}
      </div>

      <nav className="flex-1 flex flex-col gap-1 w-full">
        <SidebarItem 
          icon={<Home size={20} />} 
          label="Home" 
          active 
          collapsed={!isSidebarOpen} 
        />
      </nav>

      <div className={cn("flex flex-col gap-1 mt-auto pt-4 border-t border-outline-variant w-full", !isSidebarOpen && "items-center")}>
        <SidebarItem icon={<HelpCircle size={20} />} label="Help" collapsed={!isSidebarOpen} />
        
        <button 
          onClick={() => setIsSettingsOpen(true)}
          className={cn(
            "flex items-center gap-3 rounded-full text-sm font-medium transition-colors text-foreground hover:bg-surface-container",
            !isSidebarOpen ? "justify-center p-2.5" : "px-3 py-2.5"
          )}
          title={!isSidebarOpen ? "Settings" : undefined}
        >
          <Settings size={20} />
          {isSidebarOpen && <span>Settings</span>}
        </button>

        <div className={cn("flex items-center", isSidebarOpen ? "justify-between px-3 py-2.5" : "justify-center py-2")}>
           {isSidebarOpen && <span className="text-sm font-medium text-foreground">Theme</span>}
           <ThemeToggle />
        </div>
      </div>
    </>
  );
}

function SidebarItem({ 
  icon, 
  label, 
  active = false, 
  collapsed = false 
}: { 
  icon: React.ReactNode, 
  label: string, 
  active?: boolean, 
  collapsed?: boolean 
}) {
  return (
    <button 
      className={cn(
        "flex items-center gap-3 rounded-full text-sm font-medium transition-colors",
        active 
          ? "bg-primary-container text-on-primary-container" 
          : "text-foreground hover:bg-surface-container",
        collapsed ? "justify-center p-2.5" : "px-3 py-2.5"
      )}
      title={collapsed ? label : undefined}
    >
      {icon}
      {!collapsed && <span>{label}</span>}
    </button>
  );
}
