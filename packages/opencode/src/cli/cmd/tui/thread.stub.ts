import { cmd } from "@/cli/cmd/cmd"

export const TuiThreadCommand = cmd({
  command: "$0 [project]",
  describe: "start opencode tui (unavailable on this platform)",
  builder: (yargs: any) => yargs,
  handler: async () => {
    process.stderr.write("TUI is not available on this platform\n")
    process.exit(1)
  },
})
