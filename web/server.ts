import express, { Request, Response } from 'express';
import os from 'os';
import path from 'path';
import { fileURLToPath } from 'url';
import http from 'http';
import dotenv from 'dotenv';

dotenv.config();

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const app = express();
const port = parseInt(process.env.PORT || '3000', 10);
const host = process.env.HOST || '0.0.0.0';
const isProduction = process.env.NODE_ENV === 'production';
const engineUrl = process.env.ENGINE_URL || 'http://127.0.0.1:9090';

app.use(express.json({ limit: '10mb' }));

// Helper: Discover active local IPv4 network interfaces
interface NetworkAddress {
  name: string;
  address: string;
  url: string;
}

function getNetworkInterfaces(): NetworkAddress[] {
  const interfaces = os.networkInterfaces();
  const addresses: NetworkAddress[] = [];

  for (const [name, netInterface] of Object.entries(interfaces)) {
    if (!netInterface) continue;
    for (const net of netInterface) {
      if (net.family === 'IPv4' && !net.internal) {
        addresses.push({
          name,
          address: net.address,
          url: `http://${net.address}:${port}`
        });
      }
    }
  }

  addresses.unshift({
    name: 'Loopback',
    address: '127.0.0.1',
    url: `http://localhost:${port}`
  });

  return addresses;
}

// -------------------------------------------------------------
// HOST GATEWAY: Remote Access (Telemetry & QR delegated to Go Engine :9090 per ADR 0002)
// -------------------------------------------------------------
// Host network info — shape matches the frontend NetworkInfo contract (RemoteAccessModal)
app.get('/api/system/network', (_req: Request, res: Response) => {
  const interfaces = getNetworkInterfaces().filter((a) => a.address !== '127.0.0.1');
  const preferred = interfaces[0]?.url || `http://localhost:${port}`;
  res.json({
    hostname: os.hostname(),
    port,
    host,
    interfaces,
    preferredLanUrl: preferred,
    isProduction,
    platform: os.platform(),
    memoryUsageMB: Math.round((os.totalmem() - os.freemem()) / (1024 * 1024))
  });
});



// -------------------------------------------------------------
// REVERSE PROXY TO GO ORCHESTRATOR (All Fleet & Business APIs)
// -------------------------------------------------------------
app.use('/api', async (req: Request, res: Response) => {
  const targetUrl = `${engineUrl}/api${req.url}`;
  try {
    const headers: Record<string, string> = {};
    for (const [k, v] of Object.entries(req.headers)) {
      if (v && k.toLowerCase() !== 'host' && k.toLowerCase() !== 'content-length') {
        headers[k] = Array.isArray(v) ? v.join(',') : v;
      }
    }

    const fetchOptions: RequestInit = {
      method: req.method,
      headers
    };

    if (req.method !== 'GET' && req.method !== 'HEAD' && req.body) {
      // Preserve the caller's content-type; only default to JSON when none was forwarded.
      if (!headers['content-type']) headers['content-type'] = 'application/json';
      fetchOptions.body = JSON.stringify(req.body);
    }

    const response = await fetch(targetUrl, fetchOptions);
    res.status(response.status);
    response.headers.forEach((val, key) => {
      res.setHeader(key, val);
    });

    const data = await response.arrayBuffer();
    res.send(Buffer.from(data));
  } catch (err: any) {
    res.status(503).json({
      error: 'Go Orchestrator Unavailable',
      message: `Failed to proxy to Go engine at ${engineUrl}. Ensure engine/cmd/agent-engine is running.`,
      detail: err?.message
    });
  }
});

// -------------------------------------------------------------
// VITE CLIENT MOUNTING (Dev & Prod Multi-Platform Delivery)
// -------------------------------------------------------------
async function startServer() {
  const server = http.createServer(app);

  if (!isProduction) {
    const { createServer: createViteServer } = await import('vite');
    const vite = await createViteServer({
      server: {
        middlewareMode: true,
        hmr: false
      },
      appType: 'spa'
    });
    app.use(vite.middlewares);
  } else {
    const distPath = path.resolve(__dirname, 'dist');
    app.use(express.static(distPath));
    app.get('*', (_req: Request, res: Response) => {
      res.sendFile(path.resolve(distPath, 'index.html'));
    });
  }

  server.listen(port, host, () => {
    const addresses = getNetworkInterfaces();
    console.log(`\n======================================================`);
    console.log(`⚓ Galleon Fleet Web Gateway is running`);
    console.log(`   Host: ${host} | Port: ${port} | Mode: ${isProduction ? 'Production' : 'Development'}`);
    console.log(`   Proxied to Go Orchestrator at: ${engineUrl}`);
    console.log(`------------------------------------------------------`);
    console.log(`📡 Local Access:      http://localhost:${port}`);
    addresses
      .filter((a) => a.address !== '127.0.0.1')
      .forEach((a) => {
        console.log(`📱 LAN / Mobile Access: ${a.url} (${a.name})`);
      });
    console.log(`======================================================\n`);
  });
}

startServer().catch((err) => {
  console.error('Fatal error starting Galleon Fleet Gateway:', err);
  process.exit(1);
});
