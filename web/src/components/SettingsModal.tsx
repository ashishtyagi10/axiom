'use client';

import React, { useState, useEffect } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Save, Key, Cpu, CheckCircle2, AlertCircle, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { getLLMSettingsAction, updateProviderAction, updateAgentMappingAction } from '@/app/actions/llm';
import { LLMProvider, AgentMapping } from '@/lib/workspace-config';

interface SettingsModalProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SettingsModal({ isOpen, onOpenChange }: SettingsModalProps) {
  const [activeTab, setActiveTab] = useState<'providers' | 'agents'>('providers');
  const [settings, setSettings] = useState<{ providers: LLMProvider[], agentMappings: AgentMapping[] } | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (isOpen) {
      loadSettings();
    }
  }, [isOpen]);

  const loadSettings = async () => {
    setIsLoading(true);
    const data = await getLLMSettingsAction();
    setSettings(data);
    setIsLoading(false);
  };

  const handleUpdateProvider = async (providerId: string, updates: Partial<LLMProvider>) => {
    setIsSaving(true);
    await updateProviderAction(providerId, updates);
    await loadSettings();
    setIsSaving(false);
  };

  const handleUpdateMapping = async (agentId: string, providerId: string, modelId: string) => {
    setIsSaving(true);
    await updateAgentMappingAction(agentId, providerId, modelId);
    await loadSettings();
    setIsSaving(false);
  };

  return (
    <Dialog.Root open={isOpen} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/40 backdrop-blur-sm z-50 animate-in fade-in duration-200" />
        <Dialog.Content className="fixed left-[50%] top-[50%] translate-x-[-50%] translate-y-[-50%] w-full max-w-3xl bg-surface rounded-2xl shadow-2xl z-50 overflow-hidden animate-in zoom-in-95 duration-200 flex flex-col max-h-[90vh]">
          <div className="flex items-center justify-between p-4 border-b border-outline-variant shrink-0">
            <Dialog.Title className="text-xl font-normal">Settings</Dialog.Title>
            <Dialog.Close className="p-2 hover:bg-surface-container rounded-full transition-colors">
              <X size={20} />
            </Dialog.Close>
          </div>

          <div className="flex flex-1 overflow-hidden">
            {/* Sidebar Tabs */}
            <div className="w-48 bg-surface-container-low border-r border-outline-variant p-2 flex flex-col gap-1">
              <button 
                onClick={() => setActiveTab('providers')}
                className={cn(
                  "flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors",
                  activeTab === 'providers' ? "bg-primary-container text-on-primary-container" : "text-outline hover:bg-surface-container"
                )}
              >
                <Key size={18} />
                Providers
              </button>
              <button 
                onClick={() => setActiveTab('agents')}
                className={cn(
                  "flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors",
                  activeTab === 'agents' ? "bg-primary-container text-on-primary-container" : "text-outline hover:bg-surface-container"
                )}
              >
                <Cpu size={18} />
                Agent Models
              </button>
            </div>

            {/* Content Area */}
            <div className="flex-1 overflow-y-auto p-6 bg-surface">
              {isLoading ? (
                <div className="flex items-center justify-center h-40">
                  <Loader2 className="animate-spin text-outline" />
                </div>
              ) : activeTab === 'providers' ? (
                <div className="space-y-6">
                  {settings?.providers.map(provider => (
                    <ProviderConfigRow 
                      key={provider.id} 
                      provider={provider} 
                      onUpdate={(updates) => handleUpdateProvider(provider.id, updates)} 
                    />
                  ))}
                </div>
              ) : (
                <div className="space-y-6">
                  <div className="bg-surface-container-low p-4 rounded-xl border border-outline-variant mb-6">
                    <p className="text-xs text-outline leading-relaxed">
                      Assign specific AI brains to different roles. For example, use GPT-4o for orchestration and Claude 3.5 Sonnet for development tasks.
                    </p>
                  </div>
                  {settings?.agentMappings.map(mapping => (
                    <AgentMappingRow 
                      key={mapping.agentId} 
                      mapping={mapping} 
                      providers={settings.providers.filter(p => p.enabled)}
                      onUpdate={(pId, mId) => handleUpdateMapping(mapping.agentId, pId, mId)} 
                    />
                  ))}
                </div>
              )}
            </div>
          </div>

          <div className="p-4 bg-surface-container-low border-t border-outline-variant flex justify-between items-center shrink-0">
             <div className="text-xs text-outline">
                {isSaving && <span className="flex items-center gap-1"><Loader2 size={12} className="animate-spin"/> Saving changes...</span>}
             </div>
             <Dialog.Close className="px-6 py-2 bg-primary text-white rounded-full text-sm font-medium hover:shadow-md transition-all">
               Done
             </Dialog.Close>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function ProviderConfigRow({ provider, onUpdate }: { provider: LLMProvider, onUpdate: (updates: Partial<LLMProvider>) => void }) {
  const [apiKey, setApiKey] = useState(provider.apiKey);
  const [isEnabled, setIsEnabled] = useState(provider.enabled);

  return (
    <div className={cn(
      "p-4 rounded-2xl border transition-all",
      isEnabled ? "bg-surface border-primary/20 shadow-sm" : "bg-surface-container-low border-outline-variant/50 opacity-70"
    )}>
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <div className={cn("w-10 h-10 rounded-xl flex items-center justify-center", isEnabled ? "bg-primary text-white" : "bg-outline-variant text-outline")}>
            <Key size={20} />
          </div>
          <div>
            <h4 className="font-medium text-foreground">{provider.name}</h4>
            <span className="text-xs text-outline">{provider.defaultModel}</span>
          </div>
        </div>
        <button 
          onClick={() => {
            const next = !isEnabled;
            setIsEnabled(next);
            onUpdate({ enabled: next });
          }}
          className={cn(
            "px-4 py-1.5 rounded-full text-xs font-bold uppercase tracking-wider transition-all",
            isEnabled ? "bg-primary/10 text-primary" : "bg-outline-variant/20 text-outline"
          )}
        >
          {isEnabled ? 'Enabled' : 'Disabled'}
        </button>
      </div>

      <div className="space-y-3">
        <div>
          <label className="text-[10px] font-bold text-outline uppercase ml-1">API Key</label>
          <div className="flex gap-2 mt-1">
            <input 
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={`Enter ${provider.name} API Key`}
              className="flex-1 bg-surface-container px-3 py-2 rounded-lg text-sm border border-transparent focus:border-primary outline-none transition-all"
            />
            <button 
              onClick={() => onUpdate({ apiKey })}
              className="p-2 bg-surface-container hover:bg-surface-container-high rounded-lg text-primary transition-colors"
              title="Save Key"
            >
              <Save size={18} />
            </button>
          </div>
        </div>
        
        {provider.id === 'ollama' && (
           <div>
           <label className="text-[10px] font-bold text-outline uppercase ml-1">Base URL</label>
           <input 
             type="text"
             defaultValue={provider.baseUrl}
             onBlur={(e) => onUpdate({ baseUrl: e.target.value })}
             className="w-full mt-1 bg-surface-container px-3 py-2 rounded-lg text-sm border border-transparent focus:border-primary outline-none transition-all"
           />
         </div>
        )}
      </div>
    </div>
  );
}

function AgentMappingRow({ mapping, providers, onUpdate }: { mapping: AgentMapping, providers: LLMProvider[], onUpdate: (pId: string, mId: string) => void }) {
  const agentLabels: Record<string, string> = {
    orchestrator: 'Orchestrator',
    po: 'Product Owner',
    architect: 'Architect',
    developer: 'Developer'
  };

  return (
    <div className="flex items-center justify-between p-4 bg-surface-container-low rounded-2xl border border-outline-variant/30">
      <div className="flex items-center gap-3">
        <div className="w-8 h-8 rounded-full bg-surface-container-highest flex items-center justify-center text-outline">
          <Cpu size={16} />
        </div>
        <span className="text-sm font-medium text-foreground">{agentLabels[mapping.agentId]}</span>
      </div>

      <div className="flex gap-2">
        <select 
          value={mapping.providerId}
          onChange={(e) => onUpdate(e.target.value, mapping.modelId)}
          className="bg-surface border border-outline-variant rounded-lg px-2 py-1.5 text-xs outline-none focus:border-primary"
        >
          {providers.length === 0 && <option value="">No providers enabled</option>}
          {providers.map(p => (
            <option key={p.id} value={p.id}>{p.name}</option>
          ))}
        </select>
        
        <input 
          type="text"
          defaultValue={mapping.modelId}
          onBlur={(e) => onUpdate(mapping.providerId, e.target.value)}
          className="bg-surface border border-outline-variant rounded-lg px-3 py-1.5 text-xs w-32 outline-none focus:border-primary"
          placeholder="Model ID"
        />
      </div>
    </div>
  );
}
