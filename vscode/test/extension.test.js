'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const Module = require('node:module');

test('activation starts the configured executable and deactivation stops it', async () => {
  const original = Module._load;
  const events = [];
  const subscriptions = [];
  class LanguageClient {
    constructor(id, name, server, options) {
      assert.equal(id, 'ratslang');
      assert.equal(server.command, '/tmp/custom-ratslang-lsp');
      assert.deepEqual(options.documentSelector.map(d => d.language), ['rl', 'rl']);
      assert.ok(options.synchronize.fileEvents);
    }
    async start() { events.push('start'); }
    async stop() { events.push('stop'); }
  }
  Module._load = function (name, ...rest) {
    if (name === 'vscode') return {
      workspace: {
        getConfiguration: () => ({get: () => '/tmp/custom-ratslang-lsp'}),
        createFileSystemWatcher: () => ({dispose() {}}),
      },
      window: {createOutputChannel: () => ({dispose() {}})},
    };
    if (name === 'vscode-languageclient/node') return {LanguageClient};
    return original.call(this, name, ...rest);
  };
  try {
    const extension = require('../extension');
    await extension.activate({subscriptions});
    await extension.deactivate();
    assert.deepEqual(events, ['start', 'stop']);
    assert.equal(subscriptions.length, 3);
  } finally { Module._load = original; }
});

test('extension manifest activates the language client', () => {
  const manifest = require('../package.json');
  assert.equal(manifest.main, './extension.js');
  assert.ok(manifest.activationEvents.includes('onLanguage:rl'));
  assert.ok(manifest.dependencies['vscode-languageclient']);
  assert.equal(manifest.contributes.configuration.properties['ratslang.serverPath'].default, 'ratslang-lsp');
});
