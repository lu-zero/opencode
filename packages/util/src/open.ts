import { spawn } from "node:child_process"
import open from "open"

interface BrowserCommand {
  command: string
  args: string[]
}

/**
 * `$BROWSER` candidates for a URL, in order, or an empty list to use the
 * platform opener. An empty list also covers Windows, where the variable is
 * a Unix convention and the system opener already works.
 *
 * The field codes match the Rust `webbrowser` crate: colon-separated
 * candidates, whitespace-separated arguments, and `%s`, `%c` and `%%`.
 * The URL is appended to a candidate with no `%s`.
 */
export function browserCommands(
  url: string,
  env: Record<string, string | undefined> = process.env,
  platform: NodeJS.Platform = process.platform,
): BrowserCommand[] {
  if (platform === "win32") return []
  const value = env.BROWSER?.trim()
  if (!value) return []
  const commands: BrowserCommand[] = []
  for (const entry of value.split(":")) {
    const parts = entry.trim().split(/\s+/).filter(Boolean)
    if (parts.length === 0) continue
    let placed = false
    const expanded = parts.map(part => {
      const result = expandFieldCodes(part, url)
      placed ||= result.placed
      return result.value
    })
    if (!placed) expanded.push(url)
    commands.push({ command: expanded[0], args: expanded.slice(1) })
  }
  return commands
}

/** One left-to-right pass so a `%` inside the URL is never rescanned. */
function expandFieldCodes(part: string, url: string): { value: string; placed: boolean } {
  let value = ""
  let placed = false
  for (let i = 0; i < part.length; i++) {
    if (part[i] !== "%") {
      value += part[i]
      continue
    }
    const next = part[++i]
    if (next === "s") {
      value += url
      placed = true
    } else if (next === "c") value += ":"
    else if (next === "%") value += "%"
    else if (next === undefined) value += "%"
    else value += "%" + next
  }
  return { value, placed }
}

/** Resolve once the process actually started; reject if it could not. */
function spawnDetached(command: string, args: string[]) {
  return new Promise<void>((resolve, reject) => {
    const child = spawn(command, args, { stdio: "ignore", detached: true })
    child.once("spawn", () => {
      child.unref()
      resolve()
    })
    child.once("error", reject)
  })
}

export async function openUrl(input: string) {
  const url = URL.canParse(input) ? new URL(input) : undefined
  if (!url || (url.protocol !== "http:" && url.protocol !== "https:"))
    throw new Error(`Only http and https links can be opened in the browser: ${input}`)

  for (const candidate of browserCommands(url.href)) {
    try {
      return await spawnDetached(candidate.command, candidate.args)
    } catch {
      // Try the next BROWSER candidate; a missing first choice must not win.
    }
  }
  return open(url.href)
}

export function openPath(path: string) {
  return open(path)
}
