use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    time::{Duration, timeout},
};

use super::{MPV_IPC_TIMEOUT_MS, MpvPlaybackSnapshot, UiMessage};

pub(super) async fn read_mpv_snapshot(socket_path: &str) -> Option<MpvPlaybackSnapshot> {
    let position = mpv_get_property_number(socket_path, "time-pos").await?;
    let paused = mpv_get_property_bool(socket_path, "pause")
        .await
        .unwrap_or(false);

    Some(MpvPlaybackSnapshot {
        position_secs: position.max(0.0),
        is_paused: paused,
    })
}

pub(super) fn run_player_exit_watcher(
    messages: std::sync::Arc<std::sync::Mutex<Vec<UiMessage>>>,
    mut child: std::process::Child,
    generation: u64,
) {
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let _ = child.wait();
        let elapsed_seconds = started.elapsed().as_secs();

        if let Ok(mut queue) = messages.lock() {
            queue.push(UiMessage::PlayerExited {
                generation,
                elapsed_seconds,
            });
        }
    });
}

async fn mpv_get_property_number(socket_path: &str, property: &str) -> Option<f64> {
    let response = mpv_get_property_value(socket_path, property).await?;
    response.as_f64()
}

async fn mpv_get_property_bool(socket_path: &str, property: &str) -> Option<bool> {
    let response = mpv_get_property_value(socket_path, property).await?;
    response.as_bool()
}

async fn mpv_get_property_value(socket_path: &str, property: &str) -> Option<Value> {
    let mut stream = timeout(
        Duration::from_millis(MPV_IPC_TIMEOUT_MS),
        UnixStream::connect(socket_path),
    )
    .await
    .ok()?
    .ok()?;

    let request = format!(r#"{{"command":["get_property","{property}"]}}\n"#);

    if timeout(
        Duration::from_millis(MPV_IPC_TIMEOUT_MS),
        stream.write_all(request.as_bytes()),
    )
    .await
    .is_err()
    {
        return None;
    }

    let mut buf = vec![0_u8; 1024];
    let read = timeout(
        Duration::from_millis(MPV_IPC_TIMEOUT_MS),
        stream.read(&mut buf),
    )
    .await
    .ok()?
    .ok()?;

    if read == 0 {
        return None;
    }

    let text = String::from_utf8_lossy(&buf[..read]);
    for line in text.lines() {
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            if value.get("error").and_then(Value::as_str) != Some("success") {
                continue;
            }
            if let Some(data) = value.get("data") {
                return Some(data.clone());
            }
        }
    }

    None
}
