#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const apiKey = (process.env.TAVILY_API_KEY || "").trim();
const apiUrl = (process.env.TAVILY_API_URL || "").trim().replace(/\/+$/, "");
const debugLogPath = (process.env.TAVILY_MCP_DEBUG_LOG || "").trim()
  || path.join(os.tmpdir(), "tavily-mcp-debug.log");
let responseFraming = "content-length";

function debugLog(message) {
  try {
    fs.appendFileSync(
      debugLogPath,
      `[${new Date().toISOString()}] pid=${process.pid} ${message}\n`,
      "utf8"
    );
  } catch {}
}

if (!apiKey) {
  debugLog("fatal missing TAVILY_API_KEY");
  console.error("TAVILY_API_KEY environment variable is required.");
  process.exit(1);
}

if (!apiUrl) {
  debugLog("fatal missing TAVILY_API_URL");
  console.error("TAVILY_API_URL environment variable is required.");
  process.exit(1);
}

if (typeof fetch !== "function") {
  debugLog("fatal missing global fetch");
  console.error("Node.js runtime must provide global fetch (Node 18+).");
  process.exit(1);
}

debugLog(
  `startup apiUrl=${apiUrl} keyLen=${apiKey.length} argv0=${process.argv[0]} argv1=${process.argv[1]}`
);

const SERVER_INFO = {
  name: "tavily-local-proxy-mcp",
  version: "1.0.0"
};

const TOOLS = [
  {
    name: "tavily_search",
    description: "Search the web using Tavily.",
    inputSchema: {
      type: "object",
      properties: {
        query: { type: "string", description: "Search query" }
      },
      required: ["query"]
    }
  },
  {
    name: "tavily_extract",
    description: "Extract content from URLs.",
    inputSchema: {
      type: "object",
      properties: {
        urls: {
          oneOf: [
            { type: "array", items: { type: "string" } },
            { type: "string" }
          ],
          description: "One URL or multiple URLs"
        }
      },
      required: ["urls"]
    }
  },
  {
    name: "tavily_crawl",
    description: "Crawl a website from a root URL.",
    inputSchema: {
      type: "object",
      properties: {
        url: { type: "string", description: "Root URL" }
      },
      required: ["url"]
    }
  },
  {
    name: "tavily_map",
    description: "Map website structure from a root URL.",
    inputSchema: {
      type: "object",
      properties: {
        url: { type: "string", description: "Root URL" }
      },
      required: ["url"]
    }
  },
  {
    name: "tavily_research",
    description: "Run Tavily research workflow.",
    inputSchema: {
      type: "object",
      properties: {
        input: { type: "string", description: "Research input" }
      },
      required: ["input"]
    }
  }
];

function asObject(value) {
  return value && typeof value === "object" && !Array.isArray(value) ? value : {};
}

function asArray(value) {
  return Array.isArray(value) ? value : [];
}

function writeMessage(payload) {
  const body = JSON.stringify(payload);
  if (responseFraming === "line") {
    process.stdout.write(`${body}\n`);
  } else {
    const header = `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n`;
    process.stdout.write(header + body);
  }
  const responseId = Object.prototype.hasOwnProperty.call(payload, "id") ? payload.id : "none";
  const errorCode = payload && payload.error ? payload.error.code : "none";
  debugLog(
    `writeMessage framing=${responseFraming} id=${String(responseId)} errorCode=${String(errorCode)} contentLength=${Buffer.byteLength(
      body,
      "utf8"
    )}`
  );
}

function result(id, value) {
  return { jsonrpc: "2.0", id, result: value };
}

function error(id, code, message, data) {
  const payload = {
    jsonrpc: "2.0",
    id: id ?? null,
    error: { code, message }
  };
  if (data !== undefined) {
    payload.error.data = data;
  }
  return payload;
}

