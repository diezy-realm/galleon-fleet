//! Integration test for `SystemGateway` (D1): boots the REAL `tonic`
//! server used in production (`galleon_gateway::start_grpc_server`'s own
//! `SystemGatewayService`, wired the same way) on a loopback TCP port, then
//! drives it with a real gRPC client over the wire — the same path the Go
//! engine's `pkg/client.SystemGatewayClient` exercises in production.
//!
//! This crate is server-only (`build_client(false)` in `build.rs`: the Go
//! engine owns the client), so there's no generated Rust client to import.
//! Rather than flip that crate-wide decision just for a test, this test
//! drives the server with `tonic::client::Grpc` directly against the
//! already-generated request/response prost types and the service's own
//! documented gRPC path (`clawcrew.agent.SystemGateway/<Method>`) — proving
//! the real network+codec round trip without adding a client dependency.

use galleon_gateway::grpc_system_gateway::{
    system_gateway_server::SystemGatewayServer, GetNativeToolCatalogRequest,
    GetNativeToolCatalogResponse, SystemGatewayService, SystemMetricsRequest,
    SystemMetricsResponse, ToolCallRequest, ToolCallResponse,
};
use std::net::SocketAddr;
use std::time::Duration;
use tonic::codec::ProstCodec;
use tonic::transport::{Channel, Endpoint, Server};

/// Boots the real server on an OS-assigned loopback port and returns a
/// connected channel plus the server's shutdown handle.
async fn spawn_server() -> (Channel, tokio::task::JoinHandle<()>, SocketAddr) {
    spawn_server_with_root(None).await
}

async fn spawn_server_with_root(
    root: Option<std::path::PathBuf>,
) -> (Channel, tokio::task::JoinHandle<()>, SocketAddr) {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = probe.local_addr().expect("local addr");
    drop(probe);

    let handle = tokio::spawn(async move {
        Server::builder()
            .add_service(SystemGatewayServer::new(SystemGatewayService::new(root)))
            .serve(addr)
            .await
            .expect("gRPC server exited unexpectedly");
    });

    // Give the listener a moment to actually start accepting.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let channel = Endpoint::from_shared(format!("http://{addr}"))
        .expect("valid endpoint")
        .connect()
        .await
        .expect("connect to the server we just booted");

    (channel, handle, addr)
}

/// Minimal unary gRPC call against `SystemGateway/ExecuteNativeTool`,
/// without pulling in a generated client — see module docs.
async fn execute_native_tool(
    channel: Channel,
    tool_name: &str,
    arguments_json: &str,
) -> ToolCallResponse {
    let mut grpc = tonic::client::Grpc::new(channel);
    let path =
        http::uri::PathAndQuery::from_static("/clawcrew.agent.SystemGateway/ExecuteNativeTool");
    grpc.ready().await.expect("channel ready");
    let request = tonic::Request::new(ToolCallRequest {
        tool_name: tool_name.to_string(),
        arguments_json: arguments_json.to_string(),
        workspace_path: None,
        timeout_seconds: None,
    });
    grpc.unary(request, path, ProstCodec::default())
        .await
        .expect("unary call succeeds")
        .into_inner()
}

async fn execute_native_tool_with_workspace(
    channel: Channel,
    tool_name: &str,
    arguments_json: &str,
    workspace_path: Option<&str>,
) -> ToolCallResponse {
    let mut grpc = tonic::client::Grpc::new(channel);
    let path =
        http::uri::PathAndQuery::from_static("/clawcrew.agent.SystemGateway/ExecuteNativeTool");
    grpc.ready().await.expect("channel ready");
    let request = tonic::Request::new(ToolCallRequest {
        tool_name: tool_name.to_string(),
        arguments_json: arguments_json.to_string(),
        workspace_path: workspace_path.map(|s| s.to_string()),
        timeout_seconds: None,
    });
    grpc.unary(request, path, ProstCodec::default())
        .await
        .expect("unary call succeeds")
        .into_inner()
}

async fn get_native_tool_catalog(channel: Channel) -> GetNativeToolCatalogResponse {
    let mut grpc = tonic::client::Grpc::new(channel);
    let path =
        http::uri::PathAndQuery::from_static("/clawcrew.agent.SystemGateway/GetNativeToolCatalog");
    grpc.ready().await.expect("channel ready");
    let request = tonic::Request::new(GetNativeToolCatalogRequest {});
    grpc.unary(request, path, ProstCodec::default())
        .await
        .expect("unary call succeeds")
        .into_inner()
}

