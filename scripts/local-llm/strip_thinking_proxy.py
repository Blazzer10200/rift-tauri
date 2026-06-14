#!/usr/bin/env python3
"""Strip-`thinking` shim that sits in FRONT of LiteLLM for Rift local-LLM mode.

WHY: the spawned Claude Code CLI sends an Anthropic `thinking` block on every
turn (interleaved-thinking beta, no flag disables it). LiteLLM's experimental
Anthropic `/v1/messages` adapter maps `thinking` -> Ollama's `think` and forwards
it; `ollama/qwen3-coder:30b` rejects it at runtime -> HTTP 500
`OllamaException - "qwen3-coder:30b" does not support thinking`.

LiteLLM's own drop_params/additional_drop_params don't reach this adapter path
(thinking is statically "supported" by the ollama provider; the rejection is
model-specific + runtime — see LiteLLM #8199). So we strip it one hop earlier.

TOPOLOGY:  CLI -> this shim (:4000) -> LiteLLM (:4001) -> Ollama (:11434)
The shim removes `thinking` (and `context_management`/`output_config`, which the
CLI also sends and Ollama can't honour) from every JSON request body, then
transparently forwards method, path, headers, and the (streaming) response.

Run:  python strip_thinking_proxy.py [listen_port] [upstream_port]
      defaults: listen 4000, upstream 4001
"""

import http.client
import json
import sys
from socketserver import ThreadingMixIn
from http.server import BaseHTTPRequestHandler, HTTPServer

LISTEN_PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 4000
UPSTREAM_HOST = "127.0.0.1"
UPSTREAM_PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 4001
STRIP_KEYS = ("thinking", "context_management", "output_config")
HOP_BY_HOP = {
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
    "content-length",
}


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *a):  # quiet; LiteLLM already logs upstream
        pass

    def _handle(self):
        length = int(self.headers.get("Content-Length", 0) or 0)
        body = self.rfile.read(length) if length else b""

        # Strip the offending keys from JSON bodies. Non-JSON passes untouched.
        if body:
            try:
                data = json.loads(body)
                if isinstance(data, dict) and any(k in data for k in STRIP_KEYS):
                    for k in STRIP_KEYS:
                        data.pop(k, None)
                    body = json.dumps(data).encode("utf-8")
            except (ValueError, UnicodeDecodeError):
                pass

        headers = {
            k: v
            for k, v in self.headers.items()
            if k.lower() not in HOP_BY_HOP and k.lower() != "host"
        }
        headers["Content-Length"] = str(len(body))

        try:
            conn = http.client.HTTPConnection(UPSTREAM_HOST, UPSTREAM_PORT, timeout=600)
            conn.request(self.command, self.path, body=body, headers=headers)
            resp = conn.getresponse()
        except OSError as e:
            self.send_error(502, f"shim upstream error: {e}")
            return

        # Forward status + headers (minus framing), then stream the body so SSE
        # turns flow through live. Connection: close lets us signal end-of-body
        # without hand-rolling chunked encoding.
        self.send_response(resp.status)
        for k, v in resp.getheaders():
            if k.lower() in HOP_BY_HOP:
                continue
            self.send_header(k, v)
        self.send_header("Connection", "close")
        self.end_headers()
        try:
            while True:
                chunk = resp.read(8192)
                if not chunk:
                    break
                self.wfile.write(chunk)
                self.wfile.flush()
        except OSError:
            pass
        finally:
            conn.close()

    do_GET = _handle
    do_POST = _handle
    do_PUT = _handle
    do_DELETE = _handle


class ThreadingHTTPServer(ThreadingMixIn, HTTPServer):
    daemon_threads = True


if __name__ == "__main__":
    print(f"strip-thinking shim :{LISTEN_PORT} -> litellm :{UPSTREAM_PORT}", flush=True)
    ThreadingHTTPServer(("127.0.0.1", LISTEN_PORT), Handler).serve_forever()
