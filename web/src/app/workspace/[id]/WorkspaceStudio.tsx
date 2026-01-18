'use client';

import React, { useState, useEffect } from 'react';
import {
  Plus,
  ArrowLeft,
  MoreVertical,
  Share2,
  MessageSquare,
  Send,
  User,
  PanelLeft,
  PanelRight,
  Bot,
  Search,
  Settings,
  FolderOpen,
  Code,
  X,
  Loader2,
  Terminal,
  Menu
} from 'lucide-react';
import { cn } from '@/lib/utils';
import LinkNext from 'next/link';
import { ThemeToggle } from '@/components/ThemeToggle';
import { useParams, useRouter } from 'next/navigation';
import { getWorkspaceByIdAction, listFilesAction, readFileAction } from '@/app/actions/workspace';
import { orchestrateAction, developerAction } from '@/app/actions/agent';
import { slashCommandAction } from '@/app/actions/slash';
import type { SlashCommandResult, SlashCommandData, UiAction, CommandHelp } from '@/lib/api/types';
import { runCommandAction } from '@/app/actions/terminal';
import { WorkspaceConfig } from '@/lib/types';
import { FileEntry } from '@/lib/api/types';
import { FileTree } from '@/components/FileTree';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { AgentState, AgentRole } from '@/lib/agents/types';
import { LLMMessage } from '@/lib/llm/api';

