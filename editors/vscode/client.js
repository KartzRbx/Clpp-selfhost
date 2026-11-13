'use strict';
// Minimal Language Server Protocol client: JSON-RPC over a child process's
// stdio with Content-Length framing. No dependencies, no VS Code API.

const { spawn } = require('child_process');

class LspClient {
  constructor(command, args, options = {}) {
    this.command = command;
    this.args = args;
    this.cwd = options.cwd;
    this.log = options.log || (() => {});
    this.onNotification = options.onNotification || (() => {});
    this.onExit = options.onExit || (() => {});
    this.nextId = 1;
    this.pending = new Map();
    this.buffer = Buffer.alloc(0);
    this.process = null;
  }

  start() {
    this.process = spawn(this.command, this.args, { cwd: this.cwd, stdio: ['pipe', 'pipe', 'pipe'] });
    this.process.stdout.on('data', (chunk) => this.receive(chunk));
    this.process.stderr.on('data', (chunk) => this.log(chunk.toString()));
    this.process.on('error', (err) => {
      this.rejectAll(err);
      this.onExit(err);
    });
    this.process.on('exit', (code) => {
      this.rejectAll(new Error(`language server exited with code ${code}`));
      this.onExit(code);
    });
  }

  rejectAll(err) {
    for (const { reject } of this.pending.values()) reject(err);
    this.pending.clear();
  }

  receive(chunk) {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    for (;;) {
      const headerEnd = this.buffer.indexOf('\r\n\r\n');
      if (headerEnd < 0) return;
      const header = this.buffer.slice(0, headerEnd).toString('ascii');
      const match = /Content-Length:\s*(\d+)/i.exec(header);
      if (!match) {
        this.buffer = this.buffer.slice(headerEnd + 4);
        continue;
      }
      const length = Number(match[1]);
      const start = headerEnd + 4;
      if (this.buffer.length < start + length) return;
      const body = this.buffer.slice(start, start + length).toString('utf8');
      this.buffer = this.buffer.slice(start + length);
      let message;
      try {
        message = JSON.parse(body);
      } catch (err) {
        this.log(`invalid JSON from server: ${err.message}`);
        continue;
      }
      this.log(`<- ${body}`);
      this.dispatch(message);
    }
  }

  dispatch(message) {
    if (message.id !== undefined && this.pending.has(message.id)) {
      const { resolve, reject } = this.pending.get(message.id);
      this.pending.delete(message.id);
      if (message.error) reject(new Error(message.error.message));
      else resolve(message.result);
    } else if (message.method) {
      this.onNotification(message.method, message.params);
    }
  }

  write(message) {
    if (!this.process || !this.process.stdin.writable) return;
    const body = JSON.stringify(message);
    this.log(`-> ${body}`);
    this.process.stdin.write(`Content-Length: ${Buffer.byteLength(body, 'utf8')}\r\n\r\n${body}`);
  }

  request(method, params) {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.write({ jsonrpc: '2.0', id, method, params });
    });
  }

  notify(method, params) {
    this.write({ jsonrpc: '2.0', method, params });
  }

  async stop() {
    if (!this.process) return;
    try {
      await Promise.race([this.request('shutdown', null), new Promise((r) => setTimeout(r, 1000))]);
      this.notify('exit', null);
    } catch (_) {
      // Already gone.
    }
    const proc = this.process;
    this.process = null;
    setTimeout(() => {
      if (proc.exitCode === null) proc.kill();
    }, 500);
  }
}

module.exports = { LspClient };