function toolRequest(name, args) {
  const input = asObject(args);

  switch (name) {
    case "tavily_search": {
      const payload = { ...input };
      if (payload.country) {
        payload.topic = "general";
      }
      payload.include_domains = asArray(payload.include_domains);
      payload.exclude_domains = asArray(payload.exclude_domains);
      return { path: "/search", payload };
    }
    case "tavily_extract":
      return { path: "/extract", payload: input };
    case "tavily_crawl": {
      const payload = { ...input };
      payload.select_paths = asArray(payload.select_paths);
      payload.select_domains = asArray(payload.select_domains);
      return { path: "/crawl", payload };
    }
    case "tavily_map": {
      const payload = { ...input };
      payload.select_paths = asArray(payload.select_paths);
      payload.select_domains = asArray(payload.select_domains);
      return { path: "/map", payload };
    }
    case "tavily_research":
      return { path: "/research", payload: input };
    default:
      return null;
  }
}

async function postToTavily(path, payload) {
  const started = Date.now();
  const response = await fetch(`${apiUrl}${path}`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      authorization: `Bearer ${apiKey}`
    },
    body: JSON.stringify(payload)
  });

  const rawBody = await response.text();
  debugLog(
    `postToTavily path=${path} status=${response.status} elapsedMs=${Date.now() - started} bodyBytes=${Buffer.byteLength(
      rawBody,
      "utf8"
    )}`
  );
  let parsedBody;
  try {
    parsedBody = rawBody ? JSON.parse(rawBody) : null;
  } catch {
    parsedBody = rawBody;
  }

  if (!response.ok) {
    const detail =
      parsedBody && typeof parsedBody === "object"
        ? parsedBody.detail || parsedBody.message || JSON.stringify(parsedBody)
        : String(parsedBody || response.statusText || "Unknown error");
    return {
      content: [{ type: "text", text: `Tavily API error (${response.status}): ${detail}` }],
      isError: true
    };
  }

  const text = typeof parsedBody === "string" ? parsedBody : JSON.stringify(parsedBody, null, 2);
  return { content: [{ type: "text", text }] };
}

async function handleRequest(request) {
  if (!request || typeof request !== "object" || request.jsonrpc !== "2.0") {
    debugLog("handleRequest invalid request envelope");
    return error(null, -32600, "Invalid Request");
  }

  const id = Object.prototype.hasOwnProperty.call(request, "id") ? request.id : null;
  const method = request.method;
  debugLog(`handleRequest begin id=${String(id)} method=${String(method)}`);
  if (typeof method !== "string") {
    return error(id, -32600, "Invalid Request");
  }

  if (method === "initialize") {
    const params = asObject(request.params);
    const protocolVersion =
      typeof params.protocolVersion === "string" ? params.protocolVersion : "2024-11-05";

    return result(id, {
      protocolVersion,
      capabilities: { tools: {} },
      serverInfo: SERVER_INFO
    });
  }

  if (method === "tools/list") {
    return result(id, { tools: TOOLS });
  }

  if (method === "tools/call") {
    const params = asObject(request.params);
    const toolName = typeof params.name === "string" ? params.name : "";
    if (!toolName) {
      return error(id, -32602, "Invalid params: missing tool name");
    }

    const mapped = toolRequest(toolName, params.arguments);
    if (!mapped) {
      return error(id, -32601, `Unknown tool: ${toolName}`);
    }

    try {
      const toolResult = await postToTavily(mapped.path, mapped.payload);
      debugLog(`handleRequest tools/call ok id=${String(id)} tool=${toolName}`);
      return result(id, toolResult);
    } catch (requestError) {
      const message = requestError instanceof Error ? requestError.message : String(requestError);
      debugLog(`handleRequest tools/call failed id=${String(id)} tool=${toolName} err=${message}`);
      return result(id, {
        content: [{ type: "text", text: `Tavily proxy request failed: ${message}` }],
        isError: true
      });
    }
  }

  if (method.startsWith("notifications/")) {
    return null;
  }

  return error(id, -32601, `Method not found: ${method}`);
}

let buffer = Buffer.alloc(0);
let drainQueue = Promise.resolve();

function consumeLeadingWhitespace() {
  while (buffer.length > 0) {
    const ch = buffer[0];
    if (ch === 0x0a || ch === 0x0d || ch === 0x20 || ch === 0x09) {
      buffer = buffer.slice(1);
      continue;
    }
    break;
  }
}

