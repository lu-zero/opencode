import { DatabaseSync } from "node:sqlite"

type Stmt = {
  all(...params: unknown[]): Record<string, unknown>[]
}

export class Database {
  private db: DatabaseSync

  constructor(path: string, opts?: { readonly?: boolean }) {
    this.db = new DatabaseSync(path, opts)
  }

  exec(sql: string) {
    this.db.exec(sql)
  }

  query(sql: string): Stmt {
    const stmt = this.db.prepare(sql)
    return {
      all(...params: unknown[]) {
        return stmt.all(...params) as Record<string, unknown>[]
      },
    }
  }

  close() {
    this.db.close()
  }
}
