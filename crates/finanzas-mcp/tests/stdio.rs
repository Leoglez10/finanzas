use std::{io::{BufRead, BufReader, Write}, process::{Command, Stdio}, time::{SystemTime, UNIX_EPOCH}};
use serde_json::{json, Value};

#[test]
fn stdio_initializes_lists_and_calls_tools() {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let db = std::env::temp_dir().join(format!("finanzas-mcp-{}-{unique}.db", std::process::id()));
    let mut child = Command::new(env!("CARGO_BIN_EXE_finanzas-mcp"))
        .env("FINANZAS_DB", db).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    fn send(input: &mut impl Write, value: Value) { writeln!(input, "{value}").unwrap(); input.flush().unwrap(); }
    fn recv(output: &mut impl BufRead) -> Value { let mut line = String::new(); assert!(output.read_line(&mut line).unwrap() > 0, "server closed stdout"); serde_json::from_str(&line).unwrap() }
    send(&mut input,json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}));
    let init=recv(&mut output); assert_eq!(init["result"]["serverInfo"]["name"],"finanzas"); assert_eq!(init["result"]["serverInfo"]["version"],"1.0.0");
    send(&mut input,json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    send(&mut input,json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}));
    let list=recv(&mut output); let tools=list["result"]["tools"].as_array().unwrap();
    let names: Vec<_> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names,["list_accounts","get_transactions","list_categories","get_budget_month","spending_by_category","add_transaction","update_transaction","delete_transaction","set_budget_amount","list_schedules","skip_schedule","get_upcoming"]);
    assert_eq!(tools[0]["annotations"]["readOnlyHint"],true); assert_eq!(tools[7]["annotations"]["destructiveHint"],true);
    send(&mut input,json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_accounts","arguments":{}}}));
    let result=recv(&mut output); assert_eq!(result["result"]["content"][0]["text"],"[]"); assert_ne!(result["result"]["isError"],true);
    send(&mut input,json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_upcoming","arguments":{"days":366}}}));
    let error=recv(&mut output); assert_eq!(error["result"]["content"][0]["text"],"Error: days: expected an integer from 0 to 365"); assert_eq!(error["result"]["isError"],true);
    send(&mut input,json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"get_transactions","arguments":{"startDate":"2026-09-01","endDate":"2026-09-30"}}}));
    let sorted=recv(&mut output); assert_eq!(sorted["result"]["content"][0]["text"],"{\n  \"total\": 0,\n  \"transactions\": [],\n  \"truncated\": false\n}");
    send(&mut input,json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"nope","arguments":{}}}));
    let unknown=recv(&mut output); assert_eq!(unknown["result"]["content"][0]["text"],"Error: Unknown tool: nope"); assert_eq!(unknown["result"]["isError"],true);
    drop(input); assert!(child.wait().unwrap().success());
}
