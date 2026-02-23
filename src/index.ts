#!/usr/bin/env node

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { z } from 'zod';
import { Warp10Client } from './warp10-client.js';

// Configuration schema
const ConfigSchema = z.object({
  endpoint: z.string().url().describe('Warp10 endpoint URL'),
  token: z.string().optional().describe('Warp10 read/write token'),
  readToken: z.string().optional().describe('Warp10 read token'),
  writeToken: z.string().optional().describe('Warp10 write token')
});

type Config = z.infer<typeof ConfigSchema>;

// Tool input schemas
const FetchSchema = z.object({
  selector: z.string(),
  labels: z.record(z.string()).optional(),
  start: z.union([z.string(), z.number()]).optional(),
  end: z.union([z.string(), z.number()]).optional(),
  count: z.number().optional()
});

const UpdateSchema = z.object({
  className: z.string(),
  labels: z.record(z.string()).optional(),
  dataPoints: z.array(z.object({
    timestamp: z.number(),
    value: z.any(),  // Required field, put it before optional fields
    latitude: z.number().optional(),
    longitude: z.number().optional(),
    elevation: z.number().optional()
  }))
});

const DeleteSchema = z.object({
  selector: z.string(),
  labels: z.record(z.string()).optional(),
  start: z.union([z.string(), z.number()]).optional(),
  end: z.union([z.string(), z.number()]).optional()
});

const ExecSchema = z.object({
  script: z.string()
});

const FindSchema = z.object({
  selector: z.string(),
  labels: z.record(z.string()).optional()
});

const MetaSchema = z.object({
  selector: z.string(),
  labels: z.record(z.string()).optional(),
  attributes: z.record(z.string()).optional()
});

const LabelValuesSchema = z.object({
  selector: z.string(),
  labelName: z.string(),
  otherLabels: z.record(z.string()).optional()
});

const PredictSchema = z.object({
  selector: z.string(),
  labels: z.record(z.string()).optional(),
  method: z.enum(['holt_winters', 'arima', 'ses', 'linear_regression', 'polynomial', 'fft_forecast', 'lstm', 'auto']),
  historyPeriod: z.string(),
  predictionHorizon: z.number(),
  bucketSpan: z.string().optional(),
  options: z.object({
    alpha: z.number().optional(),
    beta: z.number().optional(),
    gamma: z.number().optional(),
    seasonality: z.number().optional(),
    p: z.number().optional(),
    d: z.number().optional(),
    q: z.number().optional(),
    degree: z.number().optional(),
    epochs: z.number().optional(),
    hiddenUnits: z.number().optional()
  }).optional(),
  includeConfidence: z.boolean().optional(),
  confidenceLevel: z.number().optional()
});

