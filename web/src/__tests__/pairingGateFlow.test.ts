import test from 'node:test';
import assert from 'node:assert/strict';
import {
  TOKEN_KEY,
  getToken,
  setToken,
  clearToken,
  apiClient,
  request,
} from '../utils/apiClient';

// Minimal mock storage for Node environment
class MockLocalStorage {
  private store: Record<string, string> = {};

  getItem(key: string): string | null {
    return this.store[key] ?? null;
  }

  setItem(key: string, value: string): void {
    this.store[key] = String(value);
  }

  removeItem(key: string): void {
    delete this.store[key];
  }

  clear(): void {
    this.store = {};
  }
}

// Minimal mock event target for window
class MockEventTarget {
  private listeners: Record<string, Array<(e: unknown) => void>> = {};

  addEventListener(type: string, listener: (e: unknown) => void): void {
    if (!this.listeners[type]) this.listeners[type] = [];
    this.listeners[type].push(listener);
  }

  removeEventListener(type: string, listener: (e: unknown) => void): void {
    if (!this.listeners[type]) return;
    this.listeners[type] = this.listeners[type].filter((l) => l !== listener);
  }

  dispatchEvent(event: { type: string }): boolean {
    const list = this.listeners[event.type] || [];
    for (const l of list) {
      l(event);
    }
    return true;
  }
}

// Setup mock window & localStorage
const mockStorage = new MockLocalStorage();
const mockWindow = new MockEventTarget();
(mockWindow as any).localStorage = mockStorage;
(globalThis as any).window = mockWindow;
(globalThis as any).localStorage = mockStorage;
(globalThis as any).CustomEvent = class CustomEvent {
  type: string;
  detail: unknown;
  constructor(type: string, params?: { detail?: unknown }) {
    this.type = type;
    this.detail = params?.detail;
  }
};

test('token storage operations: setToken, getToken, clearToken', () => {
  mockStorage.clear();
  assert.equal(getToken(), null);

  setToken('test-token-12345');
  assert.equal(getToken(), 'test-token-12345');
  assert.equal(mockStorage.getItem(TOKEN_KEY), 'test-token-12345');

  clearToken();
  assert.equal(getToken(), null);
  assert.equal(mockStorage.getItem(TOKEN_KEY), null);
});

test('apiClient.getPairCode calls GET /pair/code', async () => {
  const originalFetch = globalThis.fetch;
  let capturedUrl = '';
  let capturedMethod = '';

  globalThis.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
    capturedUrl = input.toString();
    capturedMethod = init?.method || 'GET';
    return {
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({
        success: true,
        pairing_required: true,
        pairing_code: 'PAIR-ABCD-999',
      }),
    } as any;
  };

  try {
    const result = await apiClient.getPairCode();
    assert.equal(capturedUrl, '/pair/code');
    assert.equal(capturedMethod, 'GET');
    assert.equal(result.success, true);
    assert.equal(result.pairing_code, 'PAIR-ABCD-999');
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('apiClient.pair sends POST /pair with X-Pairing-Code header', async () => {
  const originalFetch = globalThis.fetch;
  let capturedUrl = '';
  let capturedHeaders: Record<string, string> = {};
  const sampleAuth = 'sample-pair-tok';

  globalThis.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
    capturedUrl = input.toString();
    capturedHeaders = (init?.headers as Record<string, string>) || {};
    return {
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({
        paired: true,
        persisted: true,
        token: sampleAuth,
        message: 'Paired successfully',
      }),
    } as any;
  };

  try {
    const result = await apiClient.pair(' CODE-123 ');
    assert.equal(capturedUrl, '/pair');
    assert.equal(capturedHeaders['X-Pairing-Code'], 'CODE-123');
    assert.equal(result.paired, true);
    assert.equal(result.token, sampleAuth);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('401 response clears token and dispatches galleon:unauthorized event', async () => {
  mockStorage.clear();
  setToken('expired-token');
  assert.equal(getToken(), 'expired-token');

  let unauthorizedDispatched = false;
  const listener = () => {
    unauthorizedDispatched = true;
  };
  mockWindow.addEventListener('galleon:unauthorized', listener);

  const originalFetch = globalThis.fetch;
  globalThis.fetch = async () => {
    return {
      ok: false,
      status: 401,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({ error: 'Unauthorized' }),
    } as any;
  };

  try {
    await assert.rejects(
      async () => {
        await request('GET', '/api/devices');
      },
      /Unauthorized \(401\)/
    );

    assert.equal(getToken(), null, 'token must be cleared on 401');
    assert.equal(unauthorizedDispatched, true, 'galleon:unauthorized event must be dispatched');
  } finally {
    globalThis.fetch = originalFetch;
    mockWindow.removeEventListener('galleon:unauthorized', listener);
  }
});

test('authenticated requests attach Bearer header when token exists', async () => {
  setToken('active-valid-token');
  const originalFetch = globalThis.fetch;
  let sentAuthHeader = '';

  globalThis.fetch = async (_input: RequestInfo | URL, init?: RequestInit) => {
    const headers = init?.headers as Record<string, string>;
    sentAuthHeader = headers?.['Authorization'] || '';
    return {
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({ count: 0, devices: [] }),
    } as any;
  };

  try {
    await request('GET', '/api/devices');
    assert.equal(sentAuthHeader, 'Bearer active-valid-token');
  } finally {
    clearToken();
    globalThis.fetch = originalFetch;
  }
});