export default function WorkspaceStudio() {
  const params = useParams();
  const router = useRouter();
  const workspaceId = params.id as string;

  const [workspace, setWorkspace] = useState<WorkspaceConfig | null>(null);
  const [showSettingsModal, setShowSettingsModal] = useState(false);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [isLoadingFiles, setIsLoadingFiles] = useState(false);
  
  const [messages, setMessages] = useState<LLMMessage[]>([]);
  const [inputValue, setInputValue] = useState('');
  
  // Responsive States
  const [showLeftPanel, setShowLeftPanel] = useState(true);
  const [showRightPanel, setShowRightPanel] = useState(true);
  const [isMobileExplorerOpen, setIsMobileExplorerOpen] = useState(false);
  const [isMobileTeamOpen, setIsMobileTeamOpen] = useState(false);

  // Right Panel State
  const [rightPanelMode, setRightPanelMode] = useState<'team' | 'terminal'>('team');
  const [terminalOutput, setTerminalOutput] = useState<string>('');
  const [isTerminalRunning, setIsTerminalRunning] = useState(false);

  // Agent State
  const [agentStates, setAgentStates] = useState<Record<AgentRole, AgentState>>({
    orchestrator: { id: 'orchestrator', name: 'Orchestrator', status: 'idle' },
    po: { id: 'po', name: 'Product Owner', status: 'idle' },
    architect: { id: 'architect', name: 'Architect', status: 'idle' },
    developer: { id: 'developer', name: 'Developer', status: 'idle' }
  });

  // File Viewing State
  const [activeFile, setActiveFile] = useState<FileEntry | null>(null);
  const [fileContent, setFileContent] = useState<string>('');
  const [isLoadingContent, setIsLoadingContent] = useState(false);

  // Initial Data Fetch
  useEffect(() => {
    async function loadWorkspaceData() {
      if (!workspaceId) return;
      
      try {
        const ws = await getWorkspaceByIdAction(workspaceId);
        if (ws) {
          setWorkspace(ws);
          setIsLoadingFiles(true);
          const fsEntries = await listFilesAction(ws.id);
          setFiles(fsEntries.sort((a, b) => {
            if (a.isDirectory === b.isDirectory) return a.name.localeCompare(b.name);
            return a.isDirectory ? -1 : 1;
          }));
          setIsLoadingFiles(false);
        }
      } catch (error) {
        console.error('Failed to load workspace data:', error);
      }
    }

    loadWorkspaceData();
  }, [workspaceId]);

  const updateAgentStatus = (role: AgentRole, status: AgentState['status'], message?: string) => {
    setAgentStates(prev => ({
      ...prev,
      [role]: { ...prev[role], status, lastMessage: message }
    }));
  };

  // Handle slash commands (e.g., /help, /clear, /init)
  const handleSlashCommand = async (command: string) => {
    console.log('[SlashCommand] Handler called with:', command);
    setInputValue('');

    // Show the command in chat
    setMessages(prev => [...prev, { role: 'user', content: command }]);

    try {
      const result = await slashCommandAction(workspaceId, command);
      console.log('[SlashCommand] Result from backend:', result);
      handleSlashCommandResult(result);
    } catch (error) {
      console.error('[SlashCommand] Error:', error);
      setMessages(prev => [...prev, {
        role: 'assistant',
        content: `Error: ${error instanceof Error ? error.message : 'Failed to execute command'}`
      }]);
    }
  };

  // Handle different slash command results
  const handleSlashCommandResult = (result: SlashCommandResult) => {
    console.log('[SlashCommand] handleSlashCommandResult:', result);

    switch (result.type) {
      case 'Exit':
        console.log('[SlashCommand] Exit handler triggered');
        // Show feedback before navigating
        setMessages(prev => [...prev, { role: 'assistant', content: 'Closing workspace...' }]);
        // Small delay for visual feedback, then navigate
        setTimeout(() => {
          router.push('/');
        }, 500);
        break;

      case 'Success':
        if (result.message) {
          const msg = result.message;
          setMessages(prev => [...prev, { role: 'assistant', content: msg }]);
        }
        // Refresh file list in case /init created a file
        if (workspace) {
          listFilesAction(workspace.id).then(entries => {
            setFiles(entries.sort((a, b) => {
              if (a.isDirectory === b.isDirectory) return a.name.localeCompare(b.name);
              return a.isDirectory ? -1 : 1;
            }));
          });
        }
        break;

      case 'UiAction':
        handleUiAction(result.action);
        break;

      case 'Data':
        displayCommandData(result.data);
        break;

      case 'Error':
        setMessages(prev => [...prev, { role: 'assistant', content: `Error: ${result.message}` }]);
        break;
    }
  };

  // Handle UI actions from slash commands
  const handleUiAction = (action: UiAction) => {
    switch (action.action) {
      case 'OpenSettings':
        setShowSettingsModal(true);
        break;

      case 'ClearOutput':
        setMessages([]);
        break;

      case 'ToggleTheme':
        // Theme toggle is handled by ThemeToggle component
        // For now, just show a message
        setMessages(prev => [...prev, { role: 'assistant', content: 'Use the theme toggle in the header to switch themes.' }]);
        break;

      case 'SetTheme':
        setMessages(prev => [...prev, { role: 'assistant', content: `Theme set to: ${action.variant}` }]);
        break;

      case 'OpenWorkspaceSelector':
        router.push('/');
        break;

      case 'OpenModelSelector':
        setShowSettingsModal(true);
        break;

      case 'FocusPanel':
        // Panels are always visible, just acknowledge
        setMessages(prev => [...prev, { role: 'assistant', content: `Focus: ${action.panel}` }]);
        break;
    }
  };

  // Display data from slash commands
  const displayCommandData = (data: SlashCommandData) => {
    switch (data.data_type) {
      case 'Help':
        const helpContent = formatHelpContent(data.value.commands);
        setMessages(prev => [...prev, { role: 'assistant', content: helpContent }]);
        break;

      case 'Version':
        const versionInfo = `**Axiom** v${data.value.version}${data.value.commit ? ` (${data.value.commit})` : ''}`;
        setMessages(prev => [...prev, { role: 'assistant', content: versionInfo }]);
        break;

      case 'WorkspaceList':
        const wsContent = formatWorkspaceList(data.value);
        setMessages(prev => [...prev, { role: 'assistant', content: wsContent }]);
        break;

      case 'ModelList':
        const modelContent = `**Models** (${data.value.provider}):\n${data.value.models.map(m =>
          m === data.value.active ? `- **${m}** (active)` : `- ${m}`
        ).join('\n')}`;
        setMessages(prev => [...prev, { role: 'assistant', content: modelContent }]);
        break;

      case 'Text':
        setMessages(prev => [...prev, { role: 'assistant', content: data.value }]);
        break;
    }
  };

  // Format help content for display
  const formatHelpContent = (commands: CommandHelp[]): string => {
    let content = '**Available Commands**\n\n';
    for (const cmd of commands) {
      content += `**/${cmd.name}**`;
      if (cmd.aliases.length > 0) {
        content += ` (aliases: ${cmd.aliases.map(a => `/${a}`).join(', ')})`;
      }
      content += `\n${cmd.description}\n`;
      content += `Usage: \`${cmd.usage}\`\n\n`;
    }
    return content;
  };

  // Format workspace list for display
  const formatWorkspaceList = (workspaces: Array<{ id: string; name: string; path: string; is_active: boolean }>): string => {
    let content = '**Workspaces**\n\n';
    for (const ws of workspaces) {
      content += ws.is_active ? `- **${ws.name}** (active)\n` : `- ${ws.name}\n`;
      content += `  Path: ${ws.path}\n`;
      content += `  ID: ${ws.id}\n\n`;
    }
    return content;
  };

  const handleSendMessage = async () => {
    if (!inputValue.trim()) return;

    console.log('[handleSendMessage] Input:', JSON.stringify(inputValue));
    console.log('[handleSendMessage] Starts with /:', inputValue.trim().startsWith('/'));

    // Check for slash command
    if (inputValue.trim().startsWith('/')) {
      console.log('[SlashCommand] Detected:', inputValue.trim());
      await handleSlashCommand(inputValue.trim());
      return;
    }

    const userMsg: LLMMessage = { role: 'user', content: inputValue };
    if (activeFile) setActiveFile(null);
    setMessages(prev => [...prev, userMsg]);
    setInputValue('');

    try {
      updateAgentStatus('orchestrator', 'thinking', 'Analyzing request...');
      const decision = await orchestrateAction(workspaceId, [...messages, userMsg]);
      updateAgentStatus('orchestrator', 'idle', decision.reasoning);

      if (decision.nextAgent === 'user') {
        setMessages(prev => [...prev, { role: 'assistant', content: decision.task || "I'm not sure how to help." }]);
      } else {
        const agentRole = decision.nextAgent as AgentRole;
        const agentName = agentStates[agentRole].name;
        
        setMessages(prev => [...prev, {
          role: 'assistant', 
          content: `**Orchestrator**: Delegating to **${agentName}**.\n\n*Reasoning*: ${decision.reasoning}\n\n*Task*: ${decision.task}` 
        }]);

        updateAgentStatus(agentRole, 'working', decision.task);

        if (agentRole === 'developer') {
           if (!workspace?.path) throw new Error("Workspace path not found");
           const devResult = await developerAction(workspaceId, decision.task || '');
           const executeOps = devResult.operations.filter(op => op.type === 'execute');
           
           if (executeOps.length > 0) {
             setRightPanelMode('terminal');
             setIsTerminalRunning(true);
             for (const op of executeOps) {
               if (op.command) {
                 setTerminalOutput(prev => prev + `> ${op.command}\n`);
                 const result = await runCommandAction(op.command, workspace.path);
                 setTerminalOutput(prev => prev + result.stdout + result.stderr + '\n');
               }
             }
             setIsTerminalRunning(false);
             setTimeout(() => setRightPanelMode('team'), 3000);
           }

           const fsEntries = await listFilesAction(workspace.id);
           setFiles(fsEntries.sort((a, b) => {
             if (a.isDirectory === b.isDirectory) return a.name.localeCompare(b.name);
             return a.isDirectory ? -1 : 1;
           }));

           updateAgentStatus(agentRole, 'idle', 'Task complete');
           setMessages(prev => [...prev, { role: 'assistant', content: `**${agentName}**: ${devResult.message}` }]);
        } else {
          setTimeout(() => {
             updateAgentStatus(agentRole, 'idle', 'Task complete');
             setMessages(prev => [...prev, { role: 'assistant', content: `**${agentName}**: Task noted.` }]);
          }, 2000);
        }
      }
    } catch (error) {
      updateAgentStatus('orchestrator', 'idle', 'Error occurred');
      setMessages(prev => [...prev, { role: 'assistant', content: "Error while thinking. Check settings." }]);
    }
  };

  const handleFileClick = async (file: FileEntry) => {
    setActiveFile(file);
    setIsLoadingContent(true);
    setIsMobileExplorerOpen(false); // Close mobile explorer on select
    try {
      const content = await readFileAction(workspaceId, file.path);
      setFileContent(content);
    } catch (error) {
      setFileContent('Error reading file.');
    } finally {
      setIsLoadingContent(false);
    }
  };

  const closeFileView = () => {
    setActiveFile(null);
    setFileContent('');
  };

  const getLanguage = (filename: string) => {
    const ext = filename.split('.').pop()?.toLowerCase();
    switch (ext) {
      case 'ts': case 'tsx': return 'typescript';
      case 'js': case 'jsx': return 'javascript';
      case 'css': return 'css';
      case 'json': return 'json';
      case 'html': return 'html';
      case 'md': return 'markdown';
      case 'py': return 'python';
      default: return 'text';
    }
  };

  return (
    <div className="flex flex-col h-screen bg-surface-container">
      {/* Top Header */}
      <header className="h-16 flex items-center justify-between px-4 bg-surface rounded-2xl mx-2 mt-2 shrink-0 z-20 shadow-sm">
        <div className="flex items-center gap-3">
          <LinkNext href="/" className="p-2 hover:bg-surface-container rounded-full transition-colors">
            <ArrowLeft size={20} />
          </LinkNext>
          
          <div className="flex items-center gap-2 ml-1">
            <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center text-white">
              <MessageSquare size={18} />
            </div>
            <h1 className="text-lg font-normal hidden sm:block truncate max-w-[200px]">
              {workspace ? workspace.title : 'Loading...'}
            </h1>
          </div>
        </div>

        <div className="flex items-center gap-1 sm:gap-2">
          <button 
            onClick={() => setIsMobileExplorerOpen(true)}
            className="lg:hidden p-2 hover:bg-surface-container rounded-full text-outline"
            title="Explorer"
          >
            <FolderOpen size={20} />
          </button>
          
          <ThemeToggle className="hidden sm:flex" />

          <button className="hidden md:flex items-center gap-2 px-4 py-2 border border-outline rounded-full text-sm font-medium hover:bg-surface-container-low transition-colors">
            <Share2 size={16} />
            Share
          </button>
          
          <button 
            onClick={() => setIsMobileTeamOpen(true)}
            className="lg:hidden p-2 hover:bg-surface-container rounded-full text-primary"
            title="Team"
          >
            <Bot size={20} />
          </button>

          <button className="p-2 hover:bg-surface-container rounded-full">
            <MoreVertical size={20} />
          </button>
        </div>
      </header>

      {/* Main Content Area */}
      <div className="flex-1 flex overflow-hidden px-2 pt-2 pb-2 gap-2 relative">
        
        {/* Mobile Explorer Overlay */}
        {isMobileExplorerOpen && (
          <div className="lg:hidden fixed inset-0 z-[60] bg-black/40 backdrop-blur-sm" onClick={() => setIsMobileExplorerOpen(false)} />
        )}
        
        {/* Desktop Left Panel (Docked) */}
        <section 
          className={cn(
            "hidden lg:flex flex-col bg-surface rounded-2xl overflow-hidden transition-all duration-300 ease-in-out z-[70] shrink-0",
            showLeftPanel ? "w-80" : "w-16"
          )}
        >
          <LeftPanelContent
            collapsed={!showLeftPanel}
            onToggle={() => setShowLeftPanel(!showLeftPanel)}
            isLoading={isLoadingFiles}
            files={files}
            workspaceId={workspace?.id || ''}
            onFileClick={handleFileClick}
          />
        </section>

        {/* Mobile Left Panel (Overlay) */}
        <section 
          className={cn(
            "lg:hidden fixed inset-y-2 left-2 flex-col bg-surface rounded-2xl overflow-hidden transition-transform duration-300 ease-in-out z-[70] w-80 shadow-2xl",
            isMobileExplorerOpen ? "translate-x-0" : "-translate-x-[110%]"
          )}
        >
           <LeftPanelContent
            collapsed={false}
            onToggle={() => setIsMobileExplorerOpen(false)}
            isLoading={isLoadingFiles}
            files={files}
            workspaceId={workspace?.id || ''}
            onFileClick={handleFileClick}
            isMobile
          />
        </section>

        {/* Middle Panel: Chat & Code View */}
        <section className="flex-1 flex flex-col bg-surface rounded-2xl overflow-hidden shadow-sm z-10 relative">
          <div className="flex-1 overflow-hidden relative">
            {activeFile ? (
              <div className="flex flex-col h-full absolute inset-0 bg-surface z-10">
                 <div className="flex items-center justify-between p-3 border-b border-outline-variant bg-surface-container-low">
                    <div className="flex items-center gap-2 px-2 truncate">
                       <Code size={18} className="text-primary shrink-0" />
                       <span className="text-sm font-medium text-foreground truncate">{activeFile.name}</span>
                    </div>
                    <button onClick={closeFileView} className="p-1.5 hover:bg-surface-container rounded-full text-outline">
                      <X size={18} />
                    </button>
                 </div>
                 <div className="flex-1 overflow-auto bg-[#1e1e1e]">
                    {isLoadingContent ? <div className="p-10 text-outline text-center text-sm">Loading...</div> : (
                      <SyntaxHighlighter language={getLanguage(activeFile.name)} style={vscDarkPlus} showLineNumbers={true} customStyle={{ margin: 0, padding: '1rem', fontSize: '0.8rem' }}>
                        {fileContent}
                      </SyntaxHighlighter>
                    )}
                 </div>
              </div>
            ) : (
              <div className="h-full overflow-y-auto p-4 sm:p-6 space-y-6 text-foreground">
                {messages.length === 0 ? (
                  <div className="max-w-2xl mx-auto mt-10 sm:mt-20 text-center px-4">
                    <h2 className="text-3xl sm:text-4xl font-normal mb-8 tracking-tight">How can I help you build?</h2>
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-left">
                      <SuggestionChip text="Scaffold a new component" onClick={() => setInputValue("Scaffold a new component")} />
                      <SuggestionChip text="Explain architecture" onClick={() => setInputValue("Explain architecture")} />
                    </div>
                  </div>
                ) : (
                  <div className="max-w-3xl mx-auto w-full space-y-8 pt-4 pb-4">
                    {messages.map((msg, i) => (
                      <div key={i} className={cn("flex gap-3 sm:gap-4", msg.role === 'user' ? "justify-end" : "justify-start")}>
                        {msg.role === 'assistant' && (
                          <div className="w-8 h-8 rounded-full bg-primary shrink-0 flex items-center justify-center text-white"><MessageSquare size={16} /></div>
                        )}
                        <div className={cn("max-w-[85%] sm:max-w-[80%] p-3 sm:p-4 rounded-2xl text-sm leading-relaxed", msg.role === 'user' ? "bg-surface-container-highest" : "bg-transparent")}>
                          <div className="whitespace-pre-wrap">{msg.content}</div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>

          <div className="p-3 sm:p-4 bg-surface border-t border-outline-variant z-20">
            <div className="max-w-3xl mx-auto">
              <div className="flex flex-col bg-surface-container-high rounded-3xl p-2 shadow-sm focus-within:bg-surface-container-highest transition-all">
                <div className="flex items-end gap-2 px-2">
                  <textarea
                    placeholder="Ask the team..."
                    className="flex-1 bg-transparent border-none focus:ring-0 outline-none resize-none py-3 px-1 text-base placeholder:text-outline max-h-40 min-h-[48px]"
                    rows={1}
                    value={inputValue}
                    onChange={(e) => setInputValue(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' && !e.shiftKey) {
                        e.preventDefault();
                        handleSendMessage();
                      }
                    }}
                  />
                  <button onClick={handleSendMessage} className="mb-1 p-2 bg-primary text-white rounded-full hover:shadow-lg disabled:opacity-30 transition-all">
                    <Send size={20} />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Mobile Team Overlay */}
        {isMobileTeamOpen && (
          <div className="lg:hidden fixed inset-0 z-[60] bg-black/40 backdrop-blur-sm" onClick={() => setIsMobileTeamOpen(false)} />
        )}

        {/* Desktop Right Panel (Docked) */}
        <section 
          className={cn(
            "hidden lg:flex flex-col bg-surface rounded-2xl overflow-hidden transition-all duration-300 ease-in-out z-[70] shrink-0",
            showRightPanel ? "w-96" : "w-16"
          )}
        >
          <RightPanelContent 
            collapsed={!showRightPanel}
            onToggle={() => setShowRightPanel(!showRightPanel)}
            mode={rightPanelMode}
            agentStates={agentStates}
            terminalOutput={terminalOutput}
          />
        </section>

        {/* Mobile Right Panel (Overlay) */}
        <section 
          className={cn(
            "lg:hidden fixed inset-y-2 right-2 flex-col bg-surface rounded-2xl overflow-hidden transition-transform duration-300 ease-in-out z-[70] w-80 shadow-2xl",
            isMobileTeamOpen ? "translate-x-0" : "translate-x-[110%]"
          )}
        >
          <RightPanelContent 
            collapsed={false}
            onToggle={() => setIsMobileTeamOpen(false)}
            mode={rightPanelMode}
            agentStates={agentStates}
            terminalOutput={terminalOutput}
            isMobile
          />
        </section>

      </div>
    </div>
  );
}

// --- Helper Components ---

function LeftPanelContent({ collapsed, onToggle, isLoading, files, workspaceId, onFileClick, isMobile }: any) {
  if (collapsed) {
    return (
      <div className="flex flex-col items-center py-4 gap-4 h-full bg-surface-container/50">
        <button onClick={onToggle} className="p-2 rounded-full hover:bg-surface-container text-outline transition-colors" title="Expand Explorer">
          <PanelLeft size={20} />
        </button>
        <div className="w-8 h-1 bg-outline-variant rounded-full" />
        <button className="p-2 rounded-full hover:bg-surface-container text-outline"><Search size={20} /></button>
        <button className="p-2 rounded-full hover:bg-surface-container text-outline"><FolderOpen size={20} /></button>
      </div>
    );
  }

  return (
    <>
      <div className="p-4 flex items-center justify-between border-b border-outline-variant min-w-[320px]">
        <h2 className="font-medium text-foreground">Explorer</h2>
        <div className="flex items-center gap-1">
          <button onClick={onToggle} className="p-2 hover:bg-surface-container rounded-full text-outline">
            {isMobile ? <X size={20} /> : <PanelLeft size={20} />}
          </button>
        </div>
      </div>
      <div className="flex-1 overflow-y-auto p-2 min-w-[320px]">
        {isLoading ? (
          <div className="flex items-center justify-center h-20 text-sm text-outline">Loading...</div>
        ) : (
          <FileTree files={files} workspaceId={workspaceId} onFileClick={onFileClick} />
        )}
      </div>
    </>
  );
}

function RightPanelContent({ collapsed, onToggle, mode, agentStates, terminalOutput, isMobile }: any) {
  if (collapsed) {
    return (
      <div className="flex flex-col items-center py-4 gap-4 h-full bg-surface-container/50">
        <button onClick={onToggle} className="p-2 rounded-full hover:bg-surface-container text-outline transition-colors" title="Expand Team">
          <PanelRight size={20} />
        </button>
        <div className="w-8 h-1 bg-outline-variant rounded-full" />
        <button className="p-2 rounded-full hover:bg-surface-container text-primary transition-colors" title="Team">
          <Bot size={20} />
        </button>
      </div>
    );
  }

  return (
    <>
      <div className="flex items-center justify-between p-4 border-b border-outline-variant min-w-[320px]">
        <div className="flex items-center gap-2">
          {mode === 'team' ? <Bot size={20} className="text-primary" /> : <Terminal size={20} className="text-secondary" />}
          <h2 className="font-medium text-foreground">{mode === 'team' ? 'Team' : 'Terminal'}</h2>
        </div>
        <button onClick={onToggle} className="p-2 hover:bg-surface-container rounded-full text-outline transition-colors">
          {isMobile ? <X size={20} /> : <PanelRight size={20} />}
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-4 min-w-[320px]">
        {mode === 'team' ? (
          <div className="space-y-6">
            <div className="p-5 bg-surface-container-high rounded-2xl border border-outline-variant">
              <h3 className="font-medium text-foreground mb-4">Orchestrator</h3>
              <p className="text-sm text-outline leading-relaxed">{agentStates.orchestrator.lastMessage || "Ready."}</p>
            </div>
            <div className="grid grid-cols-1 gap-3">
              <AgentCard state={agentStates.po} icon={<User size={16} />} />
              <AgentCard state={agentStates.architect} icon={<Settings size={16} />} />
              <AgentCard state={agentStates.developer} icon={<Bot size={16} />} />
            </div>
          </div>
        ) : (
          <div className="h-full bg-[#1e1e1e] rounded-xl p-4 font-mono text-[10px] text-white overflow-auto whitespace-pre-wrap">
            {terminalOutput}
          </div>
        )}
      </div>
    </>
  );
}

function SuggestionChip({ text, onClick }: { text: string, onClick?: () => void }) {
  return (
    <button onClick={onClick} className="p-4 bg-surface-container-low border border-outline-variant rounded-2xl text-sm text-foreground hover:bg-surface-container transition-all text-left">
      {text}
    </button>
  );
}

function AgentCard({ state, icon }: { state: AgentState, icon: React.ReactNode }) {
  return (
    <div className={cn("p-3 bg-surface-container-low rounded-xl border transition-all flex flex-col gap-2", state.status === 'working' ? "border-primary/50" : "border-transparent")}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-surface-container-highest flex items-center justify-center text-outline">{icon}</div>
          <span className="text-sm font-medium text-foreground">{state.name}</span>
        </div>
        <span className={cn("text-[10px] px-2 py-0.5 rounded capitalize", state.status === 'working' ? "bg-primary/10 text-primary" : "text-outline bg-surface-container")}>{state.status}</span>
      </div>
    </div>
  );
}
