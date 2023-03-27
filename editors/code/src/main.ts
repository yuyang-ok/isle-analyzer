// Copyright (c) The Diem Core Contributors
// Copyright (c) The ISLE Contributors
// SPDX-License-Identifier: Apache-2.0

import { Configuration } from './configuration';
import { Context } from './context';
import { Extension } from './extension';
import { log } from './log';
import * as fs from 'fs';
import * as path from 'path';

import * as childProcess from 'child_process';
import * as vscode from 'vscode';
class TraverseDirItem {
  path: string;

  is_file: boolean;

  constructor(path: string,
    is_file: boolean) {
    this.path = path;
    this.is_file = is_file;
  }
}

function traverseDir(dir: any, call_back: (path: TraverseDirItem) => void): void {
  fs.readdirSync(dir).forEach(file => {
    const fullPath = path.join(dir, file);
    if (fs.lstatSync(fullPath).isDirectory()) {
      call_back(new TraverseDirItem(fullPath, false));
      traverseDir(fullPath, call_back);
    } else {
      call_back(new TraverseDirItem(fullPath, true));
    }
  });
}

function workSpaceDir(): string | undefined {
  if (vscode.workspace.workspaceFolders !== undefined) {
    if (vscode.workspace.workspaceFolders[0] !== undefined) {
      const f = vscode.workspace.workspaceFolders[0].uri.fsPath;
      return f;
    }
  }
  return undefined;
}


/**
 * An extension command that displays the version of the server that this extension
 * interfaces with.
 */
async function serverVersion(context: Readonly<Context>): Promise<void> {
  const version = childProcess.spawnSync(
    context.configuration.serverPath,
    ['--version'],
    { encoding: 'utf8' },
  );
  if (version.stdout) {
    await vscode.window.showInformationMessage(version.stdout);
  } else if (version.error) {
    await vscode.window.showErrorMessage(
      `Could not execute isle-analyzer: ${version.error.message}.`,
    );
  } else {
    await vscode.window.showErrorMessage(
      `A problem occurred when executing '${context.configuration.serverPath}'.`,
    );
  }
}


async function reload(context: Context): Promise<void> {
  const isle_files = new Array<string>();
  traverseDir(workSpaceDir(), (e) => {
    if (e.is_file && e.path.endsWith('.isle')) {
      isle_files.push(e.path);
    }
  });
  const isle_files_pick_items = new Array<vscode.QuickPickItem>();
  isle_files.forEach((e) => {
    isle_files_pick_items.push({ label: e, picked: true });
  });
  const isle_picked = await vscode.window.showQuickPick(isle_files_pick_items, {
    canPickMany: true,
    title: "Select ISLE file you want to include Project."
  });
  const isle_files2 = new Array<string>();
  isle_picked?.forEach((e) => {
    isle_files2.push(e.label);
  });
  const client = context.getClient();
  if (client !== undefined) {
    void client.sendRequest('isle/reload', { 'files': isle_files2 });
  }
}


export async function activate(
  extensionContext: Readonly<vscode.ExtensionContext>,
): Promise<void> {
  const extension = new Extension();
  log.info(`${extension.identifier} version ${extension.version}`);

  const configuration = new Configuration();
  log.info(`configuration: ${configuration.toString()}`);

  const context = Context.create(extensionContext, configuration);
  // An error here -- for example, if the path to the `isle-analyzer` binary that the user
  // specified in their settings is not valid -- prevents the extension from providing any
  // more utility, so return early.
  if (context instanceof Error) {
    void vscode.window.showErrorMessage(
      `Could not activate isle-analyzer: ${context.message}.`,
    );
    return;
  }

  const tokenTypes = ['struct',
    'function',
    'variable',
    'keyword',
    'string',
    'operator',
    'enumMember',
    'type',
    'number'];
  const tokenModifiers = ['declaration'];
  const legend = new vscode.SemanticTokensLegend(tokenTypes, tokenModifiers);
  const provider: vscode.DocumentSemanticTokensProvider = {
    provideDocumentSemanticTokens(
      document: vscode.TextDocument,
    ): vscode.ProviderResult<vscode.SemanticTokens> {
      // Analyze the document and return semantic tokens
      const client = context.getClient();
      if (client === undefined) {
        return undefined;
      }
      return client.sendRequest<vscode.SemanticTokens>('textDocument/semanticTokens/full',
        { 'textDocument': { uri: document.uri.toString() } });

    },
  };
  vscode.languages.registerDocumentSemanticTokensProvider({ language: 'isle', scheme: 'file' },
    provider,
    legend);

  // Register handlers for VS Code commands that the user explicitly issues.
  context.registerCommand('serverVersion', serverVersion);

  context.registerCommand('goto_definition', async (_context, ...args) => {
    const loc = args[0] as { range: vscode.Range; fpath: string };
    const t = await vscode.workspace.openTextDocument(loc.fpath);
    await vscode.window.showTextDocument(t, { selection: loc.range, preserveFocus: false });
  });

  const d = vscode.languages.registerInlayHintsProvider({ scheme: 'file', language: 'isle' },
    {
      provideInlayHints(document, range) {
        const client = context.getClient();
        if (client === undefined) {
          return undefined;
        }
        const hints = client.sendRequest<vscode.InlayHint[]>('textDocument/inlayHint',
          { range: range, textDocument: { uri: document.uri.toString() } });
        return hints;
      },
    });
  extensionContext.subscriptions.push(d);

  // Configure other language features.
  context.configureLanguage();

  // All other utilities provided by this extension occur via the language server.
  await context.startClient();

  void reload(context);
}

