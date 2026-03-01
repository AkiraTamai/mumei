import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';
let client: LanguageClient | undefined;
export function activate(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('mumei');
    const serverPath = config.get<string>('serverPath', 'mumei');
    const serverOptions: ServerOptions = {
        command: serverPath,
        args: ['lsp'],
    };
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'mumei' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.mm'),
        },
    };
    client = new LanguageClient(
        'mumei-lsp',
        'Mumei Language Server',
        serverOptions,
        clientOptions
    );
    client.start();
}
export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
