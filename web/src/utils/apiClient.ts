import { isTauriEnvironment } from './tauriBridge';

async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

export interface ToolDefinition {
  id: string;
  version: string;
  display_name: string;
  description: string;
  input_schema: Record<string, unknown>;
  output_schema?: Record<string, unknown>;
  risk_tier: 'READ' | 'WRITE' | 'EXECUTE';
  risk_class: string;
  capabilities: string[];
  requires_approval: boolean;
  timeout_seconds: number;
  max_output_bytes: number;
  idempotency_mode: string;
  source: string;
}

export interface ProcessItem {
  id: string;
  name: string;
  command: string;
  pid: number;
  cpu: number;
  memoryMB: number;
  uptime: string;
  status: 'running' | 'idle' | 'stopped';
}

export const apiClient = {
  isTauri(): boolean {
    return isTauriEnvironment();
  },

  async getCollection<T>(name: string): Promise<T[]> {
    if (this.isTauri()) {
      try {
        const data = await invokeTauri<T[]>('get_collection', { name });
        if (data && Array.isArray(data) && data.length > 0) {
          return data;
        }
      } catch (err) {
        console.warn(`[API] Tauri get_collection("${name}") fallback to HTTP:`, err);
      }
    }
    const res = await fetch(`/api/collections/${name}`);
    if (!res.ok) {
      throw new Error(`Failed to fetch collection ${name}: ${res.statusText}`);
    }
    return res.json();
  },

  async saveCollection<T>(name: string, data: T[]): Promise<void> {
    if (this.isTauri()) {
      try {
        await invokeTauri('save_collection', { name, data });
        return;
      } catch (err) {
        console.warn(`[API] Tauri save_collection("${name}") fallback to HTTP:`, err);
      }
    }
    const res = await fetch(`/api/collections/${name}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data)
    });
    if (!res.ok) {
      throw new Error(`Failed to save collection ${name}: ${res.statusText}`);
    }
  },

  async getFleetMetrics(): Promise<Record<string, unknown>> {
    if (this.isTauri()) {
      try {
        return await invokeTauri('get_fleet_metrics');
      } catch (e) {
        console.warn('[API] Tauri get_fleet_metrics fallback:', e);
      }
    }
    const res = await fetch('/api/fleet/metrics');
    return res.json();
  },

  async ringDeckBell(): Promise<string> {
    if (this.isTauri()) {
      try {
        return await invokeTauri<string>('ring_deck_bell');
      } catch (e) {
        console.warn('[API] Tauri ring_deck_bell fallback:', e);
      }
    }
    const res = await fetch('/api/fleet/deck-bell', { method: 'POST' });
    const json = await res.json();
    return json.message || 'Bell chimed successfully';
  },

  async getSystemMetrics(): Promise<Record<string, unknown>> {
    if (this.isTauri()) {
      try {
        return await invokeTauri('get_system_metrics');
      } catch (e) {
        console.warn('[API] Tauri get_system_metrics fallback:', e);
      }
    }
    const res = await fetch('/api/system/metrics');
    return res.json();
  },

  async getNetworkQrCode(url?: string): Promise<{ url: string; dataUrl: string }> {
    const query = url ? `?url=${encodeURIComponent(url)}` : '';
    const res = await fetch(`/api/network/qrcode${query}`);
    return res.json();
  },

  async getExecutiveBriefing(): Promise<Record<string, unknown>[]> {
    if (this.isTauri()) {
      try {
        return await invokeTauri<Record<string, unknown>[]>('get_executive_briefing');
      } catch (e) {
        console.warn('[API] Tauri get_executive_briefing fallback:', e);
      }
    }
    const res = await fetch('/api/system/executive-briefing');
    return res.json();
  },

  async getHarborProviders(): Promise<Record<string, unknown>[]> {
    if (this.isTauri()) {
      try {
        return await invokeTauri<Record<string, unknown>[]>('get_harbor_providers');
      } catch (e) {
        console.warn('[API] Tauri get_harbor_providers fallback:', e);
      }
    }
    const res = await fetch('/api/providers/harbor');
    return res.json();
  },

  async getDiagnostics(): Promise<Record<string, unknown>[]> {
    if (this.isTauri()) {
      try {
        return await invokeTauri<Record<string, unknown>[]>('get_diagnostics');
      } catch (e) {
        console.warn('[API] Tauri get_diagnostics fallback:', e);
      }
    }
    const res = await fetch('/api/diagnostics');
    return res.json();
  },

  async applyRemedy(): Promise<Record<string, unknown>> {
    if (this.isTauri()) {
      try {
        return await invokeTauri('apply_remedy');
      } catch (e) {
        console.warn('[API] Tauri apply_remedy fallback:', e);
      }
    }
    const res = await fetch('/api/diagnostics/remedy', { method: 'POST' });
    return res.json();
  },

  async getSnapshots(): Promise<Record<string, unknown>[]> {
    if (this.isTauri()) {
      try {
        return await invokeTauri<Record<string, unknown>[]>('get_snapshots');
      } catch (e) {
        console.warn('[API] Tauri get_snapshots fallback:', e);
      }
    }
    const res = await fetch('/api/snapshots');
    return res.json();
  },

  async createSnapshot(title?: string): Promise<Record<string, unknown>> {
    if (this.isTauri()) {
      try {
        return await invokeTauri('create_snapshot', { title: title || 'Manual Sovereign Fleet Snapshot' });
      } catch (e) {
        console.warn('[API] Tauri create_snapshot fallback:', e);
      }
    }
    const res = await fetch('/api/snapshots', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title })
    });
    return res.json();
  },

  async getEngineProcesses(): Promise<ProcessItem[]> {
    if (this.isTauri()) {
      try {
        const data = await invokeTauri<Array<{ pid: number; name: string; memory: number; cpu: number }>>('get_engine_processes');
        // Map backend response {pid,name,memory,cpu} to ProcessItem interface
        return data.map((item) => ({
          id: String(item.pid), // pid -> id
          name: item.name,
          command: '', // missing from API - omitted
          pid: item.pid,
          cpu: item.cpu,
          memoryMB: item.memory, // memory -> memoryMB
          uptime: '', // missing from API - omitted
          status: 'running' // default status
        }));
      } catch (e) {
        console.warn('[API] Tauri get_engine_processes fallback:', e);
      }
    }
    const res = await fetch('/api/engine/processes');
    const data = await res.json() as Array<{ pid: number; name: string; memory: number; cpu: number }>;
    // Map backend response {pid,name,memory,cpu} to ProcessItem interface
    return data.map((item) => ({
      id: String(item.pid), // pid -> id
      name: item.name,
      command: '', // missing from API - omitted
      pid: item.pid,
      cpu: item.cpu,
      memoryMB: item.memory, // memory -> memoryMB
      uptime: '', // missing from API - omitted
      status: 'running' // default status
    }));
  },

  async executeTerminalCommand(
    command: string,
    approved = false,
    authToken?: string
  ): Promise<{ stdout: string; stderr?: string; exitCode: number; duration: string; riskTier?: string; requiresApproval?: boolean }> {
    try {
      const headers: Record<string, string> = { 'Content-Type': 'application/json' };
      if (authToken) {
        headers['Authorization'] = `Bearer ${authToken}`;
      } else {
        const storedToken = localStorage.getItem('galleon_auth_token');
        if (storedToken) {
          headers['Authorization'] = `Bearer ${storedToken}`;
        }
      }

      const res = await fetch('/api/terminal/exec', {
        method: 'POST',
        headers,
        body: JSON.stringify({ command, approved, approved_by: 'web_terminal_ui' })
      });

      if (res.status === 401) {
        const data = await res.json().catch(() => ({}));
        return {
          stdout: '',
          stderr: data.error || data.stderr || 'Unauthorized: caller principal not verified (S2-SEC-01)',
          exitCode: 1,
          duration: '0ms'
        };
      }

      if (!res.ok && res.status !== 403) {
        throw new Error(`Engine returned ${res.status}`);
      }
      return await res.json();
    } catch (e: any) {
      return {
        stdout: '',
        stderr: e.message || 'Failed to connect to Go Policy Gateway (:9090)',
        exitCode: 1,
        duration: '0ms'
      };
    }
  },

  async getFleetPolicies(): Promise<{ policies: Record<string, unknown>[]; riskTiers: Record<string, unknown>[] }> {
    if (this.isTauri()) {
      try {
        return await invokeTauri('get_fleet_policies');
      } catch (e) {
        console.warn('[API] Tauri get_fleet_policies fallback:', e);
      }
    }
    const res = await fetch('/api/fleet/policies');
    return res.json();
  },

  async chatQuartermaster(message: string, context?: Record<string, unknown>): Promise<{
    reply: string;
    suggestedActions?: { label: string; actionType: string; payload?: string }[];
    generatedArtifactPreview?: Record<string, unknown>;
  }> {
    if (this.isTauri()) {
      try {
        return await invokeTauri('chat_quartermaster', { message, context });
      } catch (e) {
        console.warn('[API] Tauri chat_quartermaster fallback:', e);
      }
    }
    const res = await fetch('/api/chat/quartermaster', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message, context })
    });
    return res.json();
  },

  async getHealth(): Promise<Record<string, unknown>> {
    const res = await fetch('/api/health');
    return res.json();
  },

  async getNetwork(): Promise<Record<string, unknown>> {
    const res = await fetch('/api/system/network');
    return res.json();
  },

  async get<T>(path: string): Promise<T> {
    const url = path.startsWith('/api') ? path : `/api${path}`;
    if (this.isTauri()) {
      try {
        return await invokeTauri('get_api', { path: url });
      } catch (e) {
        console.warn(`[API] Tauri get("${url}") fallback:`, e);
      }
    }
    const res = await fetch(url);
    if (!res.ok) {
      throw new Error(`GET ${url} failed with status ${res.status}`);
    }
    return res.json();
  },

  async post<T>(path: string, body?: unknown): Promise<T> {
    const url = path.startsWith('/api') ? path : `/api${path}`;
    if (this.isTauri()) {
      try {
        return await invokeTauri('post_api', { path: url, body });
      } catch (e) {
        console.warn(`[API] Tauri post("${url}") fallback:`, e);
      }
    }
    const res = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });
    if (!res.ok) {
      throw new Error(`POST ${url} failed with status ${res.status}`);
    }
    return res.json();
  },

  async approveToolExecution(id: string, decision: 'approved' | 'rejected', reason?: string): Promise<{ status: string }> {
    const action = decision === 'approved' ? 'approve' : 'deny';
    const res = await fetch(`/api/v1/tool-executions/${id}/${action}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ reason: reason || `Decision by Captain: ${decision}` })
    });
    if (!res.ok) {
      throw new Error(`Failed to ${action} tool execution ${id}: ${res.statusText}`);
    }
    return res.json();
  }
};
