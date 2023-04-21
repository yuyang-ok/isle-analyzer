// Copyright (c) The Diem Core Contributors
// Copyright (c) The ISLE Contributors
// SPDX-License-Identifier: Apache-2.0

import { Configuration } from './configuration';
import { Context } from './context';
import { Extension } from './extension';
import { log } from './log';

import * as childProcess from 'child_process';
import * as vscode from 'vscode';

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


function reload(context: Readonly<Context>) {
  const isle_files = context.configuration.isleFiles();
  const client = context.getClient();
  if (client !== undefined) {
    client.sendRequest('isle/reload', { 'files': isle_files }).catch((e) => {
      void vscode.window.showErrorMessage('load project failed:' + (e as string));
    });
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
  context.registerCommand('reload', reload);

  context.registerCommand('goto_definition', async (_context, ...args) => {
    const loc = args[0] as { range: vscode.Range; fpath: string };
    const t = await vscode.workspace.openTextDocument(loc.fpath);
    await vscode.window.showTextDocument(t, { selection: loc.range, preserveFocus: false });
  });
  context.registerCommand('isle.show.compiled.code', () => {
    const client = context.getClient();
    if (client === undefined) {
      return;
    }
    const d = vscode.window.activeTextEditor;
    if (d === undefined) {
      return;
    }

    const fpath = d.document.uri.path;
    const line = d.selection.active.line;
    const col = d.selection.active.character;
    client.sendRequest<{
      range: vscode.Range;
      result: string;
    }>('isle/show_compiled_code', { 'fpath': fpath, 'line': line, 'col': col }).then((r) => {
      void vscode.workspace.openTextDocument({ language: 'rust', content: r.result }).then((e) => {
        void vscode.window.showTextDocument(e, { selection: r.range });
      });
    }).catch((e) => {
      void vscode.window.showErrorMessage('get compiled code failed:' + (e as string));
    });

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

  reload(context);


}

