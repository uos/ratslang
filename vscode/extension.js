'use strict';

const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');

let client;

async function activate(context) {
  const command = vscode.workspace.getConfiguration('ratslang').get('serverPath', 'ratslang-lsp');
  const watcher = vscode.workspace.createFileSystemWatcher('**/*.rl');
  const output = vscode.window.createOutputChannel('Ratslang');
  context.subscriptions.push(watcher, output);
  client = new LanguageClient('ratslang', 'Ratslang', {
    command,
    args: [],
    options: { env: { ...process.env } },
  }, {
    documentSelector: [{ scheme: 'file', language: 'rl' }, { scheme: 'untitled', language: 'rl' }],
    synchronize: { fileEvents: watcher },
    outputChannel: output,
  });
  context.subscriptions.push(client);
  await client.start();
}

async function deactivate() {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

module.exports = { activate, deactivate };
