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

/**
 * The entry point to this VS Code extension.
 *
 * As per [the VS Code documentation on activation
 * events](https://code.visualstudio.com/api/references/activation-events), 'an extension must
 * export an `activate()` function from its main module and it will be invoked only once by
 * VS Code when any of the specified activation events [are] emitted."
 *
 * Activation events for this extension are listed in its `package.json` file, under the key
 * `"activationEvents"`.
 *
 * In order to achieve synchronous activation, mark the function as an asynchronous function,
 * so that you can wait for the activation to complete by await
 */


// class TerminalManager {
//   all: Map<string, vscode.Terminal | undefined>;

//   constructor() {
//     this.all = new Map();
//   }

//   alloc(typ: string, new_fun: () => vscode.Terminal): vscode.Terminal {
//     const x = this.all.get(typ);
//     if (x === undefined || x.exitStatus !== undefined) {
//       const x = new_fun();
//       this.all.set(typ, x);
//       return x;
//     }
//     return x;
//   }
// }

// const terminalManager = new TerminalManager();


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

  const tokenTypes = ['struct', 'function', 'variable',
    'keyword', 'string', 'operator', 'enumMember', 'type', 'number'];
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

  // Configure other language features.
  context.configureLanguage();

  // All other utilities provided by this extension occur via the language server.
  await context.startClient();
}
