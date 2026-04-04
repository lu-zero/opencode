import BunShim from "bun:install"

globalThis.Bun = BunShim as any

const wrap = (orig: typeof globalThis.setInterval) =>
  ((handler: any, ms?: number, ...args: any[]) => {
    const id = orig(handler, ms, ...args)
    return { ref() { return this }, unref() { return this }, [Symbol.toPrimitive]() { return id } }
  }) as any

globalThis.setTimeout = wrap(globalThis.setTimeout)
globalThis.setInterval = wrap(globalThis.setInterval)

await import("./index.deno.ts")