#[tokio::test]
async fn go_callable_grpc_surface_executes_real_bash_command() {
    let (channel, server, _addr) = spawn_server().await;

    let resp = execute_native_tool(channel, "bash", r#"{"command":"echo roundtrip-ok"}"#).await;

    assert!(resp.success, "server rejected the call: {}", resp.error);
    assert!(
        resp.output.contains("roundtrip-ok"),
        "expected real bash stdout over the wire, got: {}",
        resp.output
    );

    server.abort();
}

#[tokio::test]
async fn go_callable_grpc_surface_rejects_tool_outside_allowlist() {
    let (channel, server, _addr) = spawn_server().await;

    let resp = execute_native_tool(channel, "rm_rf", "{}").await;

    assert!(!resp.success);
    assert!(resp.error.contains("not in the allowlist"));

    server.abort();
}

#[tokio::test]
async fn go_callable_grpc_surface_executes_write_file_with_content_key() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let workspace = temp_dir.path().to_str().expect("valid utf8");
    let (channel, server, _addr) =
        spawn_server_with_root(Some(temp_dir.path().to_path_buf())).await;

    // Call write_file with SSOT canonical key "content" (not "contents")
    let args = r#"{"path":"test_ssot.txt","content":"hello from SSOT contract"}"#;
    let resp =
        execute_native_tool_with_workspace(channel, "write_file", args, Some(workspace)).await;

    assert!(resp.success, "write_file failed: {}", resp.error);
    assert!(
        resp.output.contains("wrote 24 bytes"),
        "unexpected output: {}",
        resp.output
    );

    let written_file = temp_dir.path().join("test_ssot.txt");
    let content = std::fs::read_to_string(written_file).expect("file exists");
    assert_eq!(content, "hello from SSOT contract");

    server.abort();
}

#[tokio::test]
async fn go_callable_grpc_surface_accepts_alias_execute_command() {
    let (channel, server, _addr) = spawn_server().await;

    let resp =
        execute_native_tool(channel, "execute_command", r#"{"command":"echo alias-ok"}"#).await;

    assert!(
        resp.success,
        "server rejected execute_command alias: {}",
        resp.error
    );
    assert!(resp.output.contains("alias-ok"));

    server.abort();
}

#[tokio::test]
async fn go_callable_grpc_surface_returns_native_tool_catalog() {
    let (channel, server, _addr) = spawn_server().await;

    let resp = get_native_tool_catalog(channel).await;

    assert!(!resp.tools.is_empty(), "expected native tool catalog");
    let write_tool = resp
        .tools
        .iter()
        .find(|t| t.name == "write_file")
        .expect("write_file in catalog");
    assert_eq!(write_tool.risk_tier, "write");
    assert!(write_tool.input_schema_json.contains("content"));

    server.abort();
}

#[tokio::test]
async fn go_callable_grpc_surface_propagates_request_id_tracing_metadata() {
    let (channel, server, _addr) = spawn_server().await;

    let mut grpc = tonic::client::Grpc::new(channel);
    let path =
        http::uri::PathAndQuery::from_static("/clawcrew.agent.SystemGateway/ExecuteNativeTool");
    grpc.ready().await.expect("channel ready");

    let test_trace_id = "test-trace-id-abc-98765";
    let mut request = tonic::Request::new(ToolCallRequest {
        tool_name: "bash".to_string(),
        arguments_json: r#"{"command":"echo trace-ok"}"#.to_string(),
        workspace_path: None,
        timeout_seconds: None,
    });
    request
        .metadata_mut()
        .insert("x-request-id", test_trace_id.parse().unwrap());

    let response: tonic::Response<ToolCallResponse> = grpc
        .unary(request, path, ProstCodec::default())
        .await
        .expect("unary call succeeds");

    let returned_trace_id = response
        .metadata()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok());

    assert_eq!(
        returned_trace_id,
        Some(test_trace_id),
        "Rust gateway must echo/propagate x-request-id in response metadata"
    );
    assert!(response.into_inner().output.contains("trace-ok"));

    server.abort();
}

#[tokio::test]
async fn go_callable_grpc_surface_returns_real_system_metrics() {
    let (channel, server, _addr) = spawn_server().await;

    let mut grpc = tonic::client::Grpc::new(channel);
    let path =
        http::uri::PathAndQuery::from_static("/clawcrew.agent.SystemGateway/GetSystemMetrics");
    grpc.ready().await.expect("channel ready");

    let request = tonic::Request::new(SystemMetricsRequest {});
    let response: tonic::Response<SystemMetricsResponse> = grpc
        .unary(request, path, ProstCodec::default())
        .await
        .expect("unary call succeeds");

    let metrics = response.into_inner();
    assert!(metrics.system_ram_total_bytes > 0, "expected system_ram_total_bytes > 0");
    assert!(metrics.memory_total_mb > 0, "expected memory_total_mb > 0");
    assert!(metrics.num_cpus > 0, "expected num_cpus > 0");

    server.abort();
}


