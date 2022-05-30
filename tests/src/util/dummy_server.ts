import * as express from "express"
import { Server } from "http"
import { DropHandler } from "./environment"
import { assignPort } from "./ports"

export class DummyServer implements DropHandler {
  servers: Array<Server> = []

  serve(): Promise<number> {
    const app = express()

    app.get('/', (req, res) => {
      res.send('Hello World!')
    })

    app.get('/host', (req, res) => {
      res.send(req.hostname)
    })

    const port = assignPort()

    return new Promise((accept, reject) => {
      this.servers.push(app.listen(port, () => {
        accept(port)
      }))
    })
  }

  async drop() {
    for (const server of this.servers) {
      await new Promise((accept, reject) => server.close(accept))
    }
  }
}