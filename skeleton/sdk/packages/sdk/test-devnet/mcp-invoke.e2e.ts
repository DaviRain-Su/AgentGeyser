import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { Client as McpClient } from '@modelcontextprotocol/sdk/client/index.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';
import { createKeyPairSignerFromBytes, createSolanaRpc } from '@solana/web3.js';
import { afterAll, expect, test } from 'vitest';
import { signAndSend, type Connection } from '../src/index.js';

const EVIDENCE = process.env.AGENTGEYSER_MCP_EVIDENCE ?? '/tmp/m5c-evidence/f15-mcp-evidence.json';

function env(name: string, fallback?: string): string {
  const value = process.env[name] ?? fallback;
  if (!value) throw new Error(`missing required env: ${name}`);
  return value;
}

function loadKeypairBytes(path: string): Uint8Array {
  const parsed = JSON.parse(readFileSync(path, 'utf8')) as unknown;
  if (!Array.isArray(parsed) || parsed.length !== 64) {
    throw new Error(`expected 64-byte keypair array at ${path}`);
  }
  return Uint8Array.from(parsed as number[]);
}

function textContent(result: unknown): string {
  const content = (result as { content?: Array<{ type: string; text?: string }> }).content;
  const text = content?.find((item) => item.type === 'text')?.text;
  if (!text) throw new Error(`missing MCP text content: ${JSON.stringify(result)}`);
  return text;
}

let client: McpClient | undefined;

afterAll(async () => {
  await client?.close();
});

test('MCP invoke_skill builds, signs, submits, and confirms a surfpool tx', async () => {
  const mcpUrl = env('AGENTGEYSER_MCP_URL', 'http://127.0.0.1:9099/mcp');
  const rpcUrl = env('AGENTGEYSER_RPC_URL', 'http://127.0.0.1:8899');
  const srcOwner = env('AGENTGEYSER_DEVNET_SRC_OWNER');
  const srcAta = env('AGENTGEYSER_DEVNET_SRC_ATA');
  const dstAta = env('AGENTGEYSER_DEVNET_DST_ATA');
  const mint = env('AGENTGEYSER_DEVNET_MINT');
  const amount = Number(env('AGENTGEYSER_DEVNET_AMOUNT', '10000'));
  const keypairPath = env('AGENTGEYSER_DEVNET_KEYPAIR');

  client = new McpClient({ name: 'm5c-f15', version: '0.0.1' }, { capabilities: {} });
  await client.connect(new StreamableHTTPClientTransport(new URL(mcpUrl)));

  // Evidence datapoint 1: MCP tools/list exposes both AgentGeyser tools.
  const tools = await client.listTools();
  const toolNames = tools.tools.map((tool) => tool.name);
  expect(toolNames).toContain('list_skills');
  expect(toolNames).toContain('invoke_skill');

  const invokeResp = await client.callTool({
    name: 'invoke_skill',
    arguments: {
      chain: 'devnet',
      skill_id: 'spl-token::transfer',
      args: {
        source_ata: srcAta,
        destination_ata: dstAta,
        owner: srcOwner,
        amount,
        mint,
        decimals: 6,
      },
      accounts: {},
      payer: srcOwner,
    },
  });
  expect((invokeResp as { isError?: boolean }).isError).not.toBe(true);
  const parsed = JSON.parse(textContent(invokeResp)) as { transaction_base64?: unknown };
  expect(typeof parsed.transaction_base64).toBe('string');
  const transactionBase64 = parsed.transaction_base64 as string;
  expect(transactionBase64.length).toBeGreaterThanOrEqual(64);
  expect(transactionBase64).toMatch(/^[A-Za-z0-9+/=]+$/);

  const signer = await createKeyPairSignerFromBytes(loadKeypairBytes(keypairPath));
  expect(signer.address).toBe(srcOwner);
  const rpc = createSolanaRpc(rpcUrl);
  const connection: Connection = {
    async sendTransaction(wireB64: string): Promise<string> {
      return rpc
        .sendTransaction(wireB64 as unknown as Parameters<typeof rpc.sendTransaction>[0], {
          encoding: 'base64',
          skipPreflight: false,
          preflightCommitment: 'confirmed',
        })
        .send() as Promise<string>;
    },
  };

  const { signature } = await signAndSend({
    unsignedTx: { tx: transactionBase64 },
    signer,
    connection,
  });
  expect(signature).toMatch(/^[1-9A-HJ-NP-Za-km-z]{87,88}$/);
  const confirm = execFileSync('solana', ['confirm', signature, '--url', rpcUrl], {
    encoding: 'utf8',
    timeout: 60_000,
  });
  expect(confirm).toMatch(/Confirmed|Finalized/);

  const evidence = {
    tools_list_result: toolNames,
    invoke_skill_transaction_base64_len: transactionBase64.length,
    confirmed_signature: signature,
    timestamp_utc: new Date().toISOString(),
    tools_list: toolNames,
    invoke_skill: {
      transaction_base64: transactionBase64,
      signature,
    },
  };
  mkdirSync('/tmp/m5c-evidence', { recursive: true });
  writeFileSync(EVIDENCE, `${JSON.stringify(evidence, null, 2)}\n`);
}, 120_000);
