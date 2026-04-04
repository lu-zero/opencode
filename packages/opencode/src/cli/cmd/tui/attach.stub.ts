import { cmd } from "@/cli/cmd/cmd"

export const AttachCommand = cmd({
  command: "attach <url>",
  describe: "attach to a running opencode server (unavailable on this platform)",
  builder: (yargs: any) => yargs,
  handler: async () => {
    process.stderr.write("TUI is not available on this platform\n")
    process.exit(1)
  },
})
