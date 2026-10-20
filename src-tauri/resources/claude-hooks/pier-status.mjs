#!/usr/bin/env node
// Hook do Claude Code instalado pelo Pier (ver src-tauri/src/claude_agents.rs). Lê o JSON do hook
// no stdin e grava um status por sessão em <diretório do Claude Code>/pier-dock/agents/<id>.json,
// que o Pier lê para mostrar o agent no dock e focar a janela que o lançou.
//
// Nunca deve travar nem falhar visivelmente: um hook quebrado não pode atrapalhar o Claude Code.

import { readFileSync, writeFileSync, mkdirSync, renameSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { join, basename } from "node:path";

function readStdin() {
  try {
    return readFileSync(0, "utf8");
  } catch {
    return "";
  }
}

function statusDir() {
  const base = process.env.CLAUDE_CONFIG_DIR || join(process.env.HOME || process.env.USERPROFILE || ".", ".claude");
  return join(base, "pier-dock", "agents");
}

function statusPath(dir, sessionId) {
  return join(dir, `${sessionId}.json`);
}

function readPrevious(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return null;
  }
}

/** Mapa pid -> ppid de todos os processos vivos, numa chamada só (reaproveitado para a cadeia inteira). */
function parentMap() {
  const map = new Map();
  try {
    if (process.platform === "win32") {
      const out = execFileSync(
        "powershell",
        ["-NoProfile", "-NonInteractive", "-Command", "Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId | ConvertTo-Json -Compress"],
        { encoding: "utf8", timeout: 5000, windowsHide: true },
      );
      const rows = JSON.parse(out);
      for (const row of Array.isArray(rows) ? rows : [rows]) {
        map.set(Number(row.ProcessId), Number(row.ParentProcessId));
      }
    } else {
      const out = execFileSync("ps", ["-axo", "pid=,ppid="], { encoding: "utf8", timeout: 5000 });
      for (const line of out.trim().split("\n")) {
        const [pid, ppid] = line.trim().split(/\s+/).map(Number);
        if (!Number.isNaN(pid)) map.set(pid, ppid);
      }
    }
  } catch {
    // Sem o mapa, a cadeia fica só com o próprio processo do hook.
  }
  return map;
}

/**
 * A cadeia de pids até a raiz — o terminal ou IDE que lançou a sessão está nela, só não sabemos em
 * qual nível. Só é calculada uma vez por sessão (no SessionStart) e reaproveitada depois: o
 * processo do hook muda a cada invocação, mas o terminal/IDE dono da sessão não.
 */
function pidChain() {
  const map = parentMap();
  const chain = [];
  let pid = process.pid;
  for (let i = 0; i < 40 && pid && !chain.includes(pid); i++) {
    chain.push(pid);
    pid = map.get(pid);
  }
  return chain;
}

/**
 * Best effort: nem toda ferramenta devolve um formato padronizado em `tool_response`, então isto
 * só pega os casos mais comuns de erro explícito. Na dúvida, fica "running" — melhor não marcar
 * erro do que marcar um errado.
 */
function statusFor(eventName, payload) {
  switch (eventName) {
    case "Notification":
      return "permission";
    case "Stop":
    case "SubagentStop":
      return "done";
    case "PostToolUse": {
      const r = payload.tool_response;
      const errored = r && typeof r === "object" && (r.is_error === true || !!r.error || r.success === false);
      return errored ? "error" : "running";
    }
    default:
      return "running";
  }
}

function projectName(cwd) {
  return (cwd && basename(cwd)) || cwd || "";
}

function writeAtomic(dir, path, data) {
  const tmp = join(dir, `.tmp-${process.pid}-${Date.now()}`);
  writeFileSync(tmp, data, "utf8");
  renameSync(tmp, path);
}

function main() {
  const payload = JSON.parse(readStdin() || "{}");
  const sessionId = payload.session_id;
  const eventName = payload.hook_event_name;
  if (!sessionId || !eventName) return;

  const dir = statusDir();
  mkdirSync(dir, { recursive: true });
  const path = statusPath(dir, sessionId);
  const previous = readPrevious(path);
  const needsChain = eventName === "SessionStart" || !previous?.pidChain?.length;
  const cwd = payload.cwd || previous?.cwd || process.cwd();

  const status = {
    sessionId,
    status: statusFor(eventName, payload),
    cwd,
    project: projectName(cwd),
    pidChain: needsChain ? pidChain() : previous.pidChain,
    updatedAt: Date.now(),
  };

  writeAtomic(dir, path, JSON.stringify(status));
}

try {
  main();
} catch {
  // Silencioso: ver o comentário do topo do arquivo.
}