async function dispatchMessage(message) {
  if (Array.isArray(message)) {
    debugLog(`dispatchMessage batch messages=${message.length}`);
    for (const entry of message) {
      const response = await handleRequest(entry);
      if (response) {
        writeMessage(response);
      }
    }
    return;
  }

  debugLog(
    `dispatchMessage method=${String(message && message.method)} id=${String(
      message && Object.prototype.hasOwnProperty.call(message, "id") ? message.id : "none"
    )}`
  );
  const response = await handleRequest(message);
  if (response) {
    writeMessage(response);
  }
}

function tryReadLineMessage() {
  if (buffer.length === 0) {
    return null;
  }

  const first = buffer[0];
  if (first !== 0x7b && first !== 0x5b) {
    return null;
  }

  const newlineIndex = buffer.indexOf(0x0a);
  if (newlineIndex === -1) {
    return null;
  }

  const line = buffer.slice(0, newlineIndex).toString("utf8").replace(/\r$/, "").trim();
  buffer = buffer.slice(newlineIndex + 1);

  if (!line) {
    return { kind: "line", message: null };
  }

  return { kind: "line", message: JSON.parse(line) };
}

function findHeaderBoundary(rawBuffer) {
  const crlfBoundary = rawBuffer.indexOf("\r\n\r\n");
  const lfBoundary = rawBuffer.indexOf("\n\n");

  if (crlfBoundary === -1 && lfBoundary === -1) {
    return null;
  }

  if (crlfBoundary !== -1 && (lfBoundary === -1 || crlfBoundary < lfBoundary)) {
    return { headerEnd: crlfBoundary, delimiterLength: 4 };
  }

  return { headerEnd: lfBoundary, delimiterLength: 2 };
}

async function processBuffer() {
  while (true) {
    consumeLeadingWhitespace();

    const lineMessage = tryReadLineMessage();
    if (lineMessage) {
      responseFraming = "line";
      if (lineMessage.message !== null) {
        await dispatchMessage(lineMessage.message);
      }
      continue;
    }

    const boundary = findHeaderBoundary(buffer);
    if (!boundary) {
      return;
    }

    const { headerEnd, delimiterLength } = boundary;
    const headerText = buffer.slice(0, headerEnd).toString("utf8");
    const contentLengthMatch = headerText.match(/Content-Length:\s*(\d+)/i);
    if (!contentLengthMatch) {
      buffer = Buffer.alloc(0);
      writeMessage(error(null, -32700, "Parse error: missing Content-Length header"));
      return;
    }

    const contentLength = Number(contentLengthMatch[1]);
    const bodyStart = headerEnd + delimiterLength;
    if (buffer.length < bodyStart + contentLength) {
      return;
    }

    const body = buffer.slice(bodyStart, bodyStart + contentLength).toString("utf8");
    buffer = buffer.slice(bodyStart + contentLength);

    let message;
    try {
      message = JSON.parse(body);
      responseFraming = "content-length";
    } catch {
      debugLog("processBuffer json parse failed");
      writeMessage(error(null, -32700, "Parse error: invalid JSON body"));
      continue;
    }

    await dispatchMessage(message);
  }
}

process.stdin.on("data", (chunk) => {
  debugLog(`stdin data bytes=${chunk.length}`);
  buffer = Buffer.concat([buffer, chunk]);
  drainQueue = drainQueue
    .then(() => processBuffer())
    .catch((err) => {
      const message = err instanceof Error ? err.message : String(err);
      debugLog(`processBuffer exception err=${message}`);
      writeMessage(error(null, -32603, `Internal error: ${message}`));
    });
});

process.stdin.on("end", () => {
  debugLog("stdin end");
  process.exit(0);
});

process.on("uncaughtException", (err) => {
  const message = err instanceof Error ? `${err.name}: ${err.message}` : String(err);
  debugLog(`uncaughtException ${message}`);
});

process.on("unhandledRejection", (reason) => {
  const message = reason instanceof Error ? `${reason.name}: ${reason.message}` : String(reason);
  debugLog(`unhandledRejection ${message}`);
});

process.stdin.resume();
