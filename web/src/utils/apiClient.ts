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

export interface HarborProvider {
  id: string;
  name: string;
  vendor: string;
  status: string;
  models: string[];
  latencyMs: number;
  capabilities: string[];
}

export interface DiagnosticItem {
  id: string;
  component: string;
  status: 'healthy' | 'warning' | 'error';
  latency: string;
  detail: string;
}

export interface SnapshotItem {
  id: string;
  createdAt: string;
  label: string;
  sizeMB: number;
  entityCount: number;
  checksum: string;
  includes: string[];
}

export interface ExecutiveBriefing {
  title: string;
  date: string;
  admiralStatus: string;
  keyAchievements: string[];
  activeAlerts: string[];
  treasuryBurnRate: string;
  recommendedFocus: string;
  generatedAt: string;
}

export interface FleetMetrics {
  activeVessels: number;
  underwayQuests: number;
  pendingApprovals: number;
  totalTreasuryLedgers: number;
  totalArtifacts: number;
  totalSpecialists: number;
  totalSquads: number;
  systemUptime: string;
  gatewayLatencyMs: number;
  activeWorkers: number;
  memoryUsedMB: number;
}

export interface SystemMetrics {
  cpuUsage: number;
  memoryUsage: {
    usedMB: number;
    totalMB: number;
    percent: number;
  };
  activeProcesses: number;
  uptime: string;
  platform: string;
  arch: string;
  goVersion: string;
  gateway_latency_ms: number;
  active_threads: number;
  isolation_mode: string;
  memory_db_mb: number;
}

export interface NetworkInterfaceInfo {
  name: string;
  address: string;
  url: string;
}

export interface NetworkInfo {
  status?: string;
  local_address?: string;
  gateway_port?: number;
  engine_port?: number;
  hostname?: string;
  port?: number;
  host?: string;
  interfaces?: NetworkInterfaceInfo[];
  preferredLanUrl?: string;
  isProduction?: boolean;
  platform?: string;
  memoryUsageMB?: number;
}

export interface ProjectGitStatus {
  branch: string;
  clean: boolean;
  staged: string[];
  unstaged: string[];
  untracked: string[];
  risk_tier: string;
  policy_check: string;
}

export interface PairResponse {
  paired: boolean;
  persisted: boolean;
  token: string;
  message: string;
}

export interface PairCodeResponse {
  success: boolean;
  pairing_required: boolean;
  pairing_code: string | null;
}

export const TOKEN_KEY = 'galleon_auth_token';

export function getToken(): string | null {
  if (typeof window === 'undefined' || !window.localStorage) {
    return null;
  }
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string): void {
  if (typeof window !== 'undefined' && window.localStorage) {
    localStorage.setItem(TOKEN_KEY, token);
  }
}

export function clearToken(): void {
  if (typeof window !== 'undefined' && window.localStorage) {
    localStorage.removeItem(TOKEN_KEY);
  }
}

export async function request<T>(
  method: string,
  path: string,
  body?: unknown,
  customHeaders?: Record<string, string>
): Promise<T> {
  const normMethod = method.toUpperCase();

  if (isTauriEnvironment()) {
    const token = getToken();
    const res = (await invokeTauri('api_request', {
      method: normMethod,
      path,
      body,
      token: token || undefined,
      headers: customHeaders,
    })) as { status: number; body: T };

    if (res.status === 401) {
      clearToken();
      if (typeof window !== 'undefined') {
        window.dispatchEvent(new CustomEvent('galleon:unauthorized'));
      }
      throw new Error(`Unauthorized (401): ${path}`);
    }

    if (res.status < 200 || res.status >= 300) {
      const errMsg =
        typeof res.body === 'object' && res.body !== null && 'error' in (res.body as any)
          ? (res.body as any).error
          : `${normMethod} ${path} failed with status ${res.status}`;
      throw new Error(errMsg);
    }

    return res.body;
  }

  const headers: Record<string, string> = { ...customHeaders };
  const token = getToken();
  if (token && !headers['Authorization']) {
    headers['Authorization'] = `Bearer ${token}`;
  }
  if (body !== undefined && !headers['Content-Type']) {
    headers['Content-Type'] = 'application/json';
  }

  const res = await fetch(path, {
    method: normMethod,
    headers,
    body: body !== undefined ? (typeof body === 'string' ? body : JSON.stringify(body)) : undefined,
  });

  if (res.status === 401) {
    clearToken();
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('galleon:unauthorized'));
    }
    throw new Error(`Unauthorized (401): ${path}`);
  }

  if (!res.ok) {
    let errMsg = `${normMethod} ${path} failed with status ${res.status}`;
    try {
      const errData = await res.json();
      if (errData && typeof errData === 'object' && 'error' in errData && errData.error) {
        errMsg = errData.error;
      }
    } catch {
      // Ignore JSON parse errors on error responses
    }
    throw new Error(errMsg);
  }

  const contentType = res.headers.get('content-type');
  if (contentType && contentType.includes('application/json')) {
    return (await res.json()) as T;
  }

  const text = await res.text();
  if (!text) {
    return undefined as unknown as T;
  }
  try {
    return JSON.parse(text) as T;
  } catch {
    return text as unknown as T;
  }
}

