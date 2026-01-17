import { getLLMSettings } from '../workspace-config';

export interface LLMMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface LLMResponse {
  content: string;
  usage?: {
    promptTokens: number;
    completionTokens: number;
  };
}

export async function chatCompletion(
  agentId: string,
  messages: LLMMessage[]
): Promise<LLMResponse> {
  const settings = await getLLMSettings();
  const mapping = settings.agentMappings.find(m => m.agentId === agentId);
  
  if (!mapping) {
    throw new Error(`No mapping found for agent: ${agentId}`);
  }

  const provider = settings.providers.find(p => p.id === mapping.providerId);
  if (!provider || !provider.enabled || !provider.apiKey) {
    throw new Error(`Provider ${mapping.providerId} is not configured or enabled`);
  }

  const modelId = mapping.modelId || provider.defaultModel;

  switch (provider.id) {
    case 'openai':
      return callOpenAI(provider.apiKey, modelId, messages, provider.baseUrl);
    case 'anthropic':
      return callAnthropic(provider.apiKey, modelId, messages);
    case 'gemini':
      return callGemini(provider.apiKey, modelId, messages);
    case 'ollama':
      return callOllama(provider.baseUrl || 'http://localhost:11434', modelId, messages);
    default:
      throw new Error(`Unsupported provider: ${provider.id}`);
  }
}

async function callOpenAI(apiKey: string, model: string, messages: LLMMessage[], baseUrl?: string): Promise<LLMResponse> {
  const url = baseUrl ? `${baseUrl}/chat/completions` : 'https://api.openai.com/v1/chat/completions';
  
  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiKey}`
    },
    body: JSON.stringify({
      model,
      messages,
      temperature: 0.7
    })
  });

  if (!response.ok) {
    const err = await response.text();
    throw new Error(`OpenAI API error: ${err}`);
  }

  const data = await response.json();
  return {
    content: data.choices[0].message.content,
    usage: {
      promptTokens: data.usage.prompt_tokens,
      completionTokens: data.usage.completion_tokens
    }
  };
}

async function callAnthropic(apiKey: string, model: string, messages: LLMMessage[]): Promise<LLMResponse> {
  const systemMessage = messages.find(m => m.role === 'system')?.content;
  const userMessages = messages.filter(m => m.role !== 'system').map(m => ({
    role: m.role === 'assistant' ? 'assistant' : 'user',
    content: m.content
  }));

  const response = await fetch('https://api.anthropic.com/v1/messages', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-api-key': apiKey,
      'anthropic-version': '2023-06-01'
    },
    body: JSON.stringify({
      model,
      system: systemMessage,
      messages: userMessages,
      max_tokens: 4096
    })
  });

  if (!response.ok) {
    const err = await response.text();
    throw new Error(`Anthropic API error: ${err}`);
  }

  const data = await response.json();
  return {
    content: data.content[0].text,
    usage: {
      promptTokens: data.usage.input_tokens,
      completionTokens: data.usage.output_tokens
    }
  };
}

async function callGemini(apiKey: string, model: string, messages: LLMMessage[]): Promise<LLMResponse> {
  // Simple implementation for Gemini REST API
  const url = `https://generativelanguage.googleapis.com/v1beta/models/${model}:generateContent?key=${apiKey}`;
  
  const contents = messages.map(m => ({
    role: m.role === 'assistant' ? 'model' : 'user',
    parts: [{ text: m.content }]
  }));

  const response = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ contents })
  });

  if (!response.ok) {
    const err = await response.text();
    throw new Error(`Gemini API error: ${err}`);
  }

  const data = await response.json();
  return {
    content: data.candidates[0].content.parts[0].text
  };
}

async function callOllama(baseUrl: string, model: string, messages: LLMMessage[]): Promise<LLMResponse> {
  const response = await fetch(`${baseUrl}/api/chat`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model,
      messages,
      stream: false
    })
  });

  if (!response.ok) {
    const err = await response.text();
    throw new Error(`Ollama error: ${err}`);
  }

  const data = await response.json();
  return {
    content: data.message.content
  };
}
