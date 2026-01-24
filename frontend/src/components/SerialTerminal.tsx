import { useState, useRef, useEffect, useCallback } from "preact/hooks";
import { Play, Square, Trash2, Download, Monitor, Server } from "lucide-preact";

// Web Serial API types (Chrome/Edge only)
interface SerialPort {
  readable: ReadableStream<Uint8Array> | null;
  writable: WritableStream<Uint8Array> | null;
  open(options: { baudRate: number; dataBits?: number; stopBits?: number; parity?: string; flowControl?: string }): Promise<void>;
  close(): Promise<void>;
}

interface NavigatorWithSerial extends Navigator {
  serial?: {
    requestPort(): Promise<SerialPort>;
  };
}

type ConnectionMode = "browser" | "server";

interface SerialPortInfo {
  device: string;
  description: string;
  manufacturer: string | null;
}

export function SerialTerminal() {
  const webSerialSupported = 'serial' in navigator;

  const [mode, setMode] = useState<ConnectionMode>(webSerialSupported ? "browser" : "server");
  const [isConnected, setIsConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [output, setOutput] = useState<string[]>([]);
  const [input, setInput] = useState('');
  const [error, setError] = useState<string | null>(null);

  // Server mode state
  const [serverPorts, setServerPorts] = useState<SerialPortInfo[]>([]);
  const [selectedPort, setSelectedPort] = useState<string>('');
  const [loadingPorts, setLoadingPorts] = useState(false);

  // Refs for connections
  const portRef = useRef<SerialPort | null>(null);
  const readerRef = useRef<ReadableStreamDefaultReader<Uint8Array> | null>(null);
  const writerRef = useRef<WritableStreamDefaultWriter<Uint8Array> | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const outputRef = useRef<HTMLDivElement>(null);
  const lineCompleteRef = useRef<boolean>(true); // Track if last line ended with newline

  // Auto-scroll to bottom
  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [output]);

  const appendOutput = useCallback((text: string) => {
    setOutput(prev => {
      // Normalize line endings
      const normalized = text.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
      const endsWithNewline = normalized.endsWith('\n');

      // Split into lines
      let lines = normalized.split('\n');
      // Remove trailing empty string from split (if text ended with \n)
      if (lines[lines.length - 1] === '') {
        lines = lines.slice(0, -1);
      }

      if (lines.length === 0) return prev;

      const newOutput = [...prev];

      lines.forEach((line, i) => {
        if (i === 0 && newOutput.length > 0 && !lineCompleteRef.current) {
          // Previous line was incomplete, append to it
          newOutput[newOutput.length - 1] += line;
        } else {
          // Start new line
          newOutput.push(line);
        }
      });

      // Update ref: line is complete if text ended with newline
      lineCompleteRef.current = endsWithNewline;

      return newOutput.slice(-500);
    });
  }, []);

  // Load server ports
  const loadServerPorts = useCallback(async () => {
    setLoadingPorts(true);
    try {
      const response = await fetch('/api/serial/ports');
      if (response.ok) {
        const ports = await response.json();
        setServerPorts(ports);
        if (ports.length > 0 && !selectedPort) {
          setSelectedPort(ports[0].device);
        }
      } else if (response.status === 501) {
        setError('pyserial not installed on server. Run: pip install pyserial');
      }
    } catch {
      setError('Failed to load serial ports from server');
    } finally {
      setLoadingPorts(false);
    }
  }, [selectedPort]);

  // Load ports when switching to server mode
  useEffect(() => {
    if (mode === 'server' && serverPorts.length === 0) {
      loadServerPorts();
    }
  }, [mode, serverPorts.length, loadServerPorts]);

  // Browser mode: Web Serial API
  const connectBrowser = useCallback(async () => {
    setIsConnecting(true);
    setError(null);

    try {
      const nav = navigator as NavigatorWithSerial;
      if (!nav.serial) throw new Error('Web Serial not available');
      const port = await nav.serial.requestPort();

      await port.open({
        baudRate: 115200,
        dataBits: 8,
        stopBits: 1,
        parity: 'none',
        flowControl: 'none',
      });

      portRef.current = port;
      setIsConnected(true);
      appendOutput('--- Connected via Web Serial (browser) ---\n');

      const reader = port.readable?.getReader();
      if (reader) {
        readerRef.current = reader;
        readBrowserLoop(reader);
      }

      const writer = port.writable?.getWriter();
      if (writer) {
        writerRef.current = writer;
      }
    } catch (e) {
      const message = e instanceof Error ? e.message : 'Failed to connect';
      setError(message);
    } finally {
      setIsConnecting(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [appendOutput]);

  const readBrowserLoop = useCallback(async (reader: ReadableStreamDefaultReader<Uint8Array>) => {
    const decoder = new TextDecoder();
    try {
      while (true) {
        const { value, done } = await reader.read();
        if (done) break;
        if (value) {
          appendOutput(decoder.decode(value));
        }
      }
    } catch (e) {
      if ((e as Error).name !== 'NetworkError') {
        appendOutput(`--- Read error: ${(e as Error).message} ---\n`);
      }
    }
  }, [appendOutput]);

  // Server mode: WebSocket proxy
  const connectServer = useCallback(async () => {
    if (!selectedPort) {
      setError('Select a serial port first');
      return;
    }

    setIsConnecting(true);
    setError(null);

    try {
      // First connect to port via REST API
      const connectResponse = await fetch('/api/serial/connect', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ port: selectedPort, baudrate: 115200 }),
      });

      if (!connectResponse.ok) {
        const err = await connectResponse.json();
        throw new Error(err.detail || 'Failed to connect');
      }

      // Then open WebSocket for real-time data
      const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const ws = new WebSocket(`${wsProtocol}//${window.location.host}/api/serial/ws`);

      ws.onopen = () => {
        setIsConnected(true);
        appendOutput(`--- Connected to ${selectedPort} via server ---\n`);
      };

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);
          if (msg.type === 'data') {
            appendOutput(msg.data);
          } else if (msg.type === 'error') {
            appendOutput(`--- Error: ${msg.message} ---\n`);
          }
        } catch {
          appendOutput(event.data);
        }
      };

      ws.onerror = () => {
        setError('WebSocket error');
      };

      ws.onclose = () => {
        setIsConnected(false);
        appendOutput('--- Disconnected ---\n');
      };

      wsRef.current = ws;
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Connection failed');
    } finally {
      setIsConnecting(false);
    }
  }, [selectedPort, appendOutput]);

  const connect = useCallback(() => {
    if (mode === 'browser') {
      connectBrowser();
    } else {
      connectServer();
    }
  }, [mode, connectBrowser, connectServer]);

  const disconnect = useCallback(async () => {
    if (mode === 'browser') {
      try {
        if (readerRef.current) {
          await readerRef.current.cancel();
          readerRef.current.releaseLock();
          readerRef.current = null;
        }
        if (writerRef.current) {
          await writerRef.current.close();
          writerRef.current = null;
        }
        if (portRef.current) {
          await portRef.current.close();
          portRef.current = null;
        }
      } catch {
        // Ignore close errors
      }
    } else {
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
      await fetch('/api/serial/disconnect', { method: 'POST' });
    }
    setIsConnected(false);
    appendOutput('--- Disconnected ---\n');
  }, [mode, appendOutput]);

  const sendCommand = useCallback(async (cmd?: string) => {
    const command = cmd || input;
    if (!command) return;

    try {
      if (mode === 'browser' && writerRef.current) {
        const encoder = new TextEncoder();
        await writerRef.current.write(encoder.encode(command + '\r\n'));
      } else if (mode === 'server' && wsRef.current) {
        wsRef.current.send(JSON.stringify({ type: 'send', data: command }));
      }
      appendOutput(`> ${command}\n`);
      if (!cmd) setInput('');
    } catch (e) {
      appendOutput(`--- Send error: ${(e as Error).message} ---\n`);
    }
  }, [mode, input, appendOutput]);

  const clearOutput = useCallback(() => {
    setOutput([]);
    lineCompleteRef.current = true;
  }, []);

  const downloadLog = useCallback(() => {
    const content = output.join('\n');
    const blob = new Blob([content], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `spoolbuddy-serial-${new Date().toISOString().slice(0, 19).replace(/[:-]/g, '')}.log`;
    a.click();
    URL.revokeObjectURL(url);
  }, [output]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (wsRef.current) wsRef.current.close();
    };
  }, []);

  return (
    <div class="space-y-3">
      {/* Mode selector */}
      <div class="flex items-center gap-2 text-sm">
        <span class="text-[var(--text-muted)]">Mode:</span>
        <button
          onClick={() => { setMode('browser'); disconnect(); }}
          disabled={isConnected}
          class={`flex items-center gap-1 px-2 py-1 rounded ${mode === 'browser' ? 'bg-[var(--accent-color)] text-white' : 'bg-[var(--card-bg)] text-[var(--text-secondary)]'} ${!webSerialSupported ? 'opacity-50' : ''}`}
          title={webSerialSupported ? 'Connect directly via browser (Chrome/Edge)' : 'Not supported in this browser'}
        >
          <Monitor class="w-3 h-3" />
          Browser
          {!webSerialSupported && <span class="text-xs">(N/A)</span>}
        </button>
        <button
          onClick={() => { setMode('server'); disconnect(); }}
          disabled={isConnected}
          class={`flex items-center gap-1 px-2 py-1 rounded ${mode === 'server' ? 'bg-[var(--accent-color)] text-white' : 'bg-[var(--card-bg)] text-[var(--text-secondary)]'}`}
          title="Connect via server (works in any browser)"
        >
          <Server class="w-3 h-3" />
          Server
        </button>
      </div>

      {/* Server mode: port selector */}
      {mode === 'server' && !isConnected && (
        <div class="flex gap-2">
          <select
            class="input flex-1 text-sm"
            value={selectedPort}
            onChange={(e) => setSelectedPort((e.target as HTMLSelectElement).value)}
            disabled={isConnecting || loadingPorts}
          >
            {serverPorts.length === 0 ? (
              <option value="">No ports found</option>
            ) : (
              serverPorts.map((port) => (
                <option key={port.device} value={port.device}>
                  {port.device} - {port.description}
                </option>
              ))
            )}
          </select>
          <button
            onClick={loadServerPorts}
            disabled={loadingPorts}
            class="btn text-sm"
          >
            {loadingPorts ? '...' : 'Refresh'}
          </button>
        </div>
      )}

      {/* Toolbar */}
      <div class="flex items-center gap-2">
        {!isConnected ? (
          <button
            onClick={connect}
            disabled={isConnecting || (mode === 'server' && !selectedPort)}
            class="btn btn-primary flex items-center gap-2 text-sm"
          >
            <Play class="w-4 h-4" />
            {isConnecting ? 'Connecting...' : 'Connect'}
          </button>
        ) : (
          <button onClick={disconnect} class="btn flex items-center gap-2 text-sm">
            <Square class="w-4 h-4" />
            Disconnect
          </button>
        )}
        <button onClick={clearOutput} class="btn text-sm" title="Clear"><Trash2 class="w-4 h-4" /></button>
        <button onClick={downloadLog} disabled={output.length === 0} class="btn text-sm" title="Download"><Download class="w-4 h-4" /></button>
        <div class="flex-1" />
        <span class={`text-xs px-2 py-1 rounded ${isConnected ? 'bg-green-500/20 text-green-500' : 'bg-[var(--text-muted)]/20 text-[var(--text-muted)]'}`}>
          {isConnected ? `Connected (${mode})` : 'Disconnected'}
        </span>
      </div>

      {error && (
        <div class="p-2 bg-red-500/10 border border-red-500/30 rounded text-red-500 text-xs">
          {error}
        </div>
      )}

      {/* Terminal output */}
      <div
        ref={outputRef}
        class="bg-black rounded-lg p-3 h-64 overflow-y-auto font-mono text-xs text-green-400 whitespace-pre-wrap"
      >
        {output.length === 0 ? (
          <span class="text-[var(--text-muted)]">
            {mode === 'browser'
              ? 'Click "Connect" to connect to ESP32 via USB...'
              : 'Select a port and click "Connect"...'}
          </span>
        ) : (
          output.map((line, i) => <div key={i}>{line}</div>)
        )}
      </div>

      {/* Input */}
      <div class="flex gap-2">
        <input
          type="text"
          class="input flex-1 font-mono text-sm"
          placeholder={isConnected ? "Enter command..." : "Connect first..."}
          value={input}
          onInput={(e) => setInput((e.target as HTMLInputElement).value)}
          onKeyDown={(e) => e.key === 'Enter' && sendCommand()}
          disabled={!isConnected}
        />
        <button onClick={() => sendCommand()} disabled={!isConnected || !input} class="btn btn-primary">
          Send
        </button>
      </div>

      {/* Quick commands */}
      {isConnected && (
        <div class="flex flex-wrap gap-2">
          <span class="text-xs text-[var(--text-muted)]">Quick:</span>
          {['help', 'status', 'reboot', 'wifi scan'].map((cmd) => (
            <button
              key={cmd}
              onClick={() => sendCommand(cmd)}
              class="text-xs px-2 py-1 bg-[var(--card-bg)] border border-[var(--border-color)] rounded hover:border-[var(--accent-color)]"
            >
              {cmd}
            </button>
          ))}
          <button
            onClick={() => sendCommand('factory-reset')}
            class="text-xs px-2 py-1 bg-red-500/20 border border-red-500/30 rounded hover:bg-red-500/30 text-red-500"
          >
            factory-reset
          </button>
        </div>
      )}

      <p class="text-xs text-[var(--text-muted)]">
        {mode === 'browser'
          ? 'Browser mode: USB must be connected to this computer. Works in Chrome/Edge only.'
          : 'Server mode: USB must be connected to the server. Works in any browser.'}
      </p>
    </div>
  );
}