export const apiClient = {
  isTauri(): boolean {
    return isTauriEnvironment();
  },

  getToken,
  setToken,
  clearToken,
  request,

  async getCollection<T>(name: string): Promise<T[]> {
    return request<T[]>('GET', `/api/collections/${name}`);
  },

  async saveCollection<T>(name: string, data: T[]): Promise<void> {
    await request<void>('POST', `/api/collections/${name}`, data);
  },

  async getFleetMetrics(): Promise<any> {
    return request('GET', '/api/fleet/metrics');
  },

  async ringDeckBell(): Promise<string> {
    const json = await request<{ message?: string }>('POST', '/api/fleet/deck-bell');
    return json?.message || 'Bell chimed successfully';
  },

  async getSystemMetrics(): Promise<SystemMetrics> {
    return request<SystemMetrics>('GET', '/api/system/metrics');
  },

  async getNetworkQrCode(url?: string): Promise<{ url: string; dataUrl: string }> {
    const query = url ? `?url=${encodeURIComponent(url)}` : '';
    return request<{ url: string; dataUrl: string }>('GET', `/api/network/qrcode${query}`);
  },

  async getExecutiveBriefing(): Promise<any> {
    return request('GET', '/api/system/executive-briefing');
  },

  async getHarborProviders(): Promise<HarborProvider[]> {
    return request<HarborProvider[]>('GET', '/api/providers/harbor');
  },

  async getDiagnostics(): Promise<DiagnosticItem[]> {
    return request<DiagnosticItem[]>('GET', '/api/diagnostics');
  },

  async applyRemedy(): Promise<any> {
    return request('POST', '/api/diagnostics/remedy');
  },

  async getSnapshots(): Promise<SnapshotItem[]> {
    return request<SnapshotItem[]>('GET', '/api/snapshots');
  },

  async createSnapshot(title?: string): Promise<SnapshotItem> {
    return request<SnapshotItem>('POST', '/api/snapshots', { title });
  },

  async getEngineProcesses(): Promise<ProcessItem[]> {
    const data = await request<Array<{ pid: number; name: string; memory: number; cpu: number }>>(
      'GET',
      '/api/engine/processes'
    );
    return (data || []).map((item) => ({
      id: String(item.pid),
      name: item.name,
      command: '',
      pid: item.pid,
      cpu: item.cpu,
      memoryMB: item.memory,
      uptime: '',
      status: 'running',
    }));
  },

  async executeTerminalCommand(
    command: string,
    approved = false,
    authToken?: string
  ): Promise<{ stdout: string; stderr?: string; exitCode: number; duration: string; riskTier?: string; requiresApproval?: boolean }> {
    if (authToken) {
      setToken(authToken);
    }
    try {
      return await request('POST', '/api/terminal/exec', {
        command,
        approved,
        approved_by: 'web_terminal_ui',
      });
    } catch (e: any) {
      return {
        stdout: '',
        stderr: e.message || 'Failed to connect to Gateway',
        exitCode: 1,
        duration: '0ms',
      };
    }
  },

  async getFleetPolicies(): Promise<{ policies: Record<string, unknown>[]; riskTiers: Record<string, unknown>[] }> {
    return request('GET', '/api/fleet/policies');
  },

  async chatQuartermaster(message: string, context?: Record<string, unknown>): Promise<{
    reply: string;
    suggestedActions?: { label: string; actionType: string; payload?: string }[];
    generatedArtifactPreview?: Record<string, unknown>;
  }> {
    return request('POST', '/api/chat/quartermaster', { message, context });
  },

  async getNetwork(): Promise<any> {
    return request('GET', '/api/system/network');
  },

  async getProjectGitStatus(path?: string): Promise<ProjectGitStatus> {
    const query = path ? `?path=${encodeURIComponent(path)}` : '';
    return request<ProjectGitStatus>('GET', `/api/project/git-status${query}`);
  },

  async get<T>(path: string): Promise<T> {
    const url = path.startsWith('/api') || path.startsWith('/pair')
      ? path
      : path.startsWith('/') ? `/api${path}` : `/api/${path}`;
    return request<T>('GET', url);
  },

  async post<T>(path: string, body?: unknown): Promise<T> {
    const url = path.startsWith('/api') || path.startsWith('/pair')
      ? path
      : path.startsWith('/') ? `/api${path}` : `/api/${path}`;
    return request<T>('POST', url, body);
  },

  async getPairCode(): Promise<PairCodeResponse> {
    return request<PairCodeResponse>('GET', '/pair/code');
  },

  async pair(code: string): Promise<PairResponse> {
    return request<PairResponse>('POST', '/pair', undefined, {
      'X-Pairing-Code': code.trim(),
    });
  },

  async approveToolExecution(id: string, decision: 'approved' | 'rejected', reason?: string): Promise<{ status: string }> {
    const action = decision === 'approved' ? 'approve' : 'deny';
    return request<{ status: string }>('POST', `/api/v1/tool-executions/${id}/${action}`, {
      reason: reason || `Decision by Captain: ${decision}`,
    });
  },
};