async function main() {
  const config: Config = {
    endpoint: process.env.WARP10_ENDPOINT || 'http://localhost:8080/api/v0',
    token: process.env.WARP10_TOKEN,
    readToken: process.env.WARP10_READ_TOKEN,
    writeToken: process.env.WARP10_WRITE_TOKEN
  };

  const warp10Client = new Warp10Client(config);

  const server = new McpServer(
    {
      name: 'mcp-warp10',
      version: '0.1.0',
    },
    {
      capabilities: {
        tools: {},
        resources: {}
      },
    }
  );

  // Register tools
  server.registerTool(
    'warp10_fetch',
    {
      description: 'Fetch time series data from Warp10',
      inputSchema: FetchSchema
    },
    async (args: z.infer<typeof FetchSchema>) => {
      const result = await warp10Client.fetch(args);
      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(result, null, 2)
          }
        ]
      };
    }
  );

  server.registerTool(
    'warp10_update',
    {
      description: 'Insert or update data points in Warp10',
      inputSchema: UpdateSchema
    },
    async (args: z.infer<typeof UpdateSchema>) => {
      const result = await warp10Client.update(args as any);
      return {
        content: [
          {
            type: 'text',
            text: `Data points updated successfully. Response: ${result}`
          }
        ]
      };
    }
  );

  server.registerTool(
    'warp10_delete',
    {
      description: 'Delete time series data from Warp10',
      inputSchema: DeleteSchema
    },
    async (args: z.infer<typeof DeleteSchema>) => {
      const result = await warp10Client.delete(args);
      return {
        content: [
          {
            type: 'text',
            text: `Data deleted successfully. Response: ${result}`
          }
        ]
      };
    }
  );

  server.registerTool(
    'warp10_exec',
    {
      description: 'Execute WarpScript code on the Warp10 server',
      inputSchema: ExecSchema
    },
    async (args: z.infer<typeof ExecSchema>) => {
      const result = await warp10Client.exec(args.script);
      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(result, null, 2)
          }
        ]
      };
    }
  );

  server.registerTool(
    'warp10_find',
    {
      description: 'Find time series matching specific criteria',
      inputSchema: FindSchema
    },
    async (args: z.infer<typeof FindSchema>) => {
      const result = await warp10Client.find(args);
      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(result, null, 2)
          }
        ]
      };
    }
  );

  server.registerTool(
    'warp10_meta',
    {
      description: 'Get or set metadata/attributes for time series',
      inputSchema: MetaSchema
    },
    async (args: z.infer<typeof MetaSchema>) => {
      const result = await warp10Client.meta(args);
      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(result, null, 2)
          }
        ]
      };
    }
  );

  server.registerTool(
    'warp10_label_values',
    {
      description: 'Get all unique values for a specific label across time series',
      inputSchema: LabelValuesSchema
    },
    async (args: z.infer<typeof LabelValuesSchema>) => {
      const result = await warp10Client.getLabelValues(args);
      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(result, null, 2)
          }
        ]
      };
    }
  );

  server.registerTool(
    'warp10_predict',
    {
      description: 'Predict future values using various algorithms (HOLT-WINTERS, ARIMA, SES, etc.)',
      inputSchema: PredictSchema
    },
    async (args: z.infer<typeof PredictSchema>) => {
      const result = await warp10Client.predict(args);
      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(result, null, 2)
          }
        ]
      };
    }
  );

  // Register resources
  server.registerResource(
    'Warp10 MCP Documentation',
    'warp10://documentation',
    {
      description: 'Complete documentation for using the Warp10 MCP server',
      mimeType: 'text/markdown'
    },
    async () => {
      return {
        contents: [
          {
            uri: 'warp10://documentation',
            mimeType: 'text/markdown',
            text: getDocumentation()
          }
        ]
      };
    }
  );

  server.registerResource(
    'WarpScript Quick Reference',
    'warp10://warpscript-reference',
    {
      description: 'Quick reference guide for WarpScript language and common operations',
      mimeType: 'text/markdown'
    },
    async () => {
      return {
        contents: [
          {
            uri: 'warp10://warpscript-reference',
            mimeType: 'text/markdown',
            text: getWarpScriptReference()
          }
        ]
      };
    }
  );

  // Connect to transport
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error('Warp10 MCP server running');
}

function getDocumentation(): string {
  return `# Warp10 MCP Server Documentation

## Overview
This MCP server provides tools to interact with a Warp10 time series database.

## Available Tools

### warp10_fetch
Fetch time series data from Warp10.

### warp10_update
Update/insert data points in Warp10.

### warp10_delete
Delete time series data from Warp10.

### warp10_exec
Execute WarpScript code on the Warp10 server.

### warp10_find
Find time series matching specific criteria.

### warp10_meta
Get or set metadata for time series.

### warp10_label_values
Get all unique values for a specific label.

### warp10_predict
Predict future values using various algorithms.

## Configuration
Set the following environment variables:
- WARP10_ENDPOINT: The Warp10 server endpoint
- WARP10_TOKEN: Read/write token (or use specific tokens below)
- WARP10_READ_TOKEN: Read-only token
- WARP10_WRITE_TOKEN: Write-only token
`;
}

function getWarpScriptReference(): string {
  return `# WarpScript Quick Reference

## Stack Operations
- DUP: Duplicate top of stack
- DROP: Remove top of stack
- SWAP: Swap two top elements
- ROT: Rotate top 3 elements

## Data Types
- 'string': String literal
- 42: Integer
- 3.14: Float
- true/false: Boolean
- [ ]: List
- { }: Map

## Time Series Operations
- FETCH: Retrieve data
- UPDATE: Store data
- DELETE: Remove data
- FIND: Search series
- META: Handle metadata

## Mathematical Functions
- +, -, *, /: Basic operations
- ABS, ROUND, FLOOR, CEIL: Rounding
- SIN, COS, TAN: Trigonometry
- LOG, EXP, SQRT: Advanced math

## Control Flow
- IFT/IFTE: Conditionals
- FOR/FOREACH: Loops
- STOP/CONTINUE/BREAK: Flow control

## Example:
\`\`\`warpscript
// Fetch last hour of data
[ 'token' 'class.name' {} NOW 1 h ] FETCH
\`\`\`
`;
}

main().catch(console.error);