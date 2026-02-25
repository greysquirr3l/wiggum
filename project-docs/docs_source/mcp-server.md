# MCP Server

Wiggum can run as an [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) server, allowing AI agents to invoke Wiggum's capabilities mid-session.

## Starting the server

```bash
wiggum serve --mcp
```

This starts the MCP server using stdio transport.

## Integration

To use Wiggum as an MCP server with your AI coding tool, add it to your MCP configuration. For example, in VS Code's MCP settings:

```json
{
  "servers": {
    "wiggum": {
      "command": "wiggum",
      "args": ["serve", "--mcp"]
    }
  }
}
```

This enables agents to generate plans and task scaffolds directly within a coding session, without leaving the editor.
