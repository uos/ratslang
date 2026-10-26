use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

struct Server {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Value>,
    next_id: u64,
}
impl Server {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ratslang-lsp"))
            .env("RUST_LOG", "info")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, messages) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut length = None;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap() == 0 {
                        return;
                    }
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.strip_prefix("Content-Length: ") {
                        length = value.trim().parse::<usize>().ok();
                    }
                }
                let mut body = vec![0; length.expect("stdout contains only LSP framing")];
                if reader.read_exact(&mut body).is_err() {
                    return;
                }
                if send.send(serde_json::from_slice(&body).unwrap()).is_err() {
                    return;
                }
            }
        });
        Self {
            child,
            stdin,
            messages,
            next_id: 0,
        }
    }
    fn send(&mut self, message: Value) {
        let body = serde_json::to_vec(&message).unwrap();
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        self.stdin.write_all(&body).unwrap();
        self.stdin.flush().unwrap();
    }
    fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({"jsonrpc":"2.0", "method":method, "params":params}));
    }
    fn receive(&self) -> Value {
        self.messages
            .recv_timeout(Duration::from_secs(15))
            .expect("server response within 15s")
    }
    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        self.send(json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}));
        loop {
            let message = self.receive();
            if message["id"] == id {
                assert!(message.get("error").is_none(), "{message}");
                return message["result"].clone();
            }
        }
    }
    fn diagnostics(&self, version: i64) -> Value {
        loop {
            let message = self.receive();
            if message["method"] == "textDocument/publishDiagnostics"
                && message["params"]["version"] == version
            {
                return message["params"]["diagnostics"].clone();
            }
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn editor_session_supports_constraints_completion_navigation_and_incremental_edits() {
    let folder = std::env::temp_dir().join(format!("ratslang-lsp-session-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let defaults = "sensor {\n# How many\n# @recommended\n# @type integer\n# @min 1\ncount = 3\n# @enum CPU, Backend\nbackend = CPU\n}\n";
    std::fs::write(folder.join("default.rl"), defaults).unwrap();
    let uri = tower_lsp_server::ls_types::Uri::from_file_path(folder.join("user.rl")).unwrap();
    let uri = uri.as_str();
    let mut server = Server::start();
    let init = server.request("initialize", json!({"processId":null, "capabilities":{}}));
    assert_eq!(init["capabilities"]["positionEncoding"], "utf-16");
    assert_eq!(init["capabilities"]["textDocumentSync"]["change"], 2);
    assert_eq!(init["capabilities"]["definitionProvider"], true);
    server.notify("initialized", json!({}));
    let source = "# @schema ./default.rl\nsensor {\n count = 3\n backend = CPU\n}\n";
    server.notify(
        "textDocument/didOpen",
        json!({"textDocument":{"uri":uri,"languageId":"rl","version":1,"text":source}}),
    );
    assert_eq!(server.diagnostics(1), json!([]));
    let symbols = server.request(
        "textDocument/documentSymbol",
        json!({"textDocument":{"uri":uri}}),
    );
    assert!(
        symbols
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["name"] == "sensor" && s["children"].as_array().unwrap().len() == 2)
    );
    let hover = server.request(
        "textDocument/hover",
        json!({"textDocument":{"uri":uri},"position":{"line":2,"character":4}}),
    );
    assert!(
        hover["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("How many")
    );
    assert!(
        hover["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("**Override source:**")
    );
    assert!(
        hover["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("default.rl:")
    );
    let completion = server.request(
        "textDocument/completion",
        json!({"textDocument":{"uri":uri},"position":{"line":3,"character":12}}),
    );
    assert!(
        completion
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["label"] == "Backend")
    );
    server.notify("textDocument/didChange", json!({"textDocument":{"uri":uri,"version":2},"contentChanges":[{"range":{"start":{"line":2,"character":9},"end":{"line":2,"character":10}},"text":"0"},{"range":{"start":{"line":3,"character":11},"end":{"line":3,"character":14}},"text":"Backend"}]}));
    let errors = server.diagnostics(2);
    assert_eq!(errors.as_array().unwrap().len(), 1, "{errors}");
    assert!(
        errors[0]["message"]
            .as_str()
            .unwrap()
            .contains("sensor.count")
    );
    let symbols = server.request(
        "textDocument/documentSymbol",
        json!({"textDocument":{"uri":uri}}),
    );
    assert!(
        symbols
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["name"] == "sensor")
    );
    let default_uri =
        tower_lsp_server::ls_types::Uri::from_file_path(folder.join("default.rl")).unwrap();
    // An unsaved defaults edit updates the dependent user document immediately.
    server.notify("textDocument/didOpen", json!({"textDocument":{"uri":default_uri.as_str(),"languageId":"rl","version":10,"text":defaults.replace("@min 1", "@min 0")}}));
    assert_eq!(server.diagnostics(2), json!([]));
    server.notify("textDocument/didChange", json!({"textDocument":{"uri":uri,"version":3},"contentChanges":[{"text":"# @schema ./default.rl\nsensor {\n backend = CPU\n}\n"}]}));
    assert_eq!(server.diagnostics(3), json!([]));
    let actions = server.request("textDocument/codeAction", json!({"textDocument":{"uri":uri},"range":{"start":{"line":2,"character":1},"end":{"line":2,"character":1}},"context":{"diagnostics":[]}}));
    assert!(
        actions
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["title"] == "Insert all recommended properties"
                && a["edit"]["changes"][uri][0]["newText"] == "sensor.count = 3\n")
    );
    let definition = server.request(
        "textDocument/definition",
        json!({"textDocument":{"uri":uri},"position":{"line":0,"character":2}}),
    );
    assert!(definition["uri"].as_str().unwrap().ends_with("default.rl"));
    server.notify("textDocument/didChange", json!({"textDocument":{"uri":uri,"version":4},"contentChanges":[{"text":"# @schema ./default.rl\nsensor {\n coutn = 4\n}\n"}]}));
    let errors = server.diagnostics(4);
    assert_eq!(errors.as_array().unwrap().len(), 1, "{errors}");
    assert!(
        errors[0]["message"]
            .as_str()
            .unwrap()
            .contains("Did you mean sensor.count?")
    );
    let actions = server.request("textDocument/codeAction", json!({"textDocument":{"uri":uri},"range":{"start":{"line":2,"character":2},"end":{"line":2,"character":2}},"context":{"diagnostics":errors}}));
    let correction = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["title"] == "Rename sensor.coutn to sensor.count")
        .unwrap();
    assert_eq!(correction["edit"]["changes"][uri][0]["newText"], "count");
    server.request("shutdown", Value::Null);
    server.notify("exit", Value::Null);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn remote_schema_comments_supply_documentation_and_report_load_errors_on_the_directive() {
    use std::net::TcpListener;
    for success in [true, false] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let http = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut request = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                if request.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            let body = if success {
                "# Distance from sensor\n# @min 1cm\ndistance = 2m\n# Processing engine\n# @enum CPU, Backend\nbackend = CPU\n"
            } else {
                "missing"
            };
            let status = if success { "200 OK" } else { "404 Not Found" };
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        let mut server = Server::start();
        server.request("initialize", json!({"processId":null,"capabilities":{}}));
        server.notify("initialized", json!({}));
        let uri = "file:///tmp/remote-schema-overrides.rl";
        let source =
            format!("# @schema http://{address}/default.rl\ndistance = 50cm\nbackend = CPU\n\n");
        server.notify(
            "textDocument/didOpen",
            json!({"textDocument":{"uri":uri,"languageId":"rl","version":1,"text":source}}),
        );
        let diagnostics = server.diagnostics(1);
        if success {
            assert_eq!(diagnostics, json!([]));
            let completion = server.request(
                "textDocument/completion",
                json!({"textDocument":{"uri":uri},"position":{"line":3,"character":0}}),
            );
            let distance = completion
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["label"] == "distance")
                .unwrap();
            assert!(
                distance["documentation"]["value"]
                    .as_str()
                    .unwrap()
                    .contains("Distance from sensor")
            );
            let hover = server.request(
                "textDocument/hover",
                json!({"textDocument":{"uri":uri},"position":{"line":1,"character":2}}),
            );
            let docs = hover["contents"]["value"].as_str().unwrap();
            assert!(
                docs.contains("Distance from sensor")
                    && docs.contains("Default")
                    && docs.contains("Minimum"),
                "{docs}"
            );
            let completion = server.request(
                "textDocument/completion",
                json!({"textDocument":{"uri":uri},"position":{"line":2,"character":12}}),
            );
            assert!(
                completion
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|c| c["label"] == "Backend")
            );
        } else {
            assert_eq!(diagnostics[0]["range"]["start"]["line"], 0);
            assert_eq!(
                diagnostics[0]["range"]["end"]["character"],
                source.lines().next().unwrap().len()
            );
            assert!(diagnostics[0]["message"].as_str().unwrap().contains("404"));
        }
        let definition = server.request(
            "textDocument/definition",
            json!({"textDocument":{"uri":uri},"position":{"line":0,"character":5}}),
        );
        assert_eq!(definition["uri"], format!("http://{address}/default.rl"));
        server.request("shutdown", Value::Null);
        server.notify("exit", Value::Null);
        http.join().unwrap();
    }
}
