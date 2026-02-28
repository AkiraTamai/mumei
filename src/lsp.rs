//! # LSP モジュール
//!
//! `mumei lsp` コマンドの実装。
//! JSON-RPC over stdio で Language Server Protocol を提供する。
//!
//! ## 対応機能（Phase 1: 最小実装）
//! - `initialize` / `initialized` ハンドシェイク
//! - `textDocument/didOpen` / `textDocument/didChange` → パースして diagnostics 送信
//! - `shutdown` / `exit`
//!
//! ## 将来の拡張（Phase 2+）
//! - `textDocument/hover` — atom の requires/ensures 表示
//! - `textDocument/completion` — キーワード・atom 名補完
//! - `textDocument/publishDiagnostics` — Z3 検証エラーのリアルタイム表示
//! - `textDocument/definition` — 定義ジャンプ
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use crate::parser;
// =============================================================================
// メイン処理
// =============================================================================
/// `mumei lsp` のエントリポイント — stdio で JSON-RPC メッセージを処理
pub fn run() {
    eprintln!("mumei-lsp: starting (stdio mode)...");
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    // ファイル URI → ソースコード のキャッシュ
    let mut documents: HashMap<String, String> = HashMap::new();
    loop {
        // LSP メッセージを読み取り
        let message = match read_message(&mut reader) {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("mumei-lsp: read error: {}", e);
                break;
            }
        };
        // JSON パース
        let json: serde_json::Value = match serde_json::from_str(&message) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("mumei-lsp: JSON parse error: {}", e);
                continue;
            }
        };
        let method = json.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = json.get("id").cloned();
        match method {
            "initialize" => {
                let result = serde_json::json!({
                    "capabilities": {
                        "textDocumentSync": 1,
                        "hoverProvider": false,
                        "completionProvider": null
                    },
                    "serverInfo": {
                        "name": "mumei-lsp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                });
                if let Some(id) = id {
                    send_response(&mut writer, id, result);
                }
            }
            "initialized" => {
                eprintln!("mumei-lsp: initialized");
            }
            "textDocument/didOpen" => {
                if let Some(params) = json.get("params") {
                    if let Some(td) = params.get("textDocument") {
                        let uri = td.get("uri").and_then(|u| u.as_str()).unwrap_or("");
                        let text = td.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        documents.insert(uri.to_string(), text.to_string());
                        let diagnostics = diagnose(text);
                        send_diagnostics(&mut writer, uri, &diagnostics);
                    }
                }
            }
            "textDocument/didChange" => {
                if let Some(params) = json.get("params") {
                    if let Some(td) = params.get("textDocument") {
                        let uri = td.get("uri").and_then(|u| u.as_str()).unwrap_or("");
                        // contentChanges[0].text (full sync mode)
                        if let Some(changes) = params.get("contentChanges").and_then(|c| c.as_array()) {
                            if let Some(change) = changes.first() {
                                if let Some(text) = change.get("text").and_then(|t| t.as_str()) {
                                    documents.insert(uri.to_string(), text.to_string());
                                    let diagnostics = diagnose(text);
                                    send_diagnostics(&mut writer, uri, &diagnostics);
                                }
                            }
                        }
                    }
                }
            }
            "textDocument/didClose" => {
                if let Some(params) = json.get("params") {
                    if let Some(td) = params.get("textDocument") {
                        let uri = td.get("uri").and_then(|u| u.as_str()).unwrap_or("");
                        documents.remove(uri);
                        // diagnostics をクリア
                        send_diagnostics(&mut writer, uri, &[]);
                    }
                }
            }
            "shutdown" => {
                eprintln!("mumei-lsp: shutdown requested");
                if let Some(id) = id {
                    send_response(&mut writer, id, serde_json::Value::Null);
                }
            }
            "exit" => {
                eprintln!("mumei-lsp: exit");
                break;
            }
            _ => {
                // 未対応メソッド — リクエストなら MethodNotFound を返す
                if let Some(id) = id {
                    send_error(&mut writer, id, -32601, &format!("Method not found: {}", method));
                }
            }
        }
    }
}
// =============================================================================
// 診断（パースエラー検出）
// =============================================================================
/// ソースコードをパースして diagnostics を生成
fn diagnose(source: &str) -> Vec<serde_json::Value> {
    // parse_module は現在パニックせず空の Vec を返す設計なので、
    // パースが成功したかどうかを簡易的にチェックする
    let items = parser::parse_module(source);
    let mut diagnostics = Vec::new();
    // ソースが空でない場合にアイテムが0個 → パースエラーの可能性
    let trimmed = source.trim();
    if !trimmed.is_empty() && items.is_empty() && !trimmed.starts_with("//") {
        diagnostics.push(serde_json::json!({
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 1 }
            },
            "severity": 1,
            "source": "mumei",
            "message": "Parse error: no valid items found. Check syntax."
        }));
    }
    // 基本的な構文チェック: atom の数をカウントしてログ
    let atom_count = items.iter().filter(|i| matches!(i, parser::Item::Atom(_))).count();
    if atom_count > 0 {
        eprintln!("mumei-lsp: parsed {} atom(s) successfully", atom_count);
    }
    diagnostics
}
// =============================================================================
// LSP JSON-RPC I/O
// =============================================================================
/// LSP メッセージを stdin から読み取る（Content-Length ヘッダ付き）
fn read_message(reader: &mut impl BufRead) -> Result<String, String> {
    // ヘッダを読み取り
    let mut content_length: usize = 0;
    loop {
        let mut header_line = String::new();
        reader.read_line(&mut header_line)
            .map_err(|e| format!("Failed to read header: {}", e))?;
        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            break; // ヘッダ終了（空行）
        }
        if let Some(len_str) = trimmed.strip_prefix("Content-Length: ") {
            content_length = len_str.parse::<usize>()
                .map_err(|e| format!("Invalid Content-Length: {}", e))?;
        }
        // Content-Type 等は無視
    }
    if content_length == 0 {
        return Err("Content-Length is 0 or missing".to_string());
    }
    // ボディを読み取り
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)
        .map_err(|e| format!("Failed to read body: {}", e))?;
    String::from_utf8(body)
        .map_err(|e| format!("Invalid UTF-8 in body: {}", e))
}
/// JSON-RPC レスポンスを送信
fn send_response(writer: &mut impl Write, id: serde_json::Value, result: serde_json::Value) {
    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    });
    send_message(writer, &response);
}
/// JSON-RPC エラーレスポンスを送信
fn send_error(writer: &mut impl Write, id: serde_json::Value, code: i32, message: &str) {
    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    });
    send_message(writer, &response);
}
/// textDocument/publishDiagnostics 通知を送信
fn send_diagnostics(writer: &mut impl Write, uri: &str, diagnostics: &[serde_json::Value]) {
    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "diagnostics": diagnostics
        }
    });
    send_message(writer, &notification);
}
/// LSP メッセージを stdout に送信（Content-Length ヘッダ付き）
fn send_message(writer: &mut impl Write, message: &serde_json::Value) {
    let body = serde_json::to_string(message).unwrap_or_default();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let _ = writer.write_all(header.as_bytes());
    let _ = writer.write_all(body.as_bytes());
    let _ = writer.flush();
}
