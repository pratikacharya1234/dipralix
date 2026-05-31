// Dipralix VS Code extension.
//
// Provides four commands:
//   - dipralix.openTerminal       → opens a VS Code terminal running `dipralix-cli`
//   - dipralix.runTaskAt          → sends the `// DIPRALIX: …` task on the cursor line
//                                    as a one-shot `dipralix-cli --prompt "<task>"`
//   - dipralix.runFingerprint     → runs `dipralix-cli --fingerprint`
//   - dipralix.init               → runs `dipralix-cli --init` in the workspace
//
// Plus a CodeLens provider that shows "▶ Run with Dipralix" above every
// `// DIPRALIX:` (or `# DIPRALIX:` / `/* DIPRALIX: */`) comment in the active editor.

import * as vscode from 'vscode';

function binPath(): string {
  return vscode.workspace.getConfiguration('dipralix').get<string>('binaryPath') || 'dipralix-cli';
}

function shellQuote(s: string): string {
  // VS Code Terminal.sendText handles its own shell, but for the --prompt payload
  // we need to escape internal double quotes.
  return s.replace(/"/g, '\\"');
}

function getOrCreateTerminal(name = 'Dipralix'): vscode.Terminal {
  const existing = vscode.window.terminals.find((t) => t.name === name);
  if (existing) return existing;
  return vscode.window.createTerminal({ name });
}

function dipralixRegex(): RegExp {
  // Match the language-agnostic DIPRALIX: directive. Excludes already-done items.
  // eslint-disable-next-line no-useless-escape
  return /(?:\/\/|#|\/\*|--)\s*DIPRALIX:\s*(.+?)(?:\s*\*\/)?\s*$/;
}

class DipralixCodeLensProvider implements vscode.CodeLensProvider {
  private _onDidChange = new vscode.EventEmitter<void>();
  readonly onDidChangeCodeLenses = this._onDidChange.event;

  refresh(): void {
    this._onDidChange.fire();
  }

  provideCodeLenses(document: vscode.TextDocument): vscode.ProviderResult<vscode.CodeLens[]> {
    const lenses: vscode.CodeLens[] = [];
    const re = dipralixRegex();
    const lineCount = document.lineCount;
    for (let i = 0; i < lineCount; i++) {
      const line = document.lineAt(i);
      // skip DIPRALIX-DONE markers
      if (line.text.includes('DIPRALIX-DONE:')) continue;
      const m = re.exec(line.text);
      if (m) {
        const description = m[1].trim();
        const range = new vscode.Range(i, 0, i, line.text.length);
        lenses.push(
          new vscode.CodeLens(range, {
            title: '▶ Run with Dipralix',
            tooltip: `Send to dipralix-cli --prompt: ${description}`,
            command: 'dipralix.runTaskAt',
            arguments: [document.uri, i],
          }),
        );
      }
    }
    return lenses;
  }
}

export function activate(context: vscode.ExtensionContext) {
  const codeLens = new DipralixCodeLensProvider();
  context.subscriptions.push(
    vscode.languages.registerCodeLensProvider({ scheme: 'file' }, codeLens),
  );

  // Re-emit lens events when documents change.
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument(() => codeLens.refresh()),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('dipralix.openTerminal', () => {
      const term = getOrCreateTerminal();
      term.show();
      term.sendText(binPath());
    }),

    vscode.commands.registerCommand('dipralix.runTaskAt', async (uri?: vscode.Uri, lineNo?: number) => {
      const editor = vscode.window.activeTextEditor;
      const doc = uri ? await vscode.workspace.openTextDocument(uri) : editor?.document;
      if (!doc) return;

      const line = lineNo ?? editor?.selection.active.line ?? -1;
      if (line < 0 || line >= doc.lineCount) return;

      const m = dipralixRegex().exec(doc.lineAt(line).text);
      if (!m) {
        vscode.window.showWarningMessage('No DIPRALIX: directive on this line.');
        return;
      }
      const description = m[1].trim();
      const filePath = vscode.workspace.asRelativePath(doc.uri);
      const prompt = `Source file \`${filePath}\` line ${line + 1} requests: ${description}\n\nImplement this change. When done, replace the DIPRALIX: marker on that line with DIPRALIX-DONE:.`;

      const term = getOrCreateTerminal();
      term.show();
      term.sendText(`${binPath()} --prompt "${shellQuote(prompt)}"`);
    }),

    vscode.commands.registerCommand('dipralix.runFingerprint', () => {
      const term = getOrCreateTerminal();
      term.show();
      term.sendText(`${binPath()} --fingerprint`);
    }),

    vscode.commands.registerCommand('dipralix.init', () => {
      const term = getOrCreateTerminal();
      term.show();
      term.sendText(`${binPath()} --init`);
    }),
  );
}

export function deactivate() {
  // nothing to clean up — terminals are owned by VS Code.
}
