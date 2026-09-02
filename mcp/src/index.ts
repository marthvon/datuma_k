#!/usr/bin/env node
import { serveStdio } from "@modelcontextprotocol/server/stdio";
import { createDatumaServer } from "./server.js";

serveStdio(() => createDatumaServer());
